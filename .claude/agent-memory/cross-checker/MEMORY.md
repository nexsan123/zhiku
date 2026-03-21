# cross-checker MEMORY — 智库项目

## 项目结构关键路径

- 后端命令：`src-tauri/src/commands/` (news.rs, macro_data.rs, api_status.rs, market_data.rs, ai.rs, credit_cycle.rs, game_map.rs, settings.rs, shell.rs)
- 后端模型：`src-tauri/src/models/` (news.rs, macro_data.rs, signal.rs, ai.rs, credit.rs, intelligence.rs)
- 后端服务（含对前端输出的 struct）：market_radar.rs, game_map.rs, scenario_engine.rs, daily_brief.rs, alert_engine.rs, trend_tracker.rs
- 前端 bridge：`src/services/tauri-bridge.ts` — 唯一 invoke/listen 入口
- 契约：`contracts/api-news.ts`, `contracts/app-types.ts`
- Store：`src/stores/app-store.ts`
- Capabilities：`src-tauri/capabilities/` (3 个文件：default.json, data-engine.json, window.json)

## 已确认的架构决策

- Tauri v2：snake_case command 名透传，不自动 camelCase 转换
- serde rename_all = "camelCase" 在所有对前端输出的 struct 上一致应用
- FredResponse / FredObservation 是内部反序列化 struct，不输出给前端，无需 rename_all
- market_radar.rs 的 RadarSignal / MarketRadar 在 services/market_radar.rs 中定义（不在 models/ 下）
- Capability：core:default 覆盖所有自定义 command，无需逐一注册
- 项目已有 i18n 系统（react-i18next，zh-CN + en），不是"无 i18n"
- 项目纯暗色主题，无 data-theme 动态切换，CSS 只用 :root

## 当前已知问题（2026-03-20 reverse 扫描）

### WR-001（P0，待修复）—— 事件名不匹配
- 后端 poll_loop.rs:717 emit `"five-layer-updated"`
- 前端 tauri-bridge.ts:1055 listen `"five-layer-reasoning-updated"`
- 后果：每6小时自动推理无法推送到 CycleReasoningPanel / ForwardLookPanel
- 修复：@coder-be 将 poll_loop.rs:717 改为 `app.emit("five-layer-reasoning-updated", ...)`

### WR-002（P1，待修复）—— CycleIndicators TS 接口缺 4 字段
- TS `CycleIndicators`（tauri-bridge.ts:502）：7 字段
- Rust `CycleIndicators`（models/ai.rs:140）：11 字段
- 缺失：`commodities: CommodityCycle`, `crypto: CryptoSignal`, `fiscal: FiscalSnapshot`, `energy: EnergyData`
- 修复：@coder-fe 在 tauri-bridge.ts 接口补充 4 个字段

### WR-003（P1，待修复）—— rsshub_base_url 不在 KNOWN_KEYS
- `DataSourcesTab` 写入 `rsshub_base_url` 成功，但 `get_settings` 只枚举 KNOWN_KEYS（7 项无此 key）
- 每次打开设置页 RSSHub URL 为空，无法回显
- 修复：@coder-be 在 settings.rs KNOWN_KEYS 添加 `"rsshub_base_url"`

## 历史已修复问题

### CC-001（已修复）
之前：RadarSignal TS 接口用 `verdict` 字段，Rust 用 `bullish: Option<bool>` + `detail`。
现在：tauri-bridge.ts `MarketRadarSignal` 已对齐为 `{ name, bullish: boolean|null, detail }` ✅

### CC-002 / CC-003（已修复）
之前：前端 listen `news-updated` / `market-updated` 但后端未 emit。
现在：poll_loop.rs:245 和 301/426 分别 emit 这两个事件 ✅

## Command 总数

共 42 个注册 Command。孤立（非内部专用，无前端 invoke）：
- `get_available_indicators`（Wave 4 预留）
- `analyze_company`（Wave 4 预留）
- `get_country_credit_detail`（地图点击预留）

内部专用（正常无 FE invoke）：`fetch_fred`, `fetch_market`, `update_api_status`

## 事件总线摘要

正常连接：news-updated, market-updated, macro-updated, ai-summary-completed, five-layer-reasoning-updated, deep-analysis-completed, scenario-updated, daily-brief-updated, alerts-triggered, api-status-changed

断裂（WR-001）：five-layer-updated (后端) vs five-layer-reasoning-updated (前端)

无消费（可接受）：poll-loop-ready, cycle-reasoning-updated（listenCycleUpdated 已定义但无面板调用）

## 检查注意事项

- 后端 emit 全搜：`grep -rn "\.emit(" src-tauri/src/`
- 新的 service-layer struct 也要检查 serde：game_map.rs / scenario_engine.rs / daily_brief.rs / alert_engine.rs / trend_tracker.rs
- `daily-brief` panelId 在 App.tsx 用 `as any` 规避类型检查，contracts/app-types.ts 未添加
- `listenCycleUpdated` 在 tauri-bridge 定义，但无任何组件调用（已被 listenFiveLayerUpdated 取代？待确认）
- `get_indicator_trend` bridge 函数已定义，但无面板消费，待确认是否有面板接入趋势图
