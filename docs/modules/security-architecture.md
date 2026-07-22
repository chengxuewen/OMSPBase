# Security Architecture — 安全架构

> 状态：Phase 4 前设计 | 整合：D116 (STRIDE-Lite) + D-SEC-01 (mTLS+TLS+audit) + D117 (E-Stop) + D130 (AuthProvider) | 创建依据：doc-audit CR5

## 威胁模型 (STRIDE-Lite, D116)

| 类别 | 威胁 | 缓解 |
|------|------|------|
| Spoofing | 伪造对等点身份 | JWT + mTLS |
| Tampering | 篡改控制指令 | RTCDataChannel HMAC |
| Repudiation | 否认操作 | 审计日志 |
| Info Disclosure | 信令窃听 | TLS 1.3 |
| DoS | 信令洪泛 | 速率限制 |
| Elevation | 权限提升 | D88 RBAC |

## 认证架构

```
Client → JWT (来自 AuthProvider login) → WebSocket upgrade
       → Token 在 HTTP header: Authorization: Bearer <jwt>
       → AuthComponent validate(token) → User + Permissions
```

## JWT Token 生命周期

| 阶段 | 说明 | Config |
|------|------|--------|
| 签发 | AuthComponent.login() → JWT | exp: 24h |
| 验证 | 每次 API/WS 请求 validate() | — |
| 刷新 | POST /admin/api/auth/refresh → 新 token | exp: 24h |
| 吊销 | DELETE /admin/api/auth/revoke → 黑名单 (SQLite) | 即时生效 |
| 轮转 | JWT_SECRET 定期更换 → 所有 token 失效 | 30 天 |

## mTLS (Phase 2)

- Phase 1: Server 端 TLS (rustls) + HTTP Basic Auth 备选
- Phase 2: 双向 mTLS，对等点通信加密
- 证书: X.509，ECDSA P-256，90 天有效期

## 证书生命周期管理

Phase 1: 自签 CA + 手动轮换。TLS 证书 90 天有效期。私钥文件权限 0600。
Phase 2: Let's Encrypt (ACME) 自动续期，CRL 吊销列表。
信任链: Root CA → Intermediate CA (可选 Phase 2) → Leaf certificate。

## 密钥管理

密钥轮转策略：

| 密钥类型 | 轮转周期 | 共存窗口 | 触发方式 |
|---------|---------|---------|---------|
| WS PSK | 30 天 | 24h | 手动/配置重载 |
| JWT 签名密钥 | 90 天 | 24h (kid header) | 手动 |
| TLS 私钥 | 90 天 | 0 (即时切换) | 手动/ACME |
| HMAC 控制命令密钥 | 30 天 | 1h (旧帧重放窗口) | 手动 |
## 审计事件 Schema

```json
{
  "event_id": "uuid",
  "timestamp": "ISO8601",
  "actor": "user_id | peer_id",
  "action": "login | room.create | peer.connect | admin.config | e-stop",
  "resource": "room_id | config_key",
  "result": "success | denied | error",
  "source_ip": "optional"
}
```

- Phase 1: 审计日志 → stdout (journald)
- Phase 2: 审计日志 → SQLite (admin queryable)

## WebSocket PSK 轮转

- Phase 1: 静态 PSK (环境变量)，服务重启时更换
- Phase 2: 动态轮转，每 24h 协商新 PSK

## 速率限制

| 端点 | 限制 | 窗口 |
|------|------|------|
| /admin/api/auth/login | 5 req | 1min |
| /admin/api/* | 100 req | 1min |
| /api/* | 无限制 | — |

## Phase 依赖

- Phase 3 Component 框架: AuthComponent (JWT + 速率限制)
- Phase 4 Admin Dashboard: login/logout/session 过期 UX
- Phase 2 mediasoup SFU: 信令层 mTLS
