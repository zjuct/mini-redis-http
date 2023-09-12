#![feature(impl_trait_in_assoc_type)]

use pilota::{lazy_static::lazy_static, FastStr};
use volo_gen::volo::example::{
	PingRequest, PingResponse,
	SetRequest, SetResponse,
	GetRequest, GetResponse,	
	DelRequest, DelResponse,
	PublishRequest, PublishResponse,
	SubscribeRequest, SubscribeResponse,
	ItemServiceRequestSend,
};
use tokio::{
	task::JoinSet,
	sync::{
		Mutex,
		broadcast::{Sender, Receiver},
	},
};
use std::{collections::HashMap, future::Future};
use std::sync::Arc;


pub struct S;

lazy_static! {
	static ref DB: Mutex<HashMap<String, String>> = {
		println!("Init DB");
		Mutex::new(HashMap::new())
	};
	static ref CHANNEL: Mutex<HashMap<String, Arc<Sender<String>>>> = {
		println!("Init CHANNLE");
		Mutex::new(HashMap::new())
	};
}

const CHANNEL_SIZE: usize = 16;
const ANSI_RED: &str = "\x1b[0;31m";
const ANSI_END: &str = "\x1b[0m";

#[volo::async_trait]
impl volo_gen::volo::example::ItemService for S {
	async fn ping(&self, _req: PingRequest) ->
		::core::result::Result<PingResponse, ::volo_thrift::AnyhowError> {
		match _req.payload {
			Some(payload) => {
				Ok(PingResponse { payload })
			},
			None => {
				Ok(PingResponse { payload: "PONG".parse().unwrap() })
			}
		}
	}

	async fn set(&self, _req: SetRequest) ->
		::core::result::Result<SetResponse, ::volo_thrift::AnyhowError> {
		let mut t = DB.lock().await;
		let _ = (*t).insert(_req.key.into_string(), _req.value.into_string());
		Ok(SetResponse { status: "OK".parse().unwrap() })
	}

	async fn get(&self, _req: GetRequest) ->
		::core::result::Result<GetResponse, ::volo_thrift::AnyhowError> {
		let t = DB.lock().await;
		let res = (*t).get(&_req.key.into_string());
		
		match res {
			Some(value) => Ok(GetResponse { value: Some(value.clone().parse().unwrap()) }),
			None => Ok(GetResponse { value: None })
		}
	}

	async fn del(&self, _req: DelRequest) ->
		::core::result::Result<DelResponse, ::volo_thrift::AnyhowError> {
		let mut t = DB.lock().await;
		let mut num = 0;
		
		for key in _req.keys {
			if let Some(_) = (*t).remove(key.as_str()) {
				num += 1;
			}
		}
		Ok(DelResponse { num })
	}

	// 简化的publish，不返回接收方的数量和channel名
	async fn publish(&self, _req: PublishRequest) ->
		::core::result::Result<PublishResponse, ::volo_thrift::AnyhowError> {
		let t = CHANNEL.lock().await;
		let res = (*t).get(&_req.channel.into_string());
		println!("publish get lock");

		match res {
			Some(sender) => {
				let _  = sender.send(_req.msg.into_string());
			},
			None => {
				println!("channel not exist");
			}
		}
		Ok(PublishResponse { })
	}

	async fn subscribe(&self, _req: SubscribeRequest) ->
		::core::result::Result<SubscribeResponse, ::volo_thrift::AnyhowError> {
		let mut t = CHANNEL.lock().await;
		println!("subscribe get lock");

		let mut receivers : Vec<Receiver<String>> = Vec::new();
		for channel in _req.channels.into_iter() {
			let res = (*t).get(&channel.clone().into_string());
			match res {
				Some(sender) => {
					receivers.push(sender.subscribe());
				},
				None => {
					println!("create channel");
					let (send, recv) = tokio::sync::broadcast::channel(CHANNEL_SIZE);
					(*t).insert(channel.into_string(), Arc::new(send));
					receivers.push(recv);
				}
			}
		}

		// 为什么t的生命周期会越过for块，导致在持有mutex时await死锁
		// 先暂时用drop(t)顶一下
		drop(t);
		// return the first message received
		let mut js = JoinSet::new();

		for mut receiver in receivers.into_iter() {
			js.spawn(async move {
				receiver.recv().await
			});
		}

		let msg = js.join_next().await.unwrap().unwrap().unwrap();
		println!("get message");
		Ok(SubscribeResponse {
			msg: FastStr::new(msg),
		})
	}
}

#[derive(Clone)]
pub struct FilterService<S, P>
where
	P: Clone + Fn(ItemServiceRequestSend) -> bool,
	S: Clone,
{
	inner: S,
	pred: P,
}

impl<S, P> FilterService<S, P>
where
	P: Clone + Fn(ItemServiceRequestSend) -> bool,
	S: Clone,
{
	pub fn new(inner: S, pred: P) -> Self {
		Self {
			inner, pred,
		}
	}
}

impl<Cx, S, P> volo::Service<Cx, ItemServiceRequestSend> for FilterService<S, P>
where
	Cx: 'static + Send,
	S: 'static + volo::Service<Cx, ItemServiceRequestSend> + Send + Sync + Clone,
	S::Error: Send + Sync,
	P: 'static + Send + Sync + Clone + Fn(ItemServiceRequestSend) -> bool,
{
	type Response = S::Response;

	type Error = volo_thrift::error::Error;

	type Future<'cx> = impl Future<Output = Result<Self::Response, Self::Error>> + Send + 'cx;

	fn call<'cx, 's>(&'s self, cx: &'cx mut Cx, req: ItemServiceRequestSend) -> Self::Future<'cx>
	where
		's: 'cx,
	{
		let t = (*self).clone();
		async move {
			match (t.pred)(req.clone()) {
				true => {
					match t.inner.call(cx, req).await {
						Ok(val) => Ok(val),
						Err(_) => Err(volo_thrift::error::Error::Application(volo_thrift::ApplicationError {
							kind: volo_thrift::ApplicationErrorKind::UNKNOWN,
							message: format!("{}Request filtered{}", ANSI_RED, ANSI_END),
						}))
					}
				},
				false => {
					Err(volo_thrift::error::Error::Application(volo_thrift::ApplicationError {
						kind: volo_thrift::ApplicationErrorKind::UNKNOWN,
						message: format!("{}Request filtered{}", ANSI_RED, ANSI_END),
					}))
				},
			}
		}
	}
}

pub struct FilterLayer<P>
where
	P: Clone + Fn(ItemServiceRequestSend) -> bool,
{
	pred: P,
}

impl<P> FilterLayer<P>  
where
	P: Clone + Fn(ItemServiceRequestSend) -> bool,
{
	pub fn new(pred: P) -> Self {
		Self { pred }
	}
}

impl<S, P> volo::Layer<S> for FilterLayer<P>
where
	P: Clone + Fn(ItemServiceRequestSend) -> bool,
	S: Clone,
{
	type Service = FilterService<S, P>;

	fn layer(self, inner: S) -> Self::Service {
		let predicate = self.pred.clone();
		FilterService::new(inner, predicate)
	}
}