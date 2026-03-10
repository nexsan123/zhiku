# cross-checker MEMORY — 智库项目

## 项目结构关键路径

- 后端命令：`src-tauri/src/commands/` (news.rs, macro_data.rs, api_status.rs, market_data.rs)
- 后端模型：`src-tauri/src/models/` (news.rs, macro_data.rs, signal.rs)
- 后端服务：`src-tauri/src/services/` (poll_loop.rs emit 事件在这里)
- 前端 bridge：`src/services/tauri-bridge.ts` — 唯一 invoke/listen 入口
- 契约：`contracts/api-news.ts`, `contracts/app-types.ts`
- Store：`src/stores/app-store.ts`
- Capabilities：`src-tauri/capabilities/` (3 个文件)

## 已确认的架构决策

- Tauri v2：snake_case command 名透传，不自动 camelCase 转换
- serde rename_all = "camelCase" 在所有对前端输出的 struct 上一致应用
- FredResponse / FredObservation 是内部反序列化 struct，不输出给前端，无需 rename_all
- market_radar.rs 的 RadarSignal / MarketRadar 在 market_radar.rs 中定义（不在 models/ 下）
- Capability：core:default 覆盖所有自定义 command，无需逐一注册
- 项目无 i18n 系统，中文品牌名"智库"不适用 i18n 检查

## 已发现的历史问题（供下次检查参考）

### CC-001（P1，待修复）
RadarSignal 类型不一致：
- 后端（market_radar.rs）：`name`, `bullish: Option<bool>`, `detail`
- 前端（tauri-bridge.ts）：`name`, `verdict: 'bullish'|'bearish'|'neutral'`
- MarketRadar 后端还有 `bullishPct: f64`, `timestamp: String`，前端无

### CC-002 / CC-003（P1，待修复）
孤立 listen 事件：
- 前端 listen 了 `news-updated` 和 `market-updated`
- 后端 poll_loop.rs 只 emit `api-status-changed`
- 需要在 RSS 和 Yahoo 轮询成功后添加对应 emit

## 检查注意事项

- 后端 emit 全搜：grep `\.emit\(` src-tauri/src/
- market_radar.rs 的 struct 在 services/ 下，不在 models/ 下，检查 serde 时记得包含此文件
- get_market_data / get_market_radar / fetch_market 已注册但前端 TODO（注释），Wave 3 启用时重新检查类型对齐
- update_api_status 有后端实现但前端无 invoke（后端内部用），正常
