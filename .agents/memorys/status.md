# OMSPBase Status

> 生成: 2026-07-22 | 决策: 170+ (D1-D173) | Phase: 0-1 | 137 workspace tests | omspbase-codec crate (54 tests, PSNR 100dB) | webrtc 83 tests | FFmpeg backend

**当前**: 7 crate workspace。omspbase-codec Phase 0-1 完成 (stub+FFmpeg双后端, 54 tests, PSNR 100.0 dB roundtrip)。RTC 重命名 + PeerConnectionApi trait + RealObserver/FrameSink 接收端。

**当前**: Phase 0-1 交错。接收端完成: RealObserver (替换 NoOpObserver), FrameSink trait, VideoSink bridge (I420 提取)。RTC 前缀重命名完成。core→media 迁移完成。
**下一步**: 接收端集成测试 + webrtc_loopback_egui 验证。
**Phase 2 方向**: mediasoup SFU + webrtc-sys 默认后端 + Component 框架精简版 + Admin Dashboard SPA。
**MVP 成果**: 5 crate workspace。remote-host(10 modules) + remote-client(9 modules) + server(8) + core(9) + webrtc(8+ modules, triple-backend)。
**测试**: 147 workspace tests。omspbase-media: 54 tests。
**架构文档**: 25 篇模块文档 + 7 篇 SDD。

## 决策状态

| 决策 | 内容 | 状态 | Phase |
|------|------|:----:|:-----:|
| D124 | webrtc test补齐 | ✅ | 1 |
| D156-D165 | (省略, 见 decisions.md) | ✅ | 0-1 |
| D166 | RTC 前缀命名规范 | ✅ | 0 |
| D167 | snake_case 方法命名 | ✅ | 0 |
| D168 | 统一 backend/ 目录 | ✅ | 0 |
| D169 | core pipeline → media 迁移 | ✅ | 0 |
| D170 | RTC 重命名执行 | ✅ | 0 |
| D171 | backends/ → backend/ | ✅ | 0 |
| D172 | webrtc-sys 预存错误修复 | ✅ | 0 |
| D173 | RealObserver + FrameSink + VideoSink 接收端实现 | ✅ | 0 |

| D125, D1, D137-D155 | (省略, 见 decisions.md) | — | 0-2 |
