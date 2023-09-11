#![feature(impl_trait_in_assoc_type)]

use anyhow::anyhow;
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

use motore::BoxError;

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
		println!("a: {}", (*t).get(&String::from("a")).unwrap());
		
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
		// test: only consider the first channel
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
pub struct FilterService<S>(S);

impl<Cx, Req, S> volo::Service<Cx, Req> for FilterService<S> 
where
	Req: 'static + Send,
	S: volo::Service<Cx, Req> + 'static + Send + Sync,
 	Req : std::fmt::Debug,
	S::Error: Send + Sync + Into<volo_thrift::Error>,
	Cx: 'static + Send,
{
	type Response = S::Response;

	type Error = volo_thrift::Error;

	type Future<'cx> = impl Future<Output = Result<S::Response, Self::Error>> + 'cx;

	fn call<'cx, 's>(&'s self, cx: &'cx mut Cx, req: Req) -> Self::Future<'cx>
	where
		's: 'cx,
	{
		async move {
			if format!("{:?}", req).starts_with("Ping") {
				return Err(anyhow!("can not use ping").into());
			}
			self.0.call(cx, req).await.map_err(Into::into)
		}
	}
}

pub struct FilterLayer;

impl<S> volo::Layer<S> for FilterLayer {
	type Service = FilterService<S>;

	fn layer(self, inner: S) -> Self::Service {
		FilterService(inner)
	}
}