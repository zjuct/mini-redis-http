# mini-redis

- 支持`PING`, `GET`, `SET`, `DEL`, `PUBLISH`, `SUBSCRIBE`命令

- 提供client-cli命令行前端

- Filter中间件实现自定义请求过滤

## `PING`

```
PING [string]
```

## `GET`, `SET`, `DEL`

```
GET key
SET key value
DEL [key]...
```

## `PUBLISH`, `SUBSCRIBE`

```
PUBLISH channel message
SUBSCRIBE [channel]...
```

`PUBLISH`支持广播

1. `cargo run --bin server`启动服务器
2. 在两个终端通过`cargo run --bin client`启动两个客户端A, B
3. A运行`SUBSCRIBE hello1 hello2`，订阅两个管道
4. B运行`PUBLISH hello1 world`，A收到`world`，退出等待

`SUBSCRIBE`, `PUBLISH`为ping-pong模型，一次连接发送单个message

## `Filter`

支持自定义predicate，Filter根据predicate的结果判断是否丢弃请求
