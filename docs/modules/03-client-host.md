# 客户端与 Host

> OMSPBase 提供两种客户端形态：Client（桌面 GUI 全功能）和 Host（Headless 远端）。

---

## 对比总览

| 维度 | OMSPBase Client | OMSPBase Host |
|------|-----------------|----------------|
| **运行环境** | 桌面操作系统 | 服务器 / 边缘 / 车端 / 无桌面 |
| **GUI** | Tauri v2 | Embedded Web (localhost 配置页) |
| **SDK** | 全量（生产 + 消费） | 仅生产（capture, encode, push） |
| **角色** | 可控制他人，也可被控制 | 仅产出媒体流 |
| **安装** | 桌面安装包 | 单一二进制 `omspbase-host` |
| **体积** | 大（含 GUI 框架） | 小（无 GUI 依赖） |

**双应用决策原因**：Host 需要运行在没有桌面环境的平台上（无 GUI 的 Linux 服务器、车端嵌入式设备）。

---

## 一、Client — 桌面 GUI 全功能应用

### 场景
操作员桌面，可远程控制他人，也可被远程控制。

### 架构
```
┌─────────────────────────┐
│  Tauri v2 GUI     │
│  ┌───────────────────┐  │
│  │ React 前端         │  │
│  │ 权限驱动 UI Module │  │
│  └───────────────────┘  │
│  ┌───────────────────┐  │
│  │ omspbase-core    │  │
│  │ 全量 SDK          │  │
│  └───────────────────┘  │
└─────────────────────────┘
```

### 权限驱动 UI

客户端启动时从后台拉取权限配置，动态加载 UI Module：

```typescript
const permissions = await backend.getPermissions(userId);

const modules: Module[] = [];
if (permissions.streaming)    modules.push(StreamingModule);
if (permissions.remote)       modules.push(RemoteDesktopModule);
if (permissions.conference)   modules.push(ConferenceModule);
if (permissions.surveillance) modules.push(SurveillanceModule);

// 无权限的模块完全不加载
```

### 场景覆盖
- 远程桌面（控制他人 / 被控制）
- 视频会议（加入/主持）
- 推拉流（观看/管理）
- 监控（查看/回放）
- 遥操作（操控车辆/机器人）

---

## 二、Host — Headless 远端

### 场景
车端、机房、边缘设备、摄像头——仅产出媒体流，无需 GUI。

### 架构
```
┌─────────────────────────┐
│  omspbase-host         │
│  ┌───────────────────┐  │
│  │ 采集 (capture)     │  │
│  │ 编码 (encode)      │  │
│  │ 推流 (push)        │  │
│  └───────────────────┘  │
│  ┌───────────────────┐  │
│  │ Embedded Web      │  │
│  │ localhost 配置页   │  │
│  └───────────────────┘  │
└─────────────────────────┘
```

### 特点
- 单一二进制 `omspbase-host` (~25 MB)。Phase 1 capture → encode → push 同进程 (D102 修正: 3 进程→单进程)
- 无 GUI 依赖，适合嵌入式设备
- Embedded Web 配置页（localhost）
- 仅包含生产 SDK（采集、编码、推流）
