# 变更管理

> 从 CLAUDE.md 提取。修改已有功能时按需加载。

## 变更类型

| 类型 | 改什么 | 流程 |
|------|--------|------|
| A 仅前端逻辑 | 交互/状态 | @coder-fe → @reviewer |
| B 仅后端逻辑 | 内部实现 | @coder-be → @reviewer |
| C 设计变更 | 主题/配色 | @ui-designer → @coder-fe → 设计审查 → @reviewer |
| D 前后端联动 | bug/功能 | @coder-be → @coder-fe → 联调 → @reviewer |
| E 契约变更 | 加字段/改接口 | Lead 改契约 → BE → FE → 联调 → @reviewer |
| F 设计反馈 | FE 发现规范缺陷 | @coder-fe → Lead → @ui-designer → @coder-fe |
| G 后端反馈 | BE 发现契约不匹配 | @coder-be → Lead → 改契约走 E 或适配 |

## 退回 → 修复 → 重审（铁律 RF-01）

```
1. 退回（标注 R-001, R-002...）
   必须包含：问题描述 + 根因分析 + 影响范围 + 建议修复方向

2. Lead 转发给对应 Teammate（要求回应根因分析）

3. Teammate 修复报告必须包含：
   - 引用退回 ID
   - 认同或反驳根因分析（附理由）
   - 修复内容
   - 验证证据
   - 关联影响检查

4. 重审（scope：失败项 + 关联项 + 同类检查）
```

## 退回升级机制

- 退回 ≥2 次（M 级）→ 引入 @debugger（铁律 TG-03）
- 退回 ≥3 次（L/XL）→ Lead 分析是否架构/契约层问题
- 退回 ≥5 次 → 暂停，Lead + 用户评估方向
