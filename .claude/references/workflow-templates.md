# 工作流模板

> 从 CLAUDE.md 提取。Lead 创建 TaskCreate 依赖链时按需加载。

## 完整流程（L/XL 级，前后端 + UI）

```
X.1a 设计提案 (@ui-designer)
X.1b 用户确认 (Lead → 用户)
X.1c 设计定稿 (@ui-designer) ──┐ 可并行
X.2  后端实现 (@coder-be)     ──┘
X.3  前端实现 (@coder-fe)     ← 依赖 X.1c
X.4  设计审查 (@ui-designer)  ← 依赖 X.3
X.5  联调 (@coder-fe)         ← 依赖 X.2 + X.4
X.5.3 深度排查 (@debugger)    ← 依赖 X.5
X.5.5 一致性检查 (@cross-checker) ← 依赖 X.5.3
X.6  独立验收 (@reviewer)     ← 依赖 X.5.5
X.6.5 E2E 验证 (@e2e-verifier) ← 依赖 X.6
X.7  最终报告 (Lead)
```

| 子阶段 | 负责人 | 输入 | 输出 |
|--------|--------|------|------|
| X.1a | @ui-designer | 产品定位、页面需求 | 设计提案报告 |
| X.1b | Lead → 用户 | 设计提案 | 用户反馈 |
| X.1c | @ui-designer | 用户确认的方案 | theme.ts、variables.css、规范 |
| X.2 | @coder-be | 契约文件 | Rust 代码、集成报告 |
| X.3 | @coder-fe | 契约、设计规范 | 前端代码（Mock） |
| X.4 | @ui-designer | 前端代码 | 设计审查报告 |
| X.5 | @coder-fe | 后端就绪 + 设计通过 | 联调报告 |
| X.5.3 | @debugger | 联调代码 | Bug 列表 + 根因 |
| X.5.5 | @cross-checker | 排查通过的代码 | 一致性报告 |
| X.6 | @reviewer | 全部报告 | 验收报告 |
| X.6.5 | @e2e-verifier | reviewer 通过 | E2E 报告 |

## M 级流程 A（仅后端）

`实现 → @reviewer → @e2e-verifier`

## M 级流程 B（仅前端）

`设计规范 → 实现 → 设计审查 → @reviewer → @e2e-verifier`

## S 级流程

`实现 → @reviewer`

## 设计专项

`设计规范 → @reviewer`

## 验证 Task 前置创建（铁律 VT-01）

```
Step 1: TaskCreate 验证 Tasks（按级别）
Step 2: TaskCreate 实现 Tasks
Step 3: TaskUpdate 验证 Tasks 的 blockedBy → 指向实现 Tasks
```

验证 Task 从项目开始就存在，不可能"忘了创建"。

## 人工干预点

| ID | 干预点 | 必要性 | 触发条件 |
|----|--------|--------|---------|
| H-1 | 需求确认 | 必须 | Plan 完成后 |
| H-2 | 设计选择 | 必须 | 设计提案完成后 |
| H-3 | 目标确认 | 可选 | Spec 完成后 |
| H-4 | 凭证配置 | 必须 | 需 API Key 时 |
| H-5 | 报告审阅 | 轻量 | 最终报告后 |
