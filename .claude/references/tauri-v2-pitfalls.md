# Tauri v2 踩坑铁律

> 从 coder-be.md 提取。@coder-be 和 @coder-fe 实现时按需加载。

| 坑点 | 错误写法 | 正确写法 | ID |
|------|---------|---------|-----|
| setup 闭包中 spawn | `tokio::spawn()` | `tauri::async_runtime::spawn()` | RT-001 |
| Store 持久化 | `store.set(key, val)` 就完 | `store.set()` 后**必须** `store.save()` | RT-010 |
| invoke 参数匹配 | 参数名随意 | 参数名必须与前端 `invoke('cmd', { paramName })` 的 key 一致 | RT-008 |
| Capability 注册 | 忘记注册 | 每个 command 必须在 `capabilities/*.json` 注册，否则前端静默失败 | RT-004 |
| SQLite migration | `ALTER TABLE ADD COLUMN x` | 必须幂等：`IF NOT EXISTS` 或捕获 duplicate | RT-002 |
| serde 字段数 | Rust 少一个字段 | Rust struct 字段数 = TS interface 字段数 | RT-009 |
| CSS 主题选择器 | CSS 用 `.dark-theme` App 用 `data-theme` | 选择器必须与 App 设置方式匹配 | RT-003 |
| invoke 展开 | `invoke('cmd', { ...request })` | `invoke('cmd', { request })` 键名要匹配 Rust 参数名 | RT-008 |
