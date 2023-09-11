use lazy_static::lazy_static;
use pilota::FastStr;
use std::{net::SocketAddr, thread};
use tokio::sync::broadcast;

use volo_gen::volo::example::{
    PingRequest,
    SetRequest,
    GetRequest,
    DelRequest,
    PublishRequest,
    SubscribeRequest,
};

use mini_redis::FilterLayer;

lazy_static! {
    static ref CLIENT: volo_gen::volo::example::ItemServiceClient = {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        volo_gen::volo::example::ItemServiceClientBuilder::new("volo-example")
            .layer_outer(FilterLayer)
            .address(addr)
            .build()
    };
}

#[derive(Clone, Debug)]
enum Input {
    Ping(Option<String>),
    Set(String, String),
    Get(String),
    Del(Vec<String>),
    Publish(String, String),
    Subscribe(Vec<String>),
}

#[volo::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // spawn a thread to handle user input
    let (send, mut recv): (broadcast::Sender<Input>, broadcast::Receiver<Input>) = broadcast::channel(16); 
    thread::spawn(move || {
        get_input(send);
    });

    loop {
        match recv.recv().await {
            Ok(input) => handle_input(input).await,
            Err(_) => break,
        }
    }
}

fn get_input(send: broadcast::Sender<Input>) {
    let mut quit = false;
    while !quit {
        let mut buf = String::new();
        std::io::stdin()
            .read_line(&mut buf)
            .unwrap();

        
        let input_vec: Vec<&str> = buf.split_whitespace().collect();
//        for i in &input_vec {
//            println!("{i}");
//        }
        if input_vec.len() == 0 {
            continue;
        }
        match input_vec[0].to_uppercase().as_str() {
            "PING" => {
                match input_vec.len() {
                    1 => {
                        send.send(Input::Ping(None)).unwrap();
                    },
                    2 => {
                        send.send(Input::Ping(Some(String::from(input_vec[1])))).unwrap();
                    },
                    _ => {
                        println!("Invalid format of PING");
                    }
                }
            },
            "SET" => {
                match input_vec.len() {
                    3 => {
                        send.send(Input::Set(String::from(input_vec[1]), String::from(input_vec[2]))).unwrap();
                    },
                    _ => {
                        println!("Invalid format of SET");
                    }
                }
            },
            "GET" => {
                match input_vec.len() {
                    2 => {
                        send.send(Input::Get(String::from(input_vec[1]))).unwrap();
                    },
                    _ => {
                        println!("Invalid format of GET");
                    }
                }
            },
            "DEL" => {
                match input_vec.len() {
                    1 => {
                        println!("Invalid format of DEL");
                    },
                    _ => {
                        send.send(Input::Del(
                            input_vec[1..].iter().map(|s| String::from(*s)).collect()
                        )).unwrap();
                    }
                }
            },
            "PUBLISH" => {
                match input_vec.len() {
                    3 => {
                        send.send(Input::Publish(String::from(input_vec[1]), String::from(input_vec[2]))).unwrap();
                    },
                    _ => {
                        println!("Invalid format of PUBLISH");
                    }
                }
            },
            "SUBSCRIBE" => {
                match input_vec.len() {
                    1 => {
                        println!("Invalid format of SUBSCRIBE");
                    },
                    _ => {
                        send.send(Input::Subscribe(
                            input_vec[1..].iter().map(|s| String::from(*s)).collect()
                        )).unwrap();
                    }
                }
            },
            "QUIT" => {
                quit = true;
            },
            _ => {
                println!("Invalid input");
            }
        }
    }
}

