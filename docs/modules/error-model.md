# Error Model — 错误模型

> 状态：Phase 3 前设计 | 关联决策：D-ERR-01, D-ERR-02 | 创建依据：doc-audit M7

## 错误传播

```
Host → [FlatBuffers ErrorPayload] → Server → [HTTP status + JSON body] → Client/Remote
```

每个 crate 内部错误不得直接泄漏到边界。Component 层 `ComponentError` 是统一错误类型，跨进程时序列化为 FlatBuffers `ErrorPayload`（错误码 + 上下文 string）。

## ComponentError (thiserror)

```rust
#[derive(Debug, thiserror::Error)]
pub enum ComponentError {
    #[error("resource not found: {0}")]
    NotFound(String),
    #[error("initialization failed: {0}")]
    InitFailed(String),
    #[error("RPC timeout after {0}ms")]
    RpcTimeout(u64),
    #[error("channel closed")]
    ChannelClosed,
    #[error("component shutting down")]
    Shutdown,
    #[error("internal error: {0}")]
    Internal(String),
    #[error("not authorized: {0}")]
    Unauthorized(String),
    #[error("configuration invalid: {0}")]
    ConfigInvalid(String),
}
```

## 错误码分层 (D-ERR-02)

| 范围 | 层 | 示例 |
|------|-----|------|
| 1xxx | 通用/Component | 1001 NotFound, 1002 InitFailed |
| 2xxx | 传输层 | 2001 ConnectionRefused, 2002 Timeout |
| 3xxx | 资源 | 3001 StreamNotFound, 3002 RoomFull |
| 4xxx | 发现 | 4001 CameraNotFound, 4002 ONVIFAuthFailed |
| 5xxx | 认证授权 | 5001 TokenExpired, 5002 PermissionDenied |
| 6xxx | 媒体 | 6001 CodecUnsupported, 6002 EncoderFailed |

## HTTP 状态映射

```
ComponentError → gRPC status / HTTP status

NotFound         → 404
InitFailed       → 500
RpcTimeout       → 504
ChannelClosed    → 503
Shutdown         → 503
Internal(_)      → 500
Unauthorized(_)  → 401
ConfigInvalid(_) → 400
```

## 日志约定

- **ERROR**: InitFailed, Internal — 需要人工介入
- **WARN**: RpcTimeout, ChannelClosed — 可重试恢复
- **INFO**: Shutdown, NotFound — 正常操作流程
- **必须包含**: `component_id`, `request_id`, 上下文

> 详见 `.sisyphus/plans/component-framework-phase1/design.md`
