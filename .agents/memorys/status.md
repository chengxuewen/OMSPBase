# OMSPBase Status

> 生成: 2026-07-17 | 决策数量: 150+ (D1-D118 + ~30 sub-decisions) | Phase: 0→1 过渡 | MVP 实施启动

## Phase

**当前**: Phase 0 架构定义完成 → Phase 1 MVP 实施阶段。
**MVP 计划**: Host → Server → Remote 三组件遥操作 v2 (D118)。32 任务，~3300 行。
**架构文档**: 15 篇模块文档 (含新增 13-server / 14-remote)。7 篇 SDD (docs/sdd/)。
**审计**: 2026-07-17 doc-audit 完成，54/58 项修复已应用。
**骨架**: crates/omspbase-{host,remote,server} 已创建，待填充实现。

**当前**: Phase 0 架构定义完成 → MVP 实施提案 ready (.sisyphus/plans/mvp-host-remote/)，骨架代码已创建 (crates/omspbase-{host,remote,server})

## 决策状态

| 决策 | 内容 | 状态 | Phase |
|------|------|:----:|:-----:|
| D1 | 控制面+数据面分离 | ✅ | 0 |
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

## 文档清单

### 架构文档 (docs/modules/)
00-overview.md · 01-product-capabilities.md · 02-deployment-modes.md · 03-client-host.md · 04-sdk-layers.md · 05-auth-permissions.md · 06-plugin-system.md · 07-protocols.md · 08-pipeline-model.md · 09-transport-architecture.md · 10-signaling-architecture.md · 11-napi-binding.md · 12-recording-playback.md · 13-server-architecture.md · 14-remote-architecture.md

### SDD 文档 (docs/sdd/)
README.md · 01-camera-capture.md · 02-webrtc-push.md · 03-decode-render.md · 04-datachannel-control.md · 05-host-web-config.md · 06-server-monitoring.md · 07-emergency-stop.md

### 审计报告
docs/doc-audit-2026-07-17.md · 54/58 项修复已应用
00-overview.md · 01-product-capabilities.md · 02-deployment-modes.md · 03-client-host.md · 04-sdk-layers.md · 05-auth-permissions.md · 06-plugin-system.md · 07-protocols.md · 08-pipeline-model.md · 09-transport-architecture.md · 10-signaling-architecture.md · 11-napi-binding.md · 12-recording-playback.md

### 实施提案

**.sisyphus/plans/mvp-host-remote/** — 32 任务，~3300 行
proposal.md · design.md · tasks.md
.sisyphus/plans/mvp-host-remote/proposal.md · design.md · tasks.md
