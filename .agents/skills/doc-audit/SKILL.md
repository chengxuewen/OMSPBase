---
name: doc-audit
description: "AUDESYS 项目文档与架构审计。并行检查架构文档、设计文档、决策记录之间的自洽性、完整性、缺口和优化机会。以交互式问答与用户逐项确认每项发现，列出详情、方案与优劣、来源、影响、推荐。支持团队模式(大型审计)和背景代理模式(轻量检查)。"
---

# 文档架构审计 (Document & Architecture Audit)

对 AUDESYS 项目文档体系进行全面审计。

**哲学**: 审计不是找茬，是清债务。文档债务和代码债务一样危险。每次审计解决一批，文档体系往前一步。

---

## 入口：审计类型

### `/doc-audit`（无参数）
弹出审计类型菜单：

```
[1] 完整审计 — 全部 4 维度（默认）
[2] 决策验证 — decisions.md D1-Dx 落实情况
[3] 文档一致性 — arch ↔ hal ↔ 子文档交叉检查
[4] 参考验证 — reference/ §7 ↔ AUDESYS 设计
[5] 缺口优化 — 运维/安全/可靠性扫描
[6] 阶段审计 — SDD / TDD / 实施计划
```

### `/doc-audit full`
直接启动全量审计，跳过菜单。等效于 `[1] 完整审计`。

### `/doc-audit `<维度名>
启动指定维度。如 `decisions` / `consistency` / `references` / `gaps` / `phase`。

### `/doc-audit quick-fix`
仅检查 LOW/MEDIUM 级别问题并自动修复，不进入交互审核。

### `/doc-audit custom`
进入自定义模式：选择维度 + 文件范围 + 是否包含参考库。

---

## 审计维度

### 1. 决策验证 (Decision-Validator)
核查 `decisions.md` D1-Dx, G1-Gx 在 arch/hal-detailed/conventions/status 中的落实。

**核心问题**：
- 决策结论是否正确反映在文档中？
- 是否存在「决策说了A，文档写了B」的矛盾？
- 决策中的陈旧引用是否需要更新？
- **决策新鲜度检查**：
  - 该决策是否被后续决策明确替代？（如 D14→D44 逆转）
  - 决策中的技术栈版本是否过期？（Rust stable 版本、crate 版本）
  - 决策理由是否仍然成立？（如 "Phase 1 仅 amw_inproc" → Phase 2 是否需要更新？）

### 2. 文档一致性 (Consistency-Checker)
核查 arch ↔ hal-detailed ↔ 子文档交叉一致性。

**核心问题**：
- 同一概念描述是否一致？（技术栈、延迟值、状态标记）
- 子文档是否与主文档重复？是否应按 D14/D15 折叠为引用？
- Phase 术语是否歧义？（HAL-P1 vs Studio-P1 vs Language-P1，参考 arch.md 术语速查表）
- 命名空间/缩写一致性
- **CROSS-CHECK.md 对齐**：如果存在 `docs/CROSS-CHECK.md`，验证其声明的交叉引用是否与实际文档结构匹配

### 3. 参考验证 (Reference-Crosschecker)
核查 `docs/reference/` 中 41+ 篇参考文档的「对AUDESYS参考价值」→ AUDESYS 设计吸收情况。

**核心问题**：
- 多篇竞品一致推荐的功能/模式是否已吸收？
- 关键发现是否与当前设计矛盾？
- 已验证的模式可映射到 AUDESYS 架构？

### 4. 缺口优化 (Gap-Optimizer)
扫描缺失的关键设计章节。

**核心问题**：
- 运维/可观测性：健康检查、指标导出、日志聚合、告警路由
- 安全架构：IPC认证、mTLS、审计日志、X.509证书管理
- 错误模型：Signal写失败、StreamChannel溢出、传输断连、发现失败、类型不匹配、Config Barrier回滚
- 硬件基线：最低CPU/RAM/存储/内核版本/目标架构
- 资源限制：每模块CPU/内存/磁盘/网络预算
- 升级策略：热更新、状态迁移、配置迁移、回滚

### 5. 阶段审计 (Phase Audit — 新增)

#### 5a. SDD 文档审计
核查 SDD（规范驱动设计文档）是否覆盖 architecture.md + hal-detailed-design.md 的所有功能点。

**核心问题**：
- SDD 是否覆盖了所有 HAL 原语（Signal/StreamChannel/RPC）的测试场景？
- 边界条件是否枚举完整？
- 验收标准是否可测量、可验证？

#### 5b. TDD 测试审计
核查 TDD 测试计划是否覆盖 SDD 的全部验收标准。

**核心问题**：
- 测试用例是否与 SDD 验收标准一一对应？
- AAA 模式每项是否完整（Arrange/Act/Assert）？
- 覆盖率目标是否可达？

#### 5c. 实施计划审计
核查实施计划是否与架构文档对齐。

**核心问题**：
- 实施步骤是否按依赖顺序编排？
- Phase 定义是否与架构文档一致？
- 并行构建计划是否合理？

#### 5d. 孤立测试检测 (Orphan Test Detection)
核查测试文件是否已注册到构建系统。

