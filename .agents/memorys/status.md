# OMSPBase Status

**生成**: 2026-07-24 | 决策: 187+ (D1-D187) | Phase: 2 完成 | 75 commits

**当前**: 7 crate workspace。Phase 2 mediasoup SFU 集成全部完成 (2a-2d)。Docker/CI/DevContainer 就位，全部使用 Ubuntu 22.04 LTS (mediasoup 预构建基线)。macOS 混合开发工作流 (Host/Client 原生 + Server Docker)。P2P 管线生产就绪 (重连/ICE/STUN/房间配置)。macOS E2E 自动测试 9/9 通过 (信号中继 + SDP + DataChannel)。

## 测试

| Crate | Lib Tests | Integration | 备注 |
|-------|:---------:|:------------:|------|
| omspbase-common | 50 | — | 含 SFU 协议测试 |
| omspbase-media | 54 | — | |
| omspbase-webrtc (stub) | 11 | 67+ | |
| omspbase-webrtc (webrtc-sys) | 11 | 49 (4 ICE 预存) | |
| omspbase-webrtc (webrtc-rs) | 11 | 29 (9 SDP/ICE 预存) | |
| omspbase-codec (stub) | 0 | 32 | |
| omspbase-codec (FFmpeg) | 0 | 35 | |
| omspbase-codec (GStreamer) | 0 | 27 | pixi 环境 |
| omspbase-server | 12 | 30 (25 e2e + 5 integration) | +2 SFU E2E (Linux only) |
| omspbase-host | — | E2E 脚本 9/9 ✅ | macOS native |
| omspbase-client | — | E2E 脚本 9/9 ✅ | macOS native |

### macOS E2E 验证 (2026-07-24)
```
Host (macOS) → WS :9800 → Docker Server → WS :9800 → Client (macOS)
                         └── P2P WebRTC (574 bytes relayed) ──┘
9/9 tests pass: Server health → Build → Host connect → Client connect → SDP → DC → Relay
```

## Phase 进度

| Phase | 状态 |
|-------|:----:|
| 0-1 基础设施 | ✅ |
| 2a-2d mediasoup SFU | ✅ |
| P2P 生产就绪 (#1-5) | ✅ (#4 推迟) |
| Docker/CI/DevContainer | ✅ |
| macOS E2E 验证 | ✅ |
| Phase 3 生产就绪 | 🔲 |

## 决策状态

| 决策 | 内容 | 状态 | Phase |
|------|------|:----:|:-----:|
| D124-D185 | (见 decisions.md) | ✅ | 0-2 |
| D186 | Ubuntu 22.04 统一 | ✅ | 2 |
| D187 | macOS E2E 验证 + webrtc backend 解耦 | ✅ | 0 |
