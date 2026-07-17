# 视频会议信令协议

## 1. 概述

信令协议使用 WebSocket 作为实时传输层，gRPC 作为服务间通信。

### 设计原则

- **独立于媒体层**: 信令不依赖 SFU 具体实现，可切换 SFU 引擎
- **JSON 负载**: Web 端优先 JSON，native 端可选 Protobuf
- **请求-响应 + 事件推送**: 统一消息格式

## 2. 传输层

### 2.1 WebSocket 端点

```
ws://<host>/ws/conference/v1
wss://<host>/ws/conference/v1  (TLS)
```

连接时携带 Token 认证：

```
GET /ws/conference/v1 HTTP/1.1
Upgrade: websocket
Sec-WebSocket-Protocol: conference-v1
Authorization: Bearer <token>
```

### 2.2 消息格式

```typescript
// 请求
interface WsRequest {
  id: string;                    // 请求 ID，用于关联响应
  method: string;                // 方法名，如 "room.create"
  params: Record<string, any>;   // 参数
}

// 响应
interface WsResponse {
  id: string;                    // 对应请求 ID
  result?: Record<string, any>;  // 成功响应
  error?: {                      // 错误响应
    code: string;
    message: string;
  };
}

// 事件推送
interface WsEvent {
  type: string;                  // 事件类型
  data: Record<string, any>;     // 事件数据
  timestamp: string;             // ISO 8601
}
```

## 3. 信令方法

### 3.1 房间管理

| 方法 | 方向 | 说明 |
|------|------|------|
| `room.create` | Client → Server | 创建房间 |
| `room.join` | Client → Server | 加入房间 |
| `room.leave` | Client → Server | 离开房间 |
| `room.get` | Client → Server | 获取房间信息 |
| `room.list` | Client → Server | 列出房间 |
| `room.end` | Host → Server | 结束房间 |
| `room.lock` | Host → Server | 锁定房间 |
| `room.unlock` | Host → Server | 解锁房间 |

### 3.2 媒体协商

| 方法 | 方向 | 说明 |
|------|------|------|
| `transport.create` | Client → Server | 创建 WebRTC Transport |
| `transport.connect` | Client → Server | 连接 Transport (ICE) |
| `produce` | Client → Server | 发布媒体 |
| `consume` | Client → Server | 订阅媒体 |
| `producer.pause` | Client → Server | 暂停发布 |
| `producer.resume` | Client → Server | 恢复发布 |
| `consumer.setLayer` | Client → Server | 切换编码层 |
| `consumer.setPreferredLayers` | Client → Server | 设置优选的 SVC 层 |

### 3.3 参与者管理

| 方法 | 方向 | 说明 |
|------|------|------|
| `participant.mute` | Host → Server | 静音参与者 |
| `participant.unmute` | Host → Server | 取消静音 |
| `participant.kick` | Host → Server | 移除参与者 |
| `participant.setRole` | Host → Server | 设置角色 |

### 3.4 录制管理

| 方法 | 方向 | 说明 |
|------|------|------|
| `recording.start` | Host → Server | 开始录制 |
| `recording.stop` | Host → Server | 停止录制 |
| `recording.status` | Client → Server | 查询录制状态 |

## 4. 媒体协商流程

### 4.1 加入房间

