# OMSPBase 产品参考文档

> Phase 0 — 架构定义阶段 | 2026-07-16
> 基于 [架构设计文档](../architecture.md) 与四份产品调研报告

---

## 目录

1. [产品能力概览](./01-product-capabilities.md)
2. [部署形态](./02-deployment-modes.md)
3. [客户端与 Host](./03-client-host.md)
4. [SDK 分层与 API 参考](./04-sdk-layers.md)
5. [权限认证参考](./05-auth-permissions.md)
6. [插件体系参考](./06-plugin-system.md)
7. [协议与通信参考](./07-protocols.md)
8. [管线模型参考](./08-pipeline-model.md)
9. [传输架构](./09-transport-architecture.md)
10. [信令架构](./10-signaling-architecture.md)
11. [NAPI 绑定](./11-napi-binding.md)
12. [录制与回放](./12-recording-playback.md)

---

## 概述

OMSPBase 是 AUDE 生态的多媒体基础设施，提供七大产品能力。采用微内核 + 插件架构，支持多种部署形态。

```
AUDESYS (工业控制) ──┐              ┌── AUDEBase (企业应用)
                     ├── OMSPBase ──┤
   引用 native crate │  多媒体核心   │ Docker 模块
```

- **独立部署**：完整后端，自带用户/权限系统
- **AUDEBase 模块**：Docker 容器，委托平台 RBAC/LDAP
- **AUDESYS 嵌入**：Rust crate 静态链接，仅远程桌面 + 遥操作
