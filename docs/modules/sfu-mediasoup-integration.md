# SFU — mediasoup Integration

> 状态：Phase 2 设计 | 关联决策：D138 | 创建依据：doc-audit H7

## 概述

mediasoup-sys v0.22 作为 OMSPBase SFU 服务器，负责 Room 内 Host→Server→Remote 的媒体流转发。所有操作封装在 `SfuComponent` 中。

## 核心概念映射

| mediasoup | OMSPBase Component |
|-----------|-------------------|
| Worker | SfuComponent::WorkerPool |
| Router | RoomRouter (per-room) |
| Transport (PlainRtp) | HostRtpTransport (推流) |
| Transport (WebRtc) | RemoteWebRtcTransport (分发) |
| Producer | TrackProducer (上行) |
| Consumer | TrackConsumer (下行) |
| SDP ↔ RtpParameters | SdpAdapter (双向 codec) |

## Worker 生命周期

```
SfuComponent::init()
  ├─ WorkerPool (CPU 核数个 Worker)
  ├─ Worker::create({ rtcMinPort: 40000, rtcMaxPort: 49999 })
  └─ 监听 "died" → 自动重启
```
崩溃恢复：标记 Router dead → RoomManager 通知 Remote 重连 → WorkerPool::respawn → Remote 重建 Transport + Consumer。

## Router 配置

```rust
worker.create_router(RouterOptions {
    media_codecs: vec![
        RtpCodecCapability::Video { mime_type: "video/VP8", ... },
        RtpCodecCapability::Video { mime_type: "video/H264", ... },
        RtpCodecCapability::Audio { mime_type: "audio/opus", ... },
    ],
});
```

每 Room 一个 Router，`room_id` 索引。Room 关闭时销毁。

## Transport 创建

**PlainRtp (Host→Server)**:
```
Host → SDP offer → SdpAdapter::parse_offer()
  → router.createPlainRtpTransport()
  → transport.connect({ip, port})
```

**WebRtc (Server→Remote)**:
```
router.createWebRtcTransport({
    listen_ips: [{ip: "0.0.0.0", announced_ip: server_public_ip}],
    enable_udp: true, enable_tcp: true, prefer_udp: true,
}) → dtlsParameters + iceParameters
```

## Producer / Consumer

```
Host:  transport.produce({kind, rtpParameters}) → Producer {id, kind}
Remote: transport.consume({producer_id, rtp_capabilities}) → Consumer {id, paused: false}
```
Consumer pause/resume 控制带宽。

## SdpAdapter (S8)

双向 SDP ↔ mediasoup RtpParameters：

```rust
trait SdpAdapter {
    fn parse_offer(sdp: &str) -> Result<RtpCapabilities>;
    fn to_send_rtp_params(sdp: &str) -> Result<RtpParameters>;
    fn create_answer(caps: &RtpCapabilities) -> Result<String>;
}
```

内部使用 `webrtc-sdp` crate 解析，编解码器映射。

## Observer 集成

- **AudioLevelObserver**: 每 Router 挂一个，音量跟踪 → 自动静音
- **ActiveSpeakerObserver**: 活跃说话者检测 → 画中画切换 → RoomManager

## 崩溃恢复流程

```
1. Worker "died" → 标记 dead
2. Router → RoomEvent::WorkerLost { room_id }
3. RoomManager → broadcast RejoinRequired
4. WorkerPool::respawn
5. Remote join → 重建 Transport + Consumer
```

> 详见 `.sisyphus/plans/consolidated-mvp/plan.md` Phase 2 (S1-S18)
