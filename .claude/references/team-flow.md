# Team Flow 详细流程参考

> 从 CLAUDE.md v7.0 迁出。Lead 和 Agent 按需加载。

## 完整流程（涉及前后端 + UI）

```
X.1a 设计提案 (@ui-designer) ──── 输出可视化结构图 + 方案给 Lead
X.1b 用户确认 (Lead → 用户)  ──── 逐页展示，用户拍板
X.1c 设计定稿 (@ui-designer) ──┐
                                ├── 可并行
X.2 后端实现 (@coder-be)     ──┘
                                │
X.3 前端实现 (@coder-fe)     ──── 依赖 X.1c
                                │
X.4 设计审查 (@ui-designer)  ──── 依赖 X.3
                                │
X.5 前后端联调 (@coder-fe)   ──── 依赖 X.2 + X.4
                                │
X.5.5 一致性检查 (@cross-checker) ── 依赖 X.5
                                │
X.6 独立验收 (@reviewer)     ──── 依赖 X.5.5
                                │
X.6.5 E2E 验证 (@e2e-verifier) ── 依赖 X.6
                                │
X.7 最终报告 (Lead)
```

| 子阶段 | 负责人 | 输入 | 输出 |
|--------|--------|------|------|
| X.1a | @ui-designer | 产品定位、页面需求 | 可视化结构图 + 设计提案报告 |
| X.1b | Lead → 用户 | 设计提案 | 用户反馈 |
| X.1c | @ui-designer | 用户确认的方案 | theme.ts、variables.css、布局/组件规范 |
| X.2 | @coder-be | 契约文件 | Rust 代码、后端集成状态报告 |
| X.3 | @coder-fe | 契约、设计规范 | 前端代码（Mock 模式） |
| X.4 | @ui-designer | 前端代码 | 设计审查报告（通过/退回） |
| X.5 | @coder-fe | 后端就绪 + 设计通过 | 联调报告 + Mock 清理 |
| X.5.5 | @cross-checker | 联调完成的代码 | 一致性检查报告 |
| X.6 | @reviewer | 全部报告 | 验收报告（Rubric 评分） |
| X.6.5 | @e2e-verifier | reviewer 通过 | E2E 验证报告 |

## 精简流程

### A：仅后端

`X.1 后端实现 → X.2 验收(@reviewer) → X.3 运行时验证(@e2e-verifier)`

### B：仅前端（无后端交互）

`X.1 设计规范 → X.2 前端实现 → X.3 设计审查 → X.4 验收(@reviewer) → X.5 运行时验证(@e2e-verifier)`

### C：仅设计系统

`X.1 设计规范 → X.2 验收(@reviewer)`

## 修改已有功能（变更管理）

| 类型 | 改什么 | 流程 |
|------|--------|------|
| A 仅前端逻辑 | 交互/状态 | @coder-fe → @reviewer |
| B 仅后端逻辑 | 内部实现 | @coder-be → @reviewer |
| C 设计变更 | 主题/配色 | @ui-designer → @coder-fe → @ui-designer 审查 → @reviewer |
| D 前后端联动 | bug/功能调整 | @coder-be → @coder-fe → 联调 → @reviewer |
| E 契约变更 | 加字段/改接口 | Lead 改契约 → BE 同步 → FE 同步 → 联调 → @reviewer |
| F 设计反馈 | FE 发现规范缺陷 | @coder-fe → Lead → @ui-designer 修正 → @coder-fe |
| G 后端反馈 | BE 发现契约不匹配 | @coder-be → Lead 评估 → 改契约走 E 或 BE 层适配 |

## Teammate 直接沟通

Teammate 之间用 SendMessage 直接沟通，不必经过 Lead。**规则：沟通可以自由，但任何决策变更必须同步给 Team Lead。**

## 退回 → 修复 → 重审循环

```
@reviewer 退回（每个问题标注 R-001, R-002...）
→ Lead 转发给对应 Teammate → 修复并引用 ID → 重审（scope：失败项 + 关联项）
→ 同一问题第 6 次退回 → Lead 介入分析根因
```

## 人工干预点

| ID | 干预点 | 必要性 | 触发条件 |
|----|--------|--------|---------|
| H-1 | 需求确认 | 必须 | Plan 阶段完成后 |
| H-2 | 设计选择 | 必须 | X.1a 设计提案完成后 |
| H-3 | 目标确认 | 可选 | Spec 完成后（复杂项目） |
| H-4 | 凭证配置 | 必须 | 需要 API Key 时 |
| H-5 | 报告审阅 | 轻量 | X.7 最终报告产出后 |

## 契约定义规范

位置：`contracts/api-[模块名].ts`。必须包含：请求类型、响应类型、错误码枚举。底部附 capability 清单。改契约 = 走类型 E 流程。

## 集成地图

文件：`.team-logs/integration-map.md`，每行一个连接点，五个状态列（设计/契约/后端/前端/联调），全 ✅ 才算完成。
