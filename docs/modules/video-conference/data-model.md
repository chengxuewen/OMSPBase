# 数据模型

## 1. Room

```typescript
interface Room {
  id: string;                    // UUID
  name: string;                  // 房间名称
  createdAt: string;             // ISO 8601
  status: RoomStatus;
  maxParticipants: number;       // 最大参与人数
  recording: boolean;            // 是否录制
  mode: "conference" | "webinar" | "p2p";
  region: string;                // 部署区域

  // 媒体配置
  mediaConfig: {
    videoCodec: "vp8" | "vp9" | "h264" | "av1";
    audioCodec: "opus";
    simulcast: boolean;
    svc: boolean;
    maxBitrate: number;          // kbps
  };

  // 安全
  security: {
    e2ee: boolean;
    requireAuth: boolean;
    allowedDomains: string[];
  };
}

enum RoomStatus {
  "idle",
  "active",
  "ended",
}
```

## 2. Participant

```typescript
interface Participant {
  id: string;                    // UUID
  roomId: string;
  userId: string;                // 关联用户系统
  displayName: string;
  role: "host" | "speaker" | "listener";
  joinedAt: string;              // ISO 8601

  // 连接信息
  connection: {
    transportId: string;         // mediasoup Transport ID
    iceState: "connected" | "disconnected" | "failed";
    connectedAt: string;
    iceCandidateType: "host" | "srflx" | "relay";
    rtt: number;                 // ms
  };

  // 媒体状态
  media: {
    video: MediaState;
    audio: MediaState;
    screenShare: MediaState;
  };

  // 客户端信息
  client: {
    platform: "web" | "desktop" | "mobile";
    browser?: string;
    version: string;
  };
}

interface MediaState {
  enabled: boolean;
  muted: boolean;
  codec?: string;
  bitrate?: number;
  resolution?: { width: number; height: number };
  frameRate?: number;
}
```

## 3. Track

```typescript
type TrackKind = "audio" | "video" | "screen";

interface Track {
  id: string;
  kind: TrackKind;
  participantId: string;
  roomId: string;

  // mediasoup 映射
  producerId: string;            // mediasoup Producer ID
  routerId: string;              // mediasoup Router ID

  // 编码信息
  encoding: {
    codec: string;
    mimeType: string;            // e.g. "video/VP8"
    clockRate: number;
    channels?: number;           // audio only
  };

  // Simulcast / SVC 层
  layers?: SimulcastLayer[];
  svc?: {
    spatialLayers: number;
    temporalLayers: number;
  };

  // 统计
  stats: {
    bitrate: number;             // kbps
    packetLoss: number;          // %
    jitter: number;              // ms
    rtt: number;                 // ms
  };
}

interface SimulcastLayer {
  id: string;
  rid: "r0" | "r1" | "r2";     // 编码层 ID
  resolution: { width: number; height: number };
  bitrate: number;               // kbps
  active: boolean;               // 是否有订阅者
}
```

## 4. 编码层参考

### 4.1 Simulcast 层

| 层 | RID | 分辨率 | 码率 (VP8) | 码率 (H.264) |
|----|-----|--------|-----------|-------------|
| 低 | r0 | 320×180 | 150 kbps | 120 kbps |
| 中 | r1 | 640×360 | 400 kbps | 350 kbps |
| 高 | r2 | 1280×720 | 1200 kbps | 1000 kbps |

### 4.2 VP9 SVC 层

| 空域 | 时域 | 分辨率 | 帧率 |
|------|------|--------|------|
| S0 | T0 | 320×180 | 7.5 fps |
| S0 | T1 | 320×180 | 15 fps |
| S0 | T2 | 320×180 | 30 fps |
| S1 | T0 | 640×360 | 7.5 fps |
| S1 | T1 | 640×360 | 15 fps |
| S1 | T2 | 640×360 | 30 fps |
| S2 | T0 | 1280×720 | 7.5 fps |
| S2 | T1 | 1280×720 | 15 fps |
| S2 | T2 | 1280×720 | 30 fps |

### 4.3 音频

| 参数 | 值 |
|------|-----|
| 编码 | Opus |
| 采样率 | 48 kHz |
| 声道 | 2 (stereo) |
| 码率 | 30 kbps |
| 帧长 | 20 ms |
| FEC | 启用 |
| DTX | 启用 |

## 5. 权限模型

```typescript
interface ConferencePermission {
  // 功能
  canPublishVideo: boolean;
  canPublishAudio: boolean;
  canShareScreen: boolean;
  canRecord: boolean;
  canMuteOthers: boolean;
  canKick: boolean;
  canLock: boolean;

  // 配额
  maxPublishBitrate: number;     // kbps
  maxResolution: string;         // "720p" | "1080p" | "4k"
  maxParticipants: number;

  // 录制
  recording: "none" | "own" | "all";
}
```

## 6. 事件模型

```typescript
type ConferenceEvent =
  | { type: "room.created"; room: Room }
  | { type: "room.ended"; roomId: string }
  | { type: "participant.joined"; participant: Participant }
  | { type: "participant.left"; participantId: string; reason: string }
  | { type: "participant.muted"; participantId: string; kind: TrackKind }
  | { type: "participant.unmuted"; participantId: string; kind: TrackKind }
  | { type: "track.added"; track: Track }
  | { type: "track.removed"; trackId: string }
  | { type: "track.layer.changed"; trackId: string; layer: string }
  | { type: "dominant.speaker.changed"; participantId: string }
  | { type: "recording.started"; roomId: string }
  | { type: "recording.stopped"; roomId: string }
  | { type: "connection.state"; participantId: string; state: string }
  | { type: "error"; code: string; message: string };
```

## 7. 状态机

### Room 生命周期

```
Created ──→ Active ──→ Ended
                │
                ├──→ Recording (子状态)
                │
                └──→ Locked (子状态, 禁止新加入)
```

### Participant 连接状态

```
Invited ──→ Joining ──→ Connected ──→ Disconnected
                │            │
                └──→ Failed  └──→ Reconnecting ──→ Connected
                                       │
                                       └──→ Disconnected (超时)
```

### Track 生命周期

```
Added ──→ Active ──→ Removed
            │
            ├──→ Muted (暂停转发)
            │
            └──→ LayerChanged (编码层切换)
```