async fn handle_input(input: Input) {
    match input {
        Input::Ping(payload) => {
            let res = ping(payload).await.unwrap();
            println!("{}", res);
        },
        Input::Set(key, value) => {
            set(key, value).await.unwrap();
        },
        Input::Get(key) => {
            let res = get(key).await.unwrap();
            match res {
                Some(value) => {
                    println!("{value}");
                },
                None => {
                    println!("{{nil}}");
                }
            }
        },
        Input::Del(key) => {
            let res = del(key).await.unwrap();
            println!("{res}");
        },
        Input::Publish(channel, msg) => {
            let _ = publish(channel, msg).await.unwrap();
        },
        Input::Subscribe(channels) => {
            let res = subscribe(channels).await.unwrap();
            println!("{res}");
        }
    }
}

#[allow(dead_code)]
async fn ping(payload: Option<String>) -> Result<String, anyhow::Error> {
    let req = match payload {
        Some(payload) => PingRequest { payload: Some(FastStr::new(payload)) },
        None => PingRequest { payload: None },
    };
    let res = CLIENT.ping(req).await?;
    Ok(res.payload.into_string())
}

#[allow(dead_code)]
async fn set(key: String, value: String) -> Result<(), anyhow::Error> {
    let req = SetRequest {
        key: FastStr::new(key),
        value: FastStr::new(value),
    };
    let res = CLIENT.set(req).await?;
    println!("{}", res.status.into_string());
    Ok(())
}

#[allow(dead_code)]
async fn get(key: String) -> Result<Option<String>, anyhow::Error> {
    let req = GetRequest {
        key: FastStr::new(key),
    };
    let res = CLIENT.get(req).await?;
    match res.value {
        Some(value) => Ok(Some(value.into_string())),
        None => Ok(None),
    }
}

#[allow(dead_code)]
async fn del(keys: Vec<String>) -> Result<i64, anyhow::Error> {
    let req = DelRequest {
        keys: keys.into_iter().map(|k| FastStr::new(k)).collect(),
    };
    let res = CLIENT.del(req).await?;
    Ok(res.num)
}

#[allow(dead_code)]
async fn publish(channel: String, msg: String) -> Result<(), anyhow::Error> {
    let req = PublishRequest {
        channel: FastStr::new(channel),
        msg: FastStr::new(msg),
    };
    let _ = CLIENT.publish(req).await?;
    Ok(())
}

#[allow(dead_code)]
async fn subscribe(channels: Vec<String>) -> Result<String, anyhow::Error> {
    let req = SubscribeRequest {
        channels: channels.into_iter().map(|c| FastStr::new(c)).collect(),
    };
    let res = CLIENT.subscribe(req).await?;
    Ok(res.msg.into_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ping_test() {
        assert_eq!(ping(Some(String::from("abc"))).await.unwrap(), "abc");
        assert_eq!(ping(None).await.unwrap(), "PONG");
        assert_eq!(ping(Some(String::from("   hello\nworld   "))).await.unwrap(), "   hello\nworld   ");
    }

    #[tokio::test]
    async fn get_set_del_test() {
        set(String::from("abc"), String::from("def")).await.unwrap();
        set(String::from("hello"), String::from("world")).await.unwrap();
        
        assert_eq!(get(String::from("abc")).await.unwrap(), Some(String::from("def")));
        assert_eq!(get(String::from("hello")).await.unwrap(), Some(String::from("world")));
        assert_eq!(get(String::from("abd")).await.unwrap(), None);

        set(String::from("abc"), String::from("hij")).await.unwrap();
        set(String::from("aaa"), String::from("bbb")).await.unwrap();
        assert_eq!(get(String::from("abc")).await.unwrap(), Some(String::from("hij")));
        
        assert_eq!(del(vec![String::from("abc"), String::from("aaa")]).await.unwrap(), 2);
        assert_eq!(get(String::from("abc")).await.unwrap(), None);
        assert_eq!(get(String::from("aaa")).await.unwrap(), None);

        assert_eq!(del(vec![String::from("hello"), String::from("world")]).await.unwrap(), 1);
        assert_eq!(get(String::from("hello")).await.unwrap(), None);

        assert_eq!(del(vec![]).await.unwrap(), 0);
    }
}