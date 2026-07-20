# OMSPBase Status

> 生成: 2026-07-20 | 决策数量: 161+ (D1-D161) | Phase: 0-1 | 147 tests passing | WebRTC triple-backend (stub/webrtc-rs/webrtc-sys) | loopback 测试完整 | macOS -ObjC linker 修复

## Phase

**当前**: Phase 0-1 交错。P0 (PcBackend default methods) + P1 (RTCStats/RtpParameters) 移植完成。webrtc-sys backend 创建完成。loopback 集成测试 13 tests。egui 示例可运行。
**下一步**: Phase 0 剩余: P2 VideoTrackSource + P3 AudioTrackSource (从 webrtc-kit 移植)。Phase 1 Transport RTP 迁移 → Phase 2 mediasoup SFU。
**Phase 2 方向**: mediasoup SFU + webrtc-sys 默认后端 + Component 框架精简版 + Admin Dashboard SPA。
**MVP 成果**: 5 crate workspace。remote-host(10 modules) + remote-client(9 modules) + server(8) + core(9) + webrtc(8+ modules, triple-backend)。
**测试**: 147 workspace tests。WebRTC crate 34 tests (w3c-api 21 + loopback 13)。全部 3 后端编译通过。stub 34 pass, webrtc-rs 34 pass, webrtc-sys compiles clean。
**架构文档**: 23 篇模块文档 + 7 篇 SDD。审计 26 项发现已应用 (doc-audit 2026-07-19)。

## 决策状态

| 决策 | 内容 | 状态 | Phase |
|------|------|:----:|:-----:|
| D124 | webrtc test补齐: 18 tests → 34 tests (sdp serde, stub, w3c-api, loopback) | ✅ | 1 |
| D156 | P0: PcBackend default method 模式 (from webrtc-kit, 5 methods) | ✅ | 0 |
| D157 | P1: RTCStats + RtpParameters 类型定义 (from webrtc-kit) | ✅ | 0 |
| D158 | webrtc-sys backend: backend/webrtc_sys.rs + feature + compile_error! guard | ✅ | 0 |
| D159 | macOS linker: .cargo/config.toml -ObjC flag (libwebrtc ObjC categories) | ✅ | 0 |
| D160 | loopback 测试: tests/common/loopback.rs + tests/webrtc_loopback.rs (13 tests) | ✅ | 0 |
| D161 | egui 示例: examples/webrtc_loopback_egui.rs (eframe GUI 视频预览) | ✅ | 0 |

|
| D125 | PipelineEngine hot-plug 边界测试: 6→11 tests | ✅ | 1 |

| D1 | 控制面+数据面分离 | ✅ | 0 |
| D137 | WebRTC DC→RTP track 升级 (默认后端 webrtc-sys, D139修正) | ✅ | 1-2 |
| D138 | mediasoup SFU 架构确认 (mediasoup-sys v0.22) | ✅ | 2 |
| D139 | 后端命名: backend-libwebrtc → backend-webrtc-sys | ✅ | 1 |
| D140 | Phase 0 Feature Gate: 只实现 webrtc-sys，其余 compile_error! 占位 | ✅ | 0 |
| D141 | F6 删除: DC 媒体路径不在 omspbase-webrtc crate 层 | ✅ | 0 |
| D142 | 整合 MVP 计划采纳: 52 任务 6 阶段 (consolidated-mvp) | ✅ | 0 |

| D143 | Phase 1-2 已知风险接受: 7 项审计通过，不阻塞 Phase 0 | ✅ | 0 |

| D144 | 借鉴 webrtc-kit 多后端 trait 抽象模式 (feature-gate + cfg dispatch) | ✅ | 0 |

| D145 | Phase 0 PeerConnection struct 非 trait (Phase 2 多后端后提取) | ✅ | 0 |

| D146 | W3C API 命名兼容 (createOffer/createAnswer/addTrack/onTrack/...) | ✅ | 0 |

| D147 | AudioTrack 设计: Opus 48kHz 2ch, 多轨 per PC | ✅ | 0 |

| D148 | 多轨管理: HashMap<String, TrackRef> 注册表, 上限 8 | ✅ | 0 |

| D149 | DataChannel 保留策略: 控制指令专用, media 移入 RTP track | ✅ | 0 |

| D150 | omspbase-core 零 WebRTC 依赖 (MediaTransport trait 定义在此) | ✅ | 0 |

| D151 | RtcEngine::create_factory() cfg dispatch 入口 | ✅ | 0 |

