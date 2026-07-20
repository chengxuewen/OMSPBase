# Upgrade & Migration — 升级与迁移

> 状态：Phase 3 前设计 | 关联决策：D118 (版本兼容), D-OPS-10 (Host 升级) | 创建依据：doc-audit H8

## 版本兼容矩阵 (D118)

```
Host/Server: MAJOR.minor.patch
  - MAJOR 号不同 = 不兼容，必须同步升级
  - minor/patch 可独立滚动

Remote (Client): MAJOR.minor.patch
  - 向后兼容前一个 MAJOR（即 v2 Remote 可连 v1 Server）
  - 当前 MAJOR 与 Server 一致时推荐升级
```

## 配置迁移

host.conf 使用 JSON Schema 加 `version` 字段：

```json
{
  "version": 2,
  "video": { "codec": "h265", "bitrate_kbps": 4000 }
}
```

- 启动时读取 `version`，执行迁移链：v1→v2→v3→...
- 每步迁移是纯函数 `fn migrate(config: Value) -> Value`
- serde `#[serde(default)]` 保证新增字段无需手动填充

## 二进制升级 (Host, D-OPS-10)

systemd service 控制：

```
1. systemctl stop omspbase-host
2. 替换 /usr/local/bin/omspbase-remote-host
3. systemctl start omspbase-host
```

Host 启动时检查 `version` 字段，自动执行 schema 迁移。Remote 通过 `omspbase-server` 推送更新包。

## 数据库迁移 (Server)

sqlx migrate 管理 SQLite schema：

```
migrations/
  20260701000001_init.sql
  20260715000002_add_room_config.sql
```

Server 启动时自动执行未应用的迁移。生产环境需先备份 `omspbase.db`。

## 功能开关迁移

新增功能通过 feature flag 控制灰度：

```toml
[features]
default = ["webrtc"]
webrtc = []
h265 = []        # Phase 2
srttransport = []# Phase 3
```

Phase 切换策略：flag 默认关闭 2 个 release → beta 默认开启 → stable 默认开启并移除 flag。

## 运行时状态迁移

### Graceful Drain
- SIGTERM → 停止接受新连接 → 等待现有连接自然结束 (max 30s) → 超时后强制关闭。

### WebRTC 连接
- Drain 期间保持活跃，通过 ICE consent freshness 检测对端存活。

### 自动回滚 (Phase 2)
- 新二进制启动后 60s 内健康检查失败 → systemd 回滚到旧版本。

> 详见 `.sisyphus/plans/consolidated-mvp/plan.md` Phase 3
