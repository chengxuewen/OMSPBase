# 16. Admin Dashboard — 管理控制台

> 状态：Phase 4 设计 | 关联决策：D87, D136 Phase 1b | Phase 标签：plan.md Phase 4 (原 D136 Phase 1b)

## 架构概览

React 19 + Ant Design 5 SPA，通过 rust-embed 嵌入 Server 二进制。

```
┌──────────────────────────────────────┐
│  Server                              │
│  ┌────────────────────────────────┐  │
│  │  Gateway Component (:9800)     │  │
│  │  /admin/api/* → REST → Admin  │  │
│  │  /admin/*      → SPA fallback │  │
│  │  /health       → Monitor      │  │
│  └────────────────────────────────┘  │
│              │                        │
│  ┌───────────▼────────────────────┐  │
│  │  rust-embed: admin-ui/dist/    │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘
```

## 技术栈

| 层 | 选型 | 说明 |
|----|------|------|
| 框架 | React 19 | D87 确认 |
| 组件库 | Ant Design 5 | 企业级中后台标准 |
| 构建 | Vite | 快速 HMR + tree-shaking |
| 状态管理 | React Query (TanStack) | SWR 模式，自动缓存失效 |
| 路由 | React Router v7 | SPA 路由 |
| 嵌入 | rust-embed | 编译期嵌入 dist/ |
| 认证 | JWT Bearer + Basic Auth fallback | D88 RBAC |

## 路由设计

| 路径 | 组件 | 权限 |
|------|------|------|
| `/` | Dashboard (仪表盘) | read |
| `/rooms` | Rooms (会话列表) | read |
| `/rooms/:id` | RoomDetail (会话详情) | read |
| `/sessions` | SessionLog (事件日志) | read |
| `/settings` | Settings (配置) | write |
| `/login` | LoginPage (登录) | — |

## Admin API Contract (JSON)

### GET /admin/api/dashboard
```json
{ "active_rooms": 12, "connected_peers": 34, "cpu_pct": 45.2, "mem_mb": 256 }
```

### GET /admin/api/rooms
```json
{ "rooms": [{ "id": "...", "name": "Vehicle-01", "peers": 2, "status": "active" }] }
```

### POST /admin/api/auth/login
```json
{ "username": "admin", "password": "..." }
→ { "token": "eyJ...", "expires_at": "..." }
```

## Auth Guard 流

```
User → LoginPage → POST /admin/api/auth/login → JWT token
     → Dashboard → React Router guard → validate JWT → render
     → 401 → redirect to /login
```

## rust-embed 集成

- Vite build → `admin-ui/dist/`
- Cargo build: `rust-embed` 宏嵌入 dist/ 到二进制
- Gateway C7: `admin/*` 路由 → 静态文件 fallback (index.html)

## Phase 依赖

| 依赖 | Phase | 说明 |
|------|-------|------|
| Gateway Component (C7) | Phase 3 | 路由 + JWT middleware |
| AuthComponent (C8) | Phase 3 | login/validate API |
| MonitorComponent | Phase 3 | /health + stats |
| SQLite (D89) | Phase 3 | 用户/会话持久化 |

## 实施计划 (plan.md Phase 4)

- A1: Vite + React 脚手架
- A2: 路由 + Auth Guard
- A3: Ant Design 布局
- A4: Dashboard API
- A5: Rooms 页面
- A6: Sessions 页面
- A7: Settings 页面
