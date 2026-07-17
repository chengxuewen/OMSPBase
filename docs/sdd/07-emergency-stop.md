# SDD 07: Emergency Stop

## 1. 概述

紧急停止模块。独立于 WebRTC 的 UDP 紧急停止通道，确保主通信路径失效时仍可安全停止车辆。

**决策引用**: D117 (紧急停止独立通道), D-SAFETY-02 (SafetyEnvelope), D116 (STRIDE-Lite)

## 2. 接口定义

```rust
/// 紧急停止通道 (独立 UDP)
pub trait EmergencyStop: Send + Sync {
    fn bind(&mut self, addr: SocketAddr) -> Result<()>;
    fn send_stop(&self) -> Result<()>;
    fn poll_stop(&mut self, timeout: Duration) -> Result<Option<()>>;
    fn last_heartbeat(&self) -> Option<Instant>;
}

/// 心跳帧 (车控器定期发送)
pub struct Heartbeat {
    pub seq: u32,
    pub timestamp: Instant,
    pub state: ControlState,
}
```

### SafetyEnvelope trait (D-SAFETY-02)

```rust
pub trait SafetyEnvelope: Send {
    fn check(&self, state: &ControlState) -> SafetyLevel;
    fn limits(&self) -> &ControlLimits;
}

pub enum SafetyLevel {
    Normal,
    Warning,
    Limit,
    SoftStop,   // 2s 内减速到零
    HardStop,   // 立即切断动力
}

pub struct ControlLimits {
    pub max_steering_angle: f32,
    pub max_speed: f32,
    pub max_accel: f32,
    pub timeout_ms: u32,
    pub rtt_warning_ms: u32,
    pub rtt_emergency_ms: u32,
}
```

## 3. 架构

```
Remote                   车控器 (ECU)
  │                         │
  ├── WebRTC DataChannel ──▶│ (主控制路径)
  │                         │
  ├── UDP 紧急停止 ────────▶│ (独立路径)
  │   0xFF byte            │
  │   QoS DSCP EF         │
  │                         │
  │              heartbeat ◀─── (150ms 周期)
```

### 关键设计

| 特性 | 值 |
|------|-----|
| 传输 | 纯 UDP, 无 WebRTC 栈 |
| 数据 | 单字节 `0xFF` = 紧急停止 |
| 端口 | 独立 UDP 端口 (不同 DataChannel) |
| QoS | DSCP EF (46) 优先转发 |
| 重试 | 3 次发送, 50ms 间隔 |
| 目标延迟 | <20ms |

### 超时分级 (D-SAFETY-02)

```
L0 Normal    (RTT < 100ms)  → 正常控制
L1 Warning   (RTT 100ms+)   → 允许执行, 告警
L2 Limit     (RTT 150ms+)   → 限速限角
L3 SoftStop  (RTT 300ms+)   → 2s 减速到零
L4 HardStop  (RTT 500ms+)   → 切断动力 + UDP 紧急停止
```

## 4. 独立性验证

| 故障场景 | WebRTC DataChannel | UDP 紧急停止 | 结果 |
|----------|-------------------|--------------|------|
| WebRTC 断开 | ❌ | ✅ | 仍可紧急停止 |
| DTLS 握手失败 | ❌ | ✅ | 仍可紧急停止 |
| 4G 弱网高延迟 | ❌ (丢包) | ✅ | 仍可紧急停止 |
| WebRTC 进程崩溃 | ❌ | ✅ (ECU 直接) | 仍可紧急停止 |
| UDP 端口不可达 | ✅ | ❌ | 超时触发 SoftStop |

## 5. 错误处理

| 条件 | 分类 | 恢复 |
|------|------|------|
| UDP 丢包 (单次) | Transient | 重试 3 次, 50ms 间隔 |
| UDP 连续丢包 | Recoverable | 车控器 heartbeat 超时自动 SoftStop |
| 心跳超时 150ms | Recoverable | 自动 SoftStop |
| 心跳超时 500ms | Fatal | 自动 HardStop |

## 6. 测试计划

| 测试 | 类型 | 描述 |
|------|------|------|
| stop_latency | 集成 | UDP 紧急停止端到端 <20ms |
| webrtc_independence | 集成 | WebRTC 断开时 UDP 停止仍工作 |
| heartbeat_timeout_soft | 集成 | 150ms 无心跳触发 SoftStop |
| heartbeat_timeout_hard | 集成 | 500ms 无心跳触发 HardStop |
| safety_level_escalation | 单元 | RTT 递增时 SafetyLevel 正确升级 |
| control_limits_apply | 单元 | Limit 级别正确钳制速度/角度 |
| retry_3_times | 集成 | UDP 丢包后自动重试 3 次 |
| dscp_ef_marking | 集成 | 紧急停止包 DSCP 字段为 46 |
| safety_envelope_vehicle | 实车 | 车控器 SafetyEnvelope 独立判定一致 |