```
Client                      Server                      mediasoup
  │                           │                           │
  │  room.join(roomId)        │                           │
  │──────────────────────────►│                           │
  │                           │                           │
  │  ◄────────────────────────│ room.joined               │
  │                           │   { room,                 │
  │                           │     routerRtpCapabilities }│
  │                           │                           │
  │  transport.create(type)   │                           │
  │──────────────────────────►│                           │
  │                           │─── router.createTransport  │
  │                           │──────────────────────────►│
  │                           │◄──────────────────────────│
  │  ◄────────────────────────│ transportCreated          │
  │      { transportId, iceParameters, iceCandidates,     │
  │        dtlsParameters }                                │
  │                           │                           │
  │  transport.connect()      │                           │
  │──────────────────────────►│                           │
  │                           │─── transport.connect()    │
  │                           │──────────────────────────►│
  │                           │◄──────────────────────────│
  │  ◄────────────────────────│ transportConnected        │
  │                           │                           │
  │  produce(kind, rtpParameters)                         │
  │──────────────────────────►│                           │
  │                           │─── producer.create()      │
  │                           │──────────────────────────►│
  │                           │◄──────────────────────────│
  │  ◄────────────────────────│ produced { producerId }   │
  │                           │                           │
  │  consume(producerId)      │  (为其他参与者创建 Consumer)│
  │──────────────────────────►│                           │
  │                           │─── consumer.create()      │
  │                           │──────────────────────────►│
  │                           │◄──────────────────────────│
  │  ◄────────────────────────│ consumed { consumerId,    │
  │      producerId, id,      │
  │      kind, rtpParameters, │
  │      type, producerPaused }                           │
  │                           │                           │
```

### 4.2 订阅新参与者

当参与者 B 加入房间时，已有参与者 A 自动收到事件：

```
Server                      Client A
  │                           │
  │  participant.joined       │
  │──────────────────────────►│
  │  { participant: B }       │
  │                           │
  │  track.added              │
  │──────────────────────────►│
  │  { track: B's video }     │
  │                           │
  │  track.added              │
  │──────────────────────────►│
  │  { track: B's audio }     │
  │                           │
  │  (Client A 可以主动 consume)│
  │                           │
```

## 5. 服务间信令 (gRPC)

### 5.1 Conference Service

```protobuf
service ConferenceService {
  // 房间管理
  rpc CreateRoom(CreateRoomRequest) returns (Room);
  rpc GetRoom(GetRoomRequest) returns (Room);
  rpc EndRoom(EndRoomRequest) returns (Empty);
  rpc ListRooms(ListRoomsRequest) returns (ListRoomsResponse);

  // 参与者管理
  rpc JoinRoom(JoinRoomRequest) returns (JoinRoomResponse);
  rpc LeaveRoom(LeaveRoomRequest) returns (Empty);
  rpc KickParticipant(KickRequest) returns (Empty);
  rpc MuteParticipant(MuteRequest) returns (Empty);

  // 录制
  rpc StartRecording(RecordingRequest) returns (RecordingStatus);
  rpc StopRecording(RecordingRequest) returns (RecordingStatus);
  rpc GetRecordingStatus(RecordingRequest) returns (RecordingStatus);
}
```

### 5.2 SFU 管理 Service

```protobuf
service SfuService {
  // Worker 管理
  rpc GetWorkerStatus(Empty) returns (WorkerStatus);
  rpc ListWorkers(Empty) returns (ListWorkersResponse);
  rpc AllocateWorker(AllocateRequest) returns (Worker);

  // 区域路由
  rpc GetRegionStatus(Empty) returns (RegionStatus);
  rpc RouteToRegion(RouteRequest) returns (RouteResponse);
}
```

## 6. 错误码

| 错误码 | HTTP 类比 | 说明 |
|--------|----------|------|
| `ROOM_NOT_FOUND` | 404 | 房间不存在 |
| `ROOM_FULL` | 403 | 房间已满 |
| `ROOM_LOCKED` | 403 | 房间已锁定 |
| `ROOM_ENDED` | 410 | 房间已结束 |
| `UNAUTHORIZED` | 401 | 未认证 |
| `FORBIDDEN` | 403 | 无权限 |
| `INVALID_PARAMS` | 400 | 参数错误 |
| `TRANSPORT_ERROR` | 500 | 传输层错误 |
| `PRODUCE_FAILED` | 500 | 发布失败 |
| `CONSUME_FAILED` | 500 | 订阅失败 |
| `RATE_LIMITED` | 429 | 频率限制 |
| `INTERNAL_ERROR` | 500 | 内部错误 |

## 7. 参考

- 消息格式借鉴 Colibri2 RESTful API 设计 (Jitsi)
- 服务间 RPC 借鉴 LiveKit PSRPC 模式
- 媒体协商流程遵循 mediasoup 标准 API