| D2 | Client + Host 双应用 | ✅ | 0 |
| D3 | 微内核 + 插件体系 (MVP: 1 binary) | ✅ | 0 |
| D4 | Auth 双模式 (Local/AUDEBase) | ✅ | 0 |
| D5 | Unified Fragment Model | ✅ | 0 |
| D6 | GStreamer 热路径 | ✅ | 0 |
| D7 | Rust 自研热路径 | ✅ | 0 |
| D8 | 四部署形态 | ✅ | 0 |
| D9-D11 | WebRTC 三后端 (MVP: libwebrtc) | ✅ | 0 |
| D12 | 独立 axum/tonic 信令服务 | ✅ | 0 |
| D13 | Plugin 双模式加载 (MVP: 编译期) | ✅ | 0 |
| D14 | LiveKit 纯 SFU 插件 (→ D97) | ✅ | 0 |
| D15 | P2P 优先无 SFU (MVP 基准) | ✅ | 0 |
| D16-D19 | 参考 + Cargo workspace | ✅ | 0 |
| D20-D27 | 管线架构 (TextureHandle, 时间戳, 广播) | ✅ | 0 |
| D28-D30 | 插件注册 + Manager + Capability | ✅ | 0 |
| D31-D33 | Transport trait + sans-I/O + dispatch | ✅ | 0 |
| D34-D40 | 录制架构 (Phase 2) | ✅ | 0 |
| D41-D45 | GPU 编码/采集架构 | ✅ | 0 |
| D46-D47 | 解码 + 渲染 trait | ✅ | 0 |
| D48 | 软件编解码 VP8 优先 | ✅ | 0 |
| D49 | AudioProcessor trait (MVP: 无音频) | ✅ | 0 |
| D50 | SFU 转发+客户端混音 (Phase 2) | ✅ | 0 |
| D51 | Protobuf 双格式 信令 | ✅ | 0 |
| D52 | 单进程 axum HTTP+WS 信令 | ✅ | 0 |
| D53 | 统一 Room + Topology 枚举 | ✅ | 0 |
| D54 | 单层 sans-I/O SignalHandler trait | ✅ | 0 |
| D55-D56 | napi-rs Session + SessionType (Phase 2) | ✅ | 0 |
| D57 | gRPC Auth 最小合约 (Phase 2) | ✅ | 0 |
| D58-D60 | 插件隔离渐进策略 (Phase 2) | ✅ | 0 |
| D61 | 录制延后至 Phase 2 | ✅ | 0 |
| D62 | Host 多进程 Phase 策略 (→ D102) | ✅ | 0 |
| D63 | Phase 1 Host 单进程 (D102 修正) | ✅ | 0 |
| D64-D67 | CameraCapture, field SDK | ✅ | 0 |
| D68 | SDK 命名去掉 -sdk 后缀 | ✅ | 0 |
| D69 | Facade 模式 | ✅ | 0 |
| D70-D72 | FFmpeg/codec 策略 | ✅ | 0 |
| D73 | 最低 Ubuntu 20.04 | ✅ | 0 |
| D74 | WS Phase 1 + MQTT Phase 2 | ✅ | 0 |
| D75 | I420 标准格式 | ✅ | 0 |
| D76 | remote vs client 分离 | ✅ | 0 |
| D154 | crate 重命名: host→remote-host, remote→remote-client (对齐工业惯例) | ✅ | 0 |
| D155 | Host 单体架构确认: GStreamer + webrtc-sys 同进程共存 | ✅ | 0 |
| D77 | Host 跨平台 relay-default | ✅ | 0 |
| D78 | P2P/relay 可强制 | ✅ | 0 |
| D79 | C/C++/Python 三语言绑定 | ✅ | 0 |
| D80 | Remote config 只读 | ✅ | 0 |
| D81 | Host 内嵌 Web 配置 | ✅ | 0 |
| D82 | 10 crates + 3 binaries | ✅ | 0 |
| D83 | .a + .so 双格式输出 | ✅ | 0 |
| D84 | Host Web axum + SSE | ✅ | 0 |
| D85 | Host Web HTTP Basic Auth | ✅ | 0 |
| D86-D87 | Server 监控+JWT | ✅ | 0 |
| D88 | 单 admin+JWT → RBAC (D100) | ✅ | 0 |
| D89 | Server SQLite | ✅ | 0 |
| D90 | Server feature flags (D98) | ✅ | 0 |
| D91 | Server tracing + stdout | ✅ | 0 |
| D92 | MVP 7 项核心功能 | ✅ | 0 |
| D93 | MVP 验收标准 (150ms/50ms) | ✅ | 0 |
| D94 | 7 SDD + 4 层测试 | ✅ | 0 |
| D95 | LiveKit SFU (→ D97 替代) | ✅ | 0 |
| D96 | 默认 relay + P2P 可选 | ✅ | 0 |
| D97 | mediasoup SFU (Phase 2) | ✅ | 0 |
| D98-D101 | Server 架构四维度 | ✅ | 0 |
| D102 | Phase 1 单进程 (回退 D63/D103) | ✅ | 0 |
| D103 | iceoryx2 Phase 2 (D102 修正) | ✅ | 0 |
| D104 | Host tarball + install.sh | ✅ | 0 |
| D105 | cargo-c SDK (Phase 2, D109 现有) | ✅ | 0 |
| D106 | SDK /usr/local 安装 | ✅ | 0 |
| D107 | Host /opt/oomspbase 安装布局 | ✅ | 0 |
| D108 | systemd Type=notify | ✅ | 0 |
| D109 | C SDK: cbindgen + 手写 CMake/pc | ✅ | 0 |
| D110 | Docker Phase 2 延后 | ✅ | 0 |
| D111 | /health + /ready 端点 | ✅ | 0 |
| D112 | CI/CD build 阶段 tarball | ✅ | 0 |
| D113 | MVP 测试架构 (4-layer) | ✅ | 0 |
| D114 | host.conf Schema | ✅ | 0 |
| D115 | 管线延迟预算 (8 nodes/110ms) | ✅ | 0 |
| D116 | STRIDE-Lite 威胁模型 | ✅ | 0 |
| D117 | 紧急停止+控制安全 | ✅ | 0 |
| D118 | MVP v2: Host→Server→Remote 三组件架构 | ✅ | 1 |
| D119 | pixi 环境 + Transport de-stub + GStreamer 编译 | ✅ | 1 |
| D120 | PipelineEngine: 源并行 + 处理器链串行 + Sink 扇出 + 热插拔 | ✅ | 1 |
| D121 | webrtc-rs 0.12 重写: pure Rust 双实现 (stub + backend) | ✅ | 1 |
| D122 | PipelineEngine host 集成: GstCaptureSource + WebrtcOutputSink 适配器 | ✅ | 1 |
| D123 | PipelineEngine remote 集成: FrameSource + DecodeSink 适配器 | ✅ | 1 |
| D-ERR-01 | 错误分类 | ✅ | 0 |
| D-ERR-02 | 错误码编号 1xxx-9xxx | ✅ | 0 |
| D-SEC-01 | 安全架构 mTLS+TLS+audit | ✅ | 0 |
| D-CI-01 | GitHub Actions 3-stage | ✅ | 0 |
| D-HW-01 | Jetson Nano 硬件基线 | ✅ | 0 |
| D-HW-02 | 2-state IDLE↔PUSH | ✅ | 0 |
| D-TEST-01 | cargo test + 手写 mock | ✅ | 0 |
| D-SAFETY-02 | SafetyEnvelope 5-level | ✅ | 0 |
| D-OPS-01~07 | 7 运维决策 | ✅ | 0 |
| D-OPS-09 | 8 Prometheus 告警规则 | ✅ | 0 |
| D-OPS-10 | Host 升级策略 | ✅ | 0 |

