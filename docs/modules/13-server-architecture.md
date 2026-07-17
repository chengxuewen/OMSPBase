# 13. Server 架构

> Phase 0 — 架构定义 | 2026-07-17
> 关联决策: D86-D91, D98-D101, D118 + 主文档引用: [架构文档](../architecture.md) §2, §10

## 13.1 概述

omspbase-server 是 Phase 1 MVP 的中央中继节点，承担信令中继、媒体转发和运行监控三项职责。Server 不解码也不重编码视频帧，纯 RTP 包转发，延迟增量控制在 30ms 以内。

```
Host (macOS/Linux/Windows)
  │ WebRTC push (RTP/SRTP)
  │ Signaling (WS /ws, JSON SDP/ICE)
  ▼
omspbase-server
  ┌──────────────────────────────────────┐
  │  Signaling Relay (axum WS)           │
  │  /ws endpoint, JSON SDP/ICE 中继     │
  │  room management, PSK auth           │
  ├──────────────────────────────────────┤
  │  Relay Engine                        │
  │  Host PC ↔ Remote PC track bridging  │
  │  纯转发 无转码，不解码不重编码       │
  ├──────────────────────────────────────┤
  │  Monitoring                          │
  │  /health /ready /metrics             │
  │  session stats, rate limiting        │
  ├──────────────────────────────────────┤
  │  Web UI (React + Ant Design)         │
  │  运营面板：设备列表、会话监控        │
  ├──────────────────────────────────────┤
  │  Auth (JWT + role RBAC)              │
  │  SQLite users 表, PSK pre-shared key │
  └──────────────────────────────────────┘
  │ WebRTC forward (RTP/SRTP)
  │ Signaling (WS /ws, JSON SDP/ICE)
  ▼
Remote (macOS/Linux/Windows)
  decode + render + DataChannel control
```

## 13.2 核心组件

| 组件 | 职责 | 决策 |
|------|------|------|
| Signaling Relay | WebSocket /ws，Host/Remote 之间中继 SDP/ICE，不修改内容 | D52, D118 |
| Relay Engine | WebRTC 轨道桥接：Host 推流接入 → Remote 拉流转发 | D86, D118 |
| Room Manager | 单房间 create/join/leave，房间状态管理 (DashMap) | D86 design |
| Auth | PSK 认证 + JWT 签发 + 角色 RBAC (admin/operator/auditor) | D88, D100 |
| Monitoring | /health + /ready + /metrics + 会话统计 | D86, D99 |
| Web UI | React 19 + Ant Design 5，运营人员管理面板 | D87 |

**Relay Engine 核心原则**: Server 不解码也不重编码视频帧。Relay 引擎在 RTP 层面做 track bridging，两条 PeerConnection 之间直接转发 RTP packet。延迟增量 ≤30ms。

## 13.3 技术栈

| 组件 | 选型 | 理由 |
|------|------|------|
| HTTP/WS 框架 | axum 0.7 | WS 同进程，JWT middleware 共享 |
| 数据库 | sqlx + SQLite (编译期 SQL 校验) | 轻量，仅 users 表，无需独立 DB 服务 |
| 配置 | serde_yaml + 环境变量覆盖 | 敏感值 (JWT_SECRET) 通过 env 注入 |
| 日志 | tracing + tracing-subscriber (JSON stdout) | Docker logs 收集 |
| 指标 | prometheus-client (GET /metrics) | Prometheus 格式，Grafana 面板 |
| 中间件 | tower-http (CORS, rate-limit, trace) | 生产级 HTTP 中间件 |
| 认证 | jsonwebtoken (JWT) + argon2 (密码哈希) | 自包含 token，无外部认证服务 |
| Web UI | React 19 + Ant Design 5 + recharts | 与 omspbase-client 共享组件生态 |

## 13.4 房间与中继

Server 使用简单房间模型。一个房间包含一个 Host（推流方）和一个或多个 Remote（拉流方）。

```
Room {
  id: String (UUID v4),
  host: Option<PeerConnection>,
  remotes: Vec<PeerConnection>,
  created_at: Instant,
  status: Waiting | Relaying | Closed,
}
```

Host 加入房间后 → Waiting。Remote 加入后 → Relaying。Remote 离开后 → Waiting。所有参与者离开 → Closed。

## 13.5 认证

两层认证:

1. **PSK (Pre-Shared Key)**: WebSocket upgrade 阶段验证，所有 client (Host/Remote) 共享同一 PSK。PSK 保存在 server.conf，部署时配置。

2. **JWT (Phase 1 可选)**: SQLite users 表存储用户，argon2 哈希密码。JWT 携带 role 字段（admin/operator/auditor），axum middleware 提取到 Extensions<CurrentUser>。

## 13.6 可观测性

| 端点 | 用途 | 说明 |
|------|------|------|
| GET /health | 存活探针 | 轻量，仅返回 200 OK |
| GET /ready | 就绪探针 | 检查信令和 relay 状态 |
| GET /metrics | Prometheus 端点 | HTTP 请求计数/延迟、WS 连接数、转码带宽、会话时长 |

日志全部输出 JSON 到 stdout/stderr，由 Docker logs driver 收集。tower-http TraceLayer 注入 traceId 到每个请求。

## 13.7 状态机

```
INIT → WAITING (等待Host+Remote加入) → RELAYING (转发中) → WAITING (Remote离开)
  │                                                               │
  └── shutdown ──────────────────────────────────────────────────┘
```

## 13.8 部署

Phase 1: 单二进制 + Docker Compose（server + monitoring stack）。

```
docker-compose.yml:
  omspbase-server   # 主服务
  prometheus        # 指标收集
  grafana           # 监控面板
```

server.conf 配置端口、PSK、数据库路径。Docker 部署时通过环境变量注入敏感值。

## 13.9 Phase 演进

| Phase | 架构 | 说明 |
|-------|------|------|
| Phase 1 | 单二进制 (axum + relay 同进程) | 信令、转发、监控合一，~800 行 |
| Phase 2+ | 多节点扩缩容 | relay 可独立水平扩展，信令分离为独立服务 |

## 13.10 交叉引用

- 信令协议和消息格式: [信令架构](10-signaling-architecture.md)
- 整体三层架构: [架构文档](../architecture.md) §2
- 部署形态: [部署模式](02-deployment-modes.md)
- Host 端采集与推流: [客户端与 Host](03-client-host.md)