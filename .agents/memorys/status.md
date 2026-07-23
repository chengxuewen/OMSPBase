# OMSPBase Status

**生成**: 2026-07-23 | 决策: 181+ (D1-D181) | Phase: 0-1 收尾 | 48 commits on main

**当前**: 7 crate workspace。命名简化: omspbase-core→omspbase-common, omspbase-remote-host→omspbase-host, omspbase-remote-client→omspbase-client。依赖清理: codec/media 不再依赖 common。webrtc-rs 后端视频管线完整对齐 webrtc-sys。omspbase-codec 三后端 (stub+FFmpeg+GStreamer)。

**测试**: 各后端独立通过。
- webrtc (stub): 67+ tests 全部通过
- webrtc (webrtc-sys): 49 tests, 4 ICE/SDP 预存失败
- webrtc (webrtc-rs): 29 tests, 9 w3c_api SDP/ICE 预存失败
- codec (stub): 32 tests
- codec (FFmpeg): 35 tests
- codec (GStreamer): 27 tests
- media: 54 tests
- common: 41 tests
- server: 12 tests

**Phase 2 方向**: Track A Host 填充 (采集+编码+推流) → Track C Remote → Integration。
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
|| D173 | RealObserver + FrameSink + VideoSink 接收端实现 | ✅ | 0 |
| D174 | omspbase-codec 双后端完成 | ✅ | 0 |
| D175 | GStreamer codec 后端 (pixi) | ✅ | 0 |
| D176 | webrtc-rs write_raw_i420 接 codec | ✅ | 0 |
| D177 | webrtc-rs P0/P1 视频管线对齐 | ✅ | 0 |
| D178 | E2E P2P 编解码测试框架 | ✅ | 0 |
| D179 | GStreamer 静态链接评估 (延迟) | ✅ | 0 |
| D180 | omspbase-core → omspbase-common 重命名 + 依赖清理 | ✅ | 0 |
| D181 | 移除 remote- 前缀 (host/client) | ✅ | 0 |

| D125, D1, D137-D155 | (省略) | — | 0-2 |