## 源码统计 (crates/)

| Crate | 模块数 | 测试数 | 行数 | 状态 |
|-------|:------:|:------:|------|:----:|
| omspbase-core | 9 | 58 | ~1800 | ✅ |
| omspbase-remote-host | 10 | 21 | ~1550 | ✅ |
| omspbase-server | 8 | 37 | ~1423 | ✅ |
| omspbase-remote-client | 9 | 13 | ~1050 | ✅ |
| omspbase-webrtc | 8+ | 34 | ~2000+ | ✅ triple |
| **workspace total** | **44+** | **147** | **~7900** | ✅ |
| **workspace total** | **41** | **135**† | **~6400** | ✅ |

† workspace 与 per-crate 计数差异: GStreamer-feature 测试仅在 per-crate 计入。58+21+37+13+18=147 per-crate。

---

## 命名变更历史

| 旧名称 | 新名称 | 理由 | 决策 |
|--------|--------|------|:----:|
| omspbase-host | omspbase-remote-host | 对齐工业惯例 (host=被控侧推流) | D154 |
| omspbase-remote | omspbase-remote-client | 对齐工业惯例 (client=主控侧拉流) | D154 |
| omspbase-remote-c (Phase 2+) | omspbase-remote-client-c | 命名一致性 | D154 |
| omspbase-remote-sdk | omspbase-remote-client-sdk | 命名一致性 | D154 |

## 整合计划

`.sisyphus/plans/consolidated-mvp/plan.md` — 52 任务, 6 阶段 (D142)

| Phase | 名称 | 任务数 | 依赖 | 关键决策 |
|-------|------|:-----:|------|----------|
| 0 | Foundation — webrtc-sys Track API | 9 (1 removed) | 无 | D137, D139, D140, D144-D151 |
| 1 | Transport — Host/Remote RTP 迁移 | 7 | Phase 0 | D137, D141 |
| 2 | Server — mediasoup SFU | 18 | Phase 0 | D138 |
| 3 | Components — 精简框架 | 10 | Phase 1+2 | D136 |
| 4 | Admin — Dashboard SPA | 7 | Phase 3 | D136 |
| 5 | Plugin + 集成测试 | 7 | 全部 | D136 |
