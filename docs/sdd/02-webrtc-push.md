# SDD 02: WebRTC Push

## 1. 概述

WebRTC 推流模块。将 I420 帧编码为 H.264/H.265，经 libwebrtc 推送到 Server relay。

**决策引用**: D11 (三后端架构), D32 (编译期分发), D-ERR-01 (熔断器)

## 2. 接口定义

```rust
#[async_trait]
pub trait StreamPublisher: Send + Sync {
    fn name(&self) -> &str;
    fn supported_codecs(&self) -> Vec<Codec>;
    async fn connect(&mut self, signaling: &SignalingConfig) -> Result<()>;
    async fn push_frame(&mut self, frame: EncodedFrame) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
    fn stats(&self) -> StreamStats;
}

pub struct StreamStats {
    pub fps: f64,
    pub bitrate_kbps: u32,
    pub rtt_ms: u32,
    pub packets_lost: u64,
    pub ice_state: IceState,
}

pub struct SignalingConfig {
    pub ws_url: String,
    pub room_id: String,
    pub peer_id: String,
    pub psk: Option<String>,
}
```

### 编码配置

```rust
pub struct EncodeConfig {
    pub codec: Codec,           // H264 | H265
    pub width: u32,             // 1280
    pub height: u32,            // 720
    pub fps: u32,               // 30
    pub bitrate_kbps: u32,      // 2000
    pub gop: u32,               // 30
    pub preset: EncoderPreset,  // P1-P7
}
```

## 3. 后端策略 (D11)

| 后端 | 场景 | feature flag |
|------|------|-------------|
| str0m | Embed/LAN, sans-I/O 轻量 | backend-str0m (默认) |
| libwebrtc | 公网弱网, GCC+FEC | backend-libwebrtc |
| webrtc-rs | 未来升级目标 | backend-webrtc-rs |

编译期 `#[cfg(feature)]` 互斥，一次编译一个后端。

## 4. 信令流程 (WHIP/WHEP)

```
Remote              Server              Host
  │                   │                  │
  │─── SDP Offer ────▶│─── relay ───────▶│
  │                   │                  │
  │◀── SDP Answer ────│◀── relay ────────│
  │                   │                  │
  │◀══════ ICE ═══════▶◀══════ ICE ══════▶
  │                   │                  │
  │═══════════ RTP/SRTP ═════════════════▶
```

## 5. 错误处理 (D-ERR-01 熔断器)

| 条件 | 分类 | 错误码 | 恢复 |
|------|------|--------|------|
| ICE 连接超时 | Recoverable | 1003 | ICE restart + 指数退避 |
| WebSocket 断连 | Recoverable | 1001 | 指数退避重连 (1s, 2s, 4s...) |
| 编码器初始化失败 | Fatal | 2001 | fallback 软件 VP8 |
| 5 次失败 / 60s | 熔断 | 1001 | 停止推流, 等待 30s 恢复 |

熔断器状态机: `Closed → Open (5 failures / 60s) → HalfOpen (wait 30s) → Closed`

## 6. 测试计划

| 测试 | 类型 | 描述 |
|------|------|------|
| stream_startup | 集成 | connect → push_frame → disconnect 生命周期完整 |
| simulcast_enabled | 集成 | 多分辨率推流, 确认各层独立编码 |
| ice_restart | 集成 | 模拟 ICE 断连 → ICE restart → 恢复 |
| bitrate_adaptation | 集成 | GCC 根据模拟带宽变化调整码率 |
| circuit_breaker_open | 单元 | 5 次失败后熔断触发, Open 状态阻塞推流 |
| circuit_breaker_recover | 单元 | Open 状态 30s 后转为 HalfOpen |
| codec_negotiation | 单元 | SDP Offer/Answer 中 codec 协商正确 |
| stats_reporting | 单元 | stats() 返回实时 fps/bitrate/rtt |