namespace rs volo.example

// PING PONG
struct PingRequest {
    1: optional string payload,
}

struct PingResponse {
    1: required string payload,
}

// SET
struct SetRequest {
    1: required string key,
    2: required string value,
}

struct SetResponse {
    1: required string status,
}

// GET
struct GetRequest {
    1: required string key,
}

struct GetResponse {
    1: optional string value,
}

// DEL
struct DelRequest {
    1: required list<string> keys,
}

struct DelResponse {
    1: required i64 num,
}

// PUBLISH
struct PublishRequest {
    1: required string channel,
    2: required string msg,
}

struct PublishResponse {
    1: required i64 num,
}

// SUBSCRIBE
struct SubscribeRequest {
    1: required list<string> channels,
}

struct SubscribeResponse {
    1: required string msg,
}

service ItemService {
    PingResponse ping(1: PingRequest req),
    SetResponse set(1: SetRequest req),
    GetResponse get(1: GetRequest req),
    DelResponse del(1: DelRequest req),
    PublishResponse publish(1: PublishRequest req),
    SubscribeResponse subscribe(1: SubscribeRequest req),
}