**核心问题**：
- Rust: `crates/*/tests/` 目录下是否有未被 cargo test 自动发现的孤立测试文件？
- FlatBuffers: schema 变更后是否有对应的 round-trip 测试？
- CI: `scripts/qa/qa-fast.sh` 是否覆盖所有测试目标？

### 6. 代码→规范追溯 (Phase 1+ 预留)
当源代码存在时：核查 Rust/TS 代码实现是否与设计文档一致。

**核心问题**：
- 实现是否匹配 HAL 核心 trait 定义？
- 测试是否覆盖设计文档中的边界条件？
- 是否存在设计文档未覆盖的实现细节？

---

## 审计模式

### A. 团队模式（推荐 — 大型审计）
3+ 份大型文档 → `team_create` 4-6 个 `ultrabrain` 成员并行。

```
team_create(inline_spec={
  name: "doc-audit",
  members: [
    { name: "decision-validator", category: "ultrabrain", prompt: "<维度核心问题 + 文件列表 + 输出格式>" },
    { name: "consistency-checker", category: "ultrabrain", prompt: "..." },
    { name: "reference-crosschecker", category: "ultrabrain", prompt: "..." },
    { name: "gap-optimizer", category: "ultrabrain", prompt: "..." }
  ]
})
```

**Conductor 规范**（调度者行为）：
- 启动后立即向用户报告：「启动 N 路并行审计，预计 3-5 分钟」
- 等待全部完成前只做「非重叠工作」（如预读文档）
- 全部完成后：**去重合并**（同问题被 2+ 维度发现 → 合并为 1 项，标注多来源）
- 按严重性排序：CRITICAL → HIGH → MEDIUM → LOW
- 超时处理：任一路超过 10 分钟未产出 → 标注为「超时，部分结果」继续
- 冲突处理：维度 A 说 X、维度 B 说 Y → 标记为人类审核

### B. 背景代理模式（轻量审计）
少量文档 → `task(category="deep", run_in_background=true)` × N 并行。

### C. 单线程模式
极小范围 → 直接用 Read/Grep 检查，不启动子代理。

---

## 交互审核：发现项格式

**逐项审核**，每项使用 `question()` 工具展示。

```markdown
## 🔴/🟠/🟡/🔵 [编号]: [标题]

### 详情
| 来源1 | 位置 | 内容 |
|--------|------|------|
| 文档A | 行X | ... |
| 文档B | 行Y | ... |

### 来源
- 审计维度：[维度名]
- 原始发现：[报告] 第N项

### 可选方案
| 方案 | 优势 | 劣势 |
|------|------|------|
| A. [方案名] | ... | ... |
| B. [方案名] | ... | ... |

### 影响
- 选A：[连锁修改清单]

### 推荐
[方案X]。[理由]
```

选项：
- 采纳推荐方案 / 选择其他方案 / 不处理 / 自定义

进度：`[第N/共M项]`

---

## 工作流

### Phase 1: 启动
1. 确认审计范围和类型
2. 选择模式（团队/背景/单线程）
3. 报告：「启动 N 路并行审计，预计 3-5 分钟」

### Phase 2: 合并
1. 去重：同问题多来源 → 合并标注
2. 排序：CRITICAL → HIGH → MEDIUM → LOW
3. 交叉印证：2+ 维度同意的提升优先级

### Phase 3: 交互审核
逐项审核，question() 交互确认。

### Phase 4: 修复
1. 创建 todo list
2. 按依赖顺序：先改决策 → 再改架构 → 最后改状态
3. 每次编辑后验证（grep MODACS 残留、lsp_diagnostics）

### Phase 5: 报告
```
审计完成 — [日期]
审计类型: [全量/决策/一致性/参考/缺口/阶段]
发现总数: N | 已修复: M | 不处理: K
下次建议: [问题密集区域]
```

---

## 快速参考

### 审计命令
```
/doc-audit              → 选择类型
/doc-audit full         → 全量审计
/doc-audit quick-fix    → 自动修复 LOW/MEDIUM
/doc-audit phase sdd    → SDD 阶段审计
/doc-audit phase tdd    → TDD 测试审计
/doc-audit phase plan   → 实施计划审计
```

### 严重性标准
| 严重性 | 触发条件 | 阻断 Phase 1? |
|--------|---------|:---:|
| 🔴 CRITICAL | 文档矛盾导致实现路径错误 / 决策被推翻 / 核心 API 缺失 | ✅ |
| 🟠 HIGH | 陈旧引用 / 延迟不一致 / Phase 歧义 / 重复文档 | ⚠️ |
| 🟡 MEDIUM | 表述差异 / 示例冲突 / 缺失不阻断当前阶段 | ❌ |
| 🔵 LOW | 格式不一致 / 引用缺失 / 待确认标记 | ❌ |

### 审计建议频率
- 每次 D# 决策变更后：`/doc-audit decisions`
- Phase 转换前：`/doc-audit full`
- 每周开发期间：`/doc-audit full`
- 每次文档大改后：`/doc-audit consistency`
