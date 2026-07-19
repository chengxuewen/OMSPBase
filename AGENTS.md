# AGENTS.md — OMSPBase Project Knowledge Base

**Generated:** 2026-07-17
**Branch:** `main`

## OVERVIEW

OMSPBase — AUDE 生态多媒体系统。为 AUDESYS 和 AUDEBase 提供统一的多媒体基础设施，涵盖远程桌面、视频会议、直播推拉流、监控相机接入等能力。当前状态: Phase 0 架构定义完成，MVP 实施提案 ready，骨架代码已创建。

## STRUCTURE

```
OMSPBase/
├── .opencode/          # OpenCode 配置（插件、MCP、LSP、instructions）
│   ├── opencode.json   # 主配置：模型、插件、instructions、MCP、LSP
│   ├── agent-guide.md  # AI 代理使用指南（5 层模型体系、OMO 编排）
│   ├── agent-model-tiers.md  # 模型分层体系
│   ├── oh-my-openagent.jsonc  # OMO Agent 配置
│   ├── acp.jsonc       # ACP 配置
│   ├── init-lsp-wrap.mjs      # LSP 包装器初始化
│   ├── init-mcp-*.mjs         # MCP 初始化脚本（codegraph/playwright/postgres/openspace）
│   ├── package.json    # OpenCode 插件依赖
│   └── .gitignore
├── .agents/
│   ├── rules/          # 编码规则文件（16 语言 × common + 中文副本）
│   ├── skills/         # 技能（book-to-skill/doc-audit/openspec-*/test-harness）
│   └── memorys/        # 项目记忆文件 (decisions.md, status.md)
├── crates/              # Rust 工作区 (3 个 member crate)
│   ├── omspbase-remote-host/   # Host 应用 (headless, 采集+编码+推流)
│   ├── omspbase-remote-client/ # Remote 应用 (拉流+解码+控制)
│   └── omspbase-server/ # Server 应用 (信令+relay+监控)
├── docs/               # 设计文档 (architecture.md + modules/ + reference/ + research/)
├── README.md           # 项目简介
├── LICENSE             # Apache 2.0
├── package.json        # 根 package.json（codegraph 开发依赖）
├── bootstrap.sh / bootstrap.bat  # 开发环境引导脚本
├── .rustfmt.toml       # Rust 格式化配置
├── clippy.toml         # Clippy lint 配置
├── deny.toml           # cargo-deny 审计配置
├── rust-toolchain.toml # Rust 工具链版本
└── .gitignore
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| 项目简介 | `README.md` | 功能范围、架构定位、技术栈 |
| Agent 配置 | `.opencode/opencode.json` | instructions、MCP、LSP |
| Agent 使用指南 | `.opencode/agent-guide.md` | OMO 编排体系、5 层模型路由 |
| 模型分层 | `.opencode/agent-model-tiers.md` | 五层模型映射、provider 选择 |
| 语言规则 | `.agents/rules/{lang}/` | 各语言专属规则 |
| 通用规则 | `.agents/rules/common/` | 安全、编码风格、测试、Git 工作流 |
| 架构文档 | `docs/architecture.md` | 整体架构设计 |
| 模块文档 | `docs/modules/` | 各领域详细设计 (13 篇) |
| 项目记忆 | `.agents/memorys/` | 决策记录 (decisions.md)、状态跟踪 (status.md) |
| Rust 源码 | `crates/` | 三个 crate: omspbase-remote-host/remote/server |

## CODE MAP

_项目已进入代码实施阶段。以下为当前状态：_

| 模块 | 状态 | 说明 |
|------|------|------|
| omspbase-remote-host | 🟡 骨架完成 | Host 应用: 采集、编码、推流、信令、配置 |
| omspbase-remote-client | 🟡 骨架完成 | Remote 应用: 拉流、解码、渲染、控制 |
| omspbase-server | 🟡 骨架完成 | Server 应用: 信令 relay、监控、会话管理 |
| omspbase-core | 🔲 计划中 | 微内核: PluginManager, PipelineEngine, LicenseManager |
| napi-binding | 🔲 计划中 | Node.js 绑定: 为 AUDEBase 提供 TypeScript API |
| Phase 2+ crates | 🔲 计划中 | 详见 `.sisyphus/plans/mvp-host-remote/` 和 `docs/architecture.md` |

## CONVENTIONS

### Rust
- Edition 2024，`cargo clippy -- -D warnings`
- `thiserror` 用于库，`anyhow` 用于应用
- `&str` 优先于 `String`，`&[T]` 优先于 `Vec<T>`
- 每个 `unsafe` 块必须有 `// SAFETY:` 注释
- 业务关键 enum 使用完整 match，禁止通配符 `_`

### TypeScript
- 公共 API 显式类型注解
- `interface` 优先于 `type`（对象形状）
- `unknown` > `any`
- Zod 用于边界层模式验证
- 禁止 `as any` / `@ts-ignore` / `console.log`

### C++
- RAII 无处不在 — 不用裸 `new`/`delete`，使用智能指针
- 禁止：`malloc`/`free`、C 风格数组、`strcpy`/`strcat`/`sprintf`
- 始终：`std::array`/`std::vector`、`std::string`、初始化变量

### 通用
- 不可变性优先（永不突变，总是创建新副本）
- 小文件 > 大文件（200-400 行典型，800 行最大）
- 显式错误处理，无静默吞异常
- 布尔值前缀 `is`/`has`/`should`/`can`

## ANTI-PATTERNS

- **`as any` / `@ts-ignore`** — 永不使用，零例外
- **`console.log`** — 生产代码禁止
- **静默吞异常** — `catch(e) {}` 绝对不允许
- **对象突变** — 始终返回新对象，永不就地修改
- **硬编码密钥** — 使用环境变量或密钥管理器
- **不必要的文件写入** — 文档文件仅在用户明确要求时创建
- **Rust `unwrap()` 用于生产** — 使用 `?` 配合 `thiserror`/`anyhow`

## NOTES

- **Phase 0 完成** — 架构定义完成，进入 MVP 实施阶段
- **骨架代码已创建** — `crates/omspbase-{host,remote,server}` 三个 crate 含模块骨架
- **AUDE 生态共享依赖** — AUDESYS 引用 Rust crate，AUDEBase 通过 napi 绑定
