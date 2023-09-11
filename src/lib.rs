#![feature(impl_trait_in_assoc_type)]

use pilota::{lazy_static::lazy_static, FastStr};
use volo_gen::volo::example::{
	PingRequest, PingResponse,
	SetRequest, SetResponse,
	GetRequest, GetResponse,	
	DelRequest, DelResponse,
	PublishRequest, PublishResponse,
	SubscribeRequest, SubscribeResponse,
};
use tokio::{
	task::JoinSet,
	sync::{
		Mutex,
		broadcast::{Sender, Receiver},
	},
};
use std::collections::HashMap;
use std::sync::Arc;

pub struct S;

lazy_static! {
	static ref DB: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
	static ref CHANNEL: Mutex<HashMap<String, Arc<Sender<String>>>> = Mutex::new(HashMap::new());
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

	async fn publish(&self, _req: PublishRequest) ->
		::core::result::Result<PublishResponse, ::volo_thrift::AnyhowError> {
		let t = CHANNEL.lock().await;
		let res = (*t).get(&_req.channel.into_string());
		println!("publish get lock");

		let num = match res {
			Some(sender) => {
				println!("send");
				let _  = sender.send(_req.msg.into_string());
				1
			},
			None => {
				println!("channel not exist");
				0
			}
		};
		Ok(PublishResponse { num })
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
