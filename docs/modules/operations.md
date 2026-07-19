# Operations — 运维设计

> 状态：Phase 3 设计 | 整合：D99 + D-OPS-01~10 + D111 | 创建依据：doc-audit CR4

## 日志策略

| 组件 | 格式 | 输出 | Level |
|------|------|------|-------|
| Host | JSON (tracing-subscriber) | stdout + journald | info |
| Server | JSON (tracing-subscriber) | stdout + journald | info |
| Remote | JSON (tracing-subscriber) | stdout | info |

- tracing span: 每个请求/WS连接一个 span，trace_id 贯穿
- 生产日志: 30 天轮转，压缩归档

## 指标 (Prometheus + /metrics endpoint)

| 指标 | 类型 | 说明 |
|------|------|------|
| `omsp_rooms_active` | gauge | 活跃房间数 |
| `omsp_peers_connected` | gauge | 连接对等点数 |
| `omsp_fps_current` | gauge | 当前帧率 |
| `omsp_latency_ms` | histogram | 端到端延迟 |
| `omsp_component_status` | gauge | 组件状态 (0=stopped,1=running,2=degraded) |
| `omsp_cpu_pct` | gauge | CPU 使用率 |
| `omsp_mem_mb` | gauge | 内存使用 |

## 告警 (Alertmanager)

| 告警 | 条件 | Severity |
|------|------|----------|
| ComponentCrashed | omsp_component_status=0 | critical |
| HighLatency | omsp_latency_ms > 200ms (5min) | warning |
| NoPeers | omsp_peers_connected=0 (1min) | warning |
| HighCPU | omsp_cpu_pct > 90 (5min) | warning |

## TLS

- Phase 1: systemd socket activation + 外部 TLS 终止 (nginx/Caddy)
- Phase 2: 内建 rustls + Let's Encrypt ACME
- 证书轮换: 30 天自动续期

## 备份

- SQLite: `sqlite3 .backup` 每日 + WAL checkpoint
- session_state.json: 文件轮转，保留最近 100 个
- 备份目标: 本地 + 远程 (Phase 2)

## 容量规划

| 部署规模 | Rooms | Peers | CPU | RAM | Disk |
|----------|-------|-------|-----|-----|------|
| 小 | 10 | 20 | 2核 | 2GB | 10GB |
| 中 | 50 | 100 | 4核 | 4GB | 50GB |
| 大 | 200 | 500 | 8核 | 8GB | 200GB |

## Phase 依赖

- Phase 3 Component 框架: /metrics endpoint, tracing span 集成
- Phase 4 Admin Dashboard: 运维面板 (Dashboard, Rooms)
- Phase 2 mediasoup SFU: 媒体质量指标 (丢包率、jitter、RTT)
