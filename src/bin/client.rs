// RPC Client & HTTP Server
use lazy_static::lazy_static;
use pilota::FastStr;
use reqwest::StatusCode;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use std::net::SocketAddr;

use axum::{
    Router,
    response::{IntoResponse, Response, Html},
    extract::Path,
    routing,
    Form,
};

use volo_gen::volo::example::{
    PingRequest,
    SetRequest,
    GetRequest,
    DelRequest,
    SubscribeRequest,
    PublishRequest,
};

use tracing_subscriber::{fmt, util::SubscriberInitExt};

use serde::Deserialize;

pub const RPC_ADDR: &str = "127.0.0.1:8080";

lazy_static! {
    static ref CLIENT: volo_gen::volo::example::ItemServiceClient = {
        let addr: SocketAddr = RPC_ADDR.parse().unwrap();
        volo_gen::volo::example::ItemServiceClientBuilder::new("volo-example")
            .address(addr)
            .build()
    };
}

#[volo::main]
async fn main() {
//     tracing_subscriber::fmt::init();
    tracing_subscriber::registry().with(fmt::layer()).init();

    let app = Router::new()
        .route("/ping", routing::get(ping))
        .route("/ping/:payload", routing::get(ping_sth))
        .route("/get/:key", routing::get(get))
        .route(
            "/set",
            routing::get(show_set_form).post(set)
        )
        .route("/del", routing::get(show_del_form).post(del))
        .route("/subscribe", routing::get(show_subscribe_form).post(subscribe))
        .route("/publish", routing::get(show_publish_form).post(publish));

    let addr = SocketAddr::from(([127, 0, 0, 1], 13784));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn ping() -> (StatusCode, &'static str) {
    (StatusCode::OK, "pong")
}

async fn ping_sth(Path(key): Path<String>) -> Response{
    let req = PingRequest {
        payload: Some(FastStr::new(key)),
    };
    let res = CLIENT.ping(req).await.unwrap();
    (StatusCode::OK, res.payload.into_string()).into_response()
}

async fn get(Path(key): Path<String>) -> Response {
    tracing::debug!("GET {}", key);
    let req = GetRequest {
        key: FastStr::new(key),
    };
    let res = CLIENT.get(req).await.unwrap();
    match res.value {
        Some(value) => {
            (StatusCode::OK, value.into_string()).into_response()
        }
        None => {
            (StatusCode::OK, "{{nil}}").into_response()
        }
    }
}

async fn show_set_form() -> Html<&'static str> {
    Html(
        r#"
        <!doctype html>
        <html>
            <head></head>
            <body>
                <form action="/set" method="post">
                    <label for="key">
                        Enter key:
                        <input type="text" name="key">
                    </label>
                    <label for="value">
                        Enter value:
                        <input type="text" name="value">
                    </label>
                    <input type="submit" value="Set!">
                </form>
            </body>
        </html>
        "#,
    )
}

#[derive(Deserialize, Debug)]
struct FormKeyValue {
    key: String,
    value: String,
}

#[derive(Deserialize, Debug)]
struct FormKey {
    key: String,
}

async fn set(Form(setkey): Form<FormKeyValue>) -> Response {
    tracing::debug!("SET {} {}", setkey.key, setkey.value);
    let req = SetRequest {
        key: FastStr::new(setkey.key),
        value: FastStr::new(setkey.value),
    };
    let _ = CLIENT.set(req).await.unwrap();
    (StatusCode::OK, "set success").into_response()
}

async fn show_del_form() -> Html<&'static str> {
    Html(
        r#"
        <!doctype html>
        <html>
            <head></head>
            <body>
                <form action="/del" method="post">
                    <label for="key">
                        Enter key:
                        <input type="text" name="key">
                    </label>
                    <input type="submit" value="Delete!">
                </form>
            </body>
        </html>
        "#,
    )
}

async fn del(Form(delkey): Form<FormKey>) -> (StatusCode, &'static str) {
    let req = DelRequest {
        keys: vec![FastStr::new(delkey.key)],
    };
    let _ = CLIENT.del(req).await.unwrap();
    (StatusCode::OK, "del success")
}

async fn show_subscribe_form() -> Html<&'static str> {
    Html(
        r#"
        <!doctype html>
        <html>
            <head></head>
            <body>
                <form action="/subscribe" method="post">
                    <label for="key">
                        Enter channel:
                        <input type="text" name="key">
                    </label>
                    <input type="submit" value="Subscribe!">
                </form>
            </body>
        </html>
        "#,
    )
}

async fn subscribe(Form(channel): Form<FormKey>) -> Response {
    let req = SubscribeRequest {
        channels: vec![FastStr::new(channel.key)],
    };

    let res = CLIENT.subscribe(req).await.unwrap();
    (StatusCode::OK, res.msg.into_string()).into_response()
}

async fn show_publish_form() -> Html<&'static str> {
    Html(
        r#"
        <!doctype html>
        <html>
            <head></head>
            <body>
                <form action="/publish" method="post">
                    <label for="key">
                        Enter channel:
                        <input type="text" name="key">
                    </label>
                    <label for="value">
                        Enter message:
                        <input type="text" name="value">
                    </label>
                    <input type="submit" value="Publish!">
                </form>
            </body>
        </html>
        "#,
    )
}

async fn publish(Form(publ): Form<FormKeyValue>) -> (StatusCode, &'static str) {
    let req = PublishRequest {
        channel: FastStr::new(publ.key),
        msg: FastStr::new(publ.value),
    };
    let _ = CLIENT.publish(req).await.unwrap();

    (StatusCode::OK, "puslish success")
}