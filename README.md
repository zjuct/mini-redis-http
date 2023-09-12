# mini-redis-http

- 支持`PING`, `GET`, `SET`, `DEL`, `PUBLISH`, `SUBSCRIBE`命令

- 实现http服务器，监听端口13784

- Filter中间件实现自定义请求过滤

## `PING`

```
localhost:13784/ping
localhost:13784/ping/payload
```

## `GET`, `SET`, `DEL`

```
localhost:13784/get/:key
localhost:13784/set
localhost:13784/del
```

## `PUBLISH`, `SUBSCRIBE`

```
localhost:13784/publish
localhost:13784/subscribe
```

`PUBLISH`支持广播

1. `cargo run --bin server`启动rpc服务器
2. 在另一终端`cargo run --bin client`启动rpc客户端和http服务器
3. 使用浏览器或者curl等其它工具，通过上述URL与http服务器通信

`SUBSCRIBE`, `PUBLISH`为ping-pong模型，一次连接发送单个message

## `Filter`

支持自定义predicate，Filter根据predicate的结果判断是否丢弃请求
