# Swiss Cheese 验证管线

> 从 CLAUDE.md 提取。验证 Agent 启动时按需加载。

## 七级验证 + Level 2.5

```
Level 1 编译检查      ← Hooks: Stop 事件（tsc --noEmit, cargo check）
Level 2 代码规则      ← Hooks: PostToolUse（硬编码颜色, console.log）+ PreToolUse（安全拦截）
Level 2.5 深度排查    ← @debugger（5 关：编译/启动/静态分析/逐页功能/手动验证）
Level 3 逻辑审查      ← @cross-checker（契约/Capability/CSS/事件名/invoke）
                        @reviewer（Rubric ≥ 0.7: 逻辑30% + 安全25% + UX25% + 可维护20%）
Level 4 运行时验证    ← @e2e-verifier 层1-5（启动/功能/持久化/API/异常）
Level 5 视觉验证      ← @e2e-verifier 层6（MCP Playwright 截图 + 设计对比）
Level 6 桌面交互      ← @e2e-verifier 层7（Tauri WebDriver: 窗口/标题栏/快捷键）
Level 7 人工审阅      ← @e2e-verifier 层8（交接报告 → 用户定夺）
```

## 层间关系

- L1-2 失败 → 阻断后续
- L2.5 发现问题 → 退回修复
- L3 失败 → 退回修复
- L4 失败 → 从 L2.5 重来
- L5-6 工具不可用 → 必须安装后执行（禁止降级跳过）
- L7 → 必须输出

## @debugger 定位（Level 2.5）

- 位于实现完成后、正式验收前
- 输出 Bug 列表 + 根因分析 → 退回给实现 Agent
- 修复完成后才进入 @cross-checker / @reviewer
- 与 @reviewer 的区别：@debugger 是「找问题」（主动探测），@reviewer 是「判通过」（标准验收）

## 验证 Agent 必要性（按级别）

| 级别 | 必须的验证 Agent |
|------|-----------------|
| S | @reviewer |
| M | @reviewer + @e2e-verifier |
| L/XL | @debugger + @cross-checker + @reviewer + @e2e-verifier |
| 设计专项 | @reviewer |

## 工具依赖

| 工具 | 配置位置 |
|------|---------|
| Hooks | `.claude/hooks.json`（内置） |
| MCP Playwright | `.claude/settings.json` |
| Tauri WebDriver | `.claude/templates/tauri-webdriver-setup.md` |

## 强制退出条件（所有验收阶段）

1. 运行时验证：`npm run tauri dev` 启动成功，无 panic/crash
2. 每页功能点击：操作每个 UI 控件并验证效果
3. 持久化循环：保存 → 关闭 → 重启 → 验证恢复
4. 契约 1:1 对齐：Rust struct 字段数 = TS interface 字段数
5. 跨角色 checklist：CSS 选择器、Tauri capability、invoke 参数名、事件名
