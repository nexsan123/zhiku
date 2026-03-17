# design/ 目录规则

> 管辖者：@ui-designer
> 其他 Agent 禁止修改本目录内容（@coder-fe 可读取作为实现参考）

## 文件结构

```
design/
  theme.ts                  # 全局 design token 系统（唯一真相源）
  glassmorphism-spec.md     # Glass 视觉规范
  window-config.md          # 窗口配置规范（@coder-be 读取）
  layouts/
    app-layout.md           # 全局布局规范
  pages/
    map-page.md             # 地图页设计规范
```

## Token 命名规范

- 语义命名：`bg.base` / `text.primary` / `accent.primary`
- 禁止非语义命名：`color1` / `blue-dark-2` / `teal-3`
- 所有颜色必须是具体 hex 或 rgba 值
- 间距遵循 4px 基数系统

## 变更流程

1. @ui-designer 修改 design/ 文件
2. 通知 @coder-fe 和 Team Lead
3. @coder-fe 同步更新 `src/styles/variables.css`

## 与 src/styles/variables.css 的关系

`theme.ts` 是唯一真相源。`variables.css` 由 @coder-fe 根据 `theme.ts` 生成/维护。
两者必须 1:1 对应，@cross-checker 在验证阶段检查同步性。
