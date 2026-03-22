# 智库 — 连线总册 (Wiring Registry)

> 由 @cross-checker 自动维护。Lead 审核，不手动编辑。
> 最后更新：2026-03-22 by @cross-checker (reverse 增量扫描 commits 83ea4f8/d6726b3/219553f)

---

**版本**：v3.1
**扫描范围**：`src-tauri/src/` + `src/` + `contracts/` + `src-tauri/capabilities/`
**发现问题**：0 项（本次增量扫描全部通过）+ WR-001~003 已于 2026-03-21 修复

---

## 一、Tauri Command 全景表

> `FE invoke` 列：YES = tauri-bridge.ts 存在对应 invoke 调用；NO = 已注册但前端无调用。

| # | Command 函数名 | 实现文件:行 | lib.rs 注册 | FE invoke | 说明 |
|---|---------------|------------|-------------|-----------|------|
| 1 | `get_news` | commands/news.rs:26 | YES | YES | NewsFeedPanel |
| 2 | `get_news_count` | commands/news.rs:43 | YES | YES | NewsFeedPanel |
| 3 | `fetch_rss` | commands/news.rs:56 | YES | YES | 手动刷新 |
| 4 | `get_news_heatmap` | commands/news.rs:71 | YES | YES | MapCenter |
| 5 | `get_macro_data` | commands/macro_data.rs:10 | YES | YES | FredPanel / BisPanel / WtoPanel / FearGreedPanel / CryptoPanel |
| 6 | `fetch_fred` | commands/macro_data.rs:34 | YES | **NO** | 后端内部触发 |
| 7 | `get_api_status` | commands/api_status.rs:11 | YES | YES | StatusBar |
| 8 | `update_api_status` | commands/api_status.rs:26 | YES | **NO** | 后端内部用，正常 |
| 9 | `get_market_data` | commands/market_data.rs:12 | YES | YES | IndicesPanel / ForexPanel / OilEnergyPanel / CryptoPanel |
| 10 | `fetch_market` | commands/market_data.rs:33 | YES | **NO** | 后端内部触发 |
| 11 | `get_market_radar` | commands/market_data.rs:42 | YES | YES | MarketRadarPanel |
| 12 | `summarize_pending_news` | commands/ai.rs:20 | YES | YES | 手动触发 AI 摘要 |
| 13 | `get_ai_brief` | commands/ai.rs:38 | YES | YES | AiBriefPanel + NewsFeedPanel |
| 14 | `get_cycle_indicators` | commands/ai.rs:124 | YES | YES | CycleReasoningPanel |
| 15 | `get_cycle_reasoning` | commands/ai.rs:138 | YES | YES | CycleReasoningPanel |
| 16 | `trigger_cycle_reasoning` | commands/ai.rs:153 | YES | YES | CycleReasoningPanel |
| 17 | `get_five_layer_reasoning` | commands/ai.rs:185 | YES | YES | CycleReasoningPanel / ForwardLookPanel |
| 18 | `trigger_five_layer_reasoning` | commands/ai.rs:200 | YES | YES | CycleReasoningPanel / ForwardLookPanel |
| 19 | `get_deep_analyses` | commands/ai.rs:259 | YES | YES | IntelBriefPanel |
| 20 | `get_daily_brief` | commands/ai.rs:275 | YES | YES | DailyBriefPanel |
| 21 | `get_alerts` | commands/ai.rs:290 | YES | YES | AlertToast |
| 22 | `get_indicator_trend` | commands/ai.rs:305 | YES | YES | TrendIndicator 组件消费（FredPanel/FearGreedPanel/MarketRadarPanel）|
| 23 | `get_available_indicators` | commands/ai.rs:319 | YES | **NO** | 无前端 invoke（孤立，Wave 4 预留）|
| 24 | `analyze_company` | commands/ai.rs:334 | YES | **NO** | 无前端 invoke（孤立，Wave 4 预留）|
| 25 | `reclassify_stale_news` | commands/ai.rs:339 | YES | **NO** | 管理型批量修复 command，无常驻面板调用（孤立，正常）|
| 25 | `open_url` | commands/shell.rs:2 | YES | YES | NewsDetailModal |
| 26 | `get_settings` | commands/settings.rs:79 | YES | YES | SettingsPage 所有 Tab |
| 27 | `set_setting` | commands/settings.rs:103 | YES | YES | ApiKeysTab / DataSourcesTab |
| 28 | `delete_setting` | commands/settings.rs:116 | YES | YES | SettingsPage |
| 29 | `test_connection` | commands/settings.rs:125 | YES | YES | ApiKeysTab |
| 30 | `list_ai_models` | commands/settings.rs:286 | YES | YES | AiModelsTab / StatusBar |
| 31 | `save_ai_model` | commands/settings.rs:302 | YES | YES | AiModelsTab |
| 32 | `remove_ai_model` | commands/settings.rs:327 | YES | YES | AiModelsTab |
| 33 | `test_ai_model` | commands/settings.rs:336 | YES | YES | AiModelsTab |
| 34 | `get_rss_sources` | commands/settings.rs:489 | YES | YES | DataSourcesTab |
| 35 | `get_credit_cycle_overview` | commands/credit_cycle.rs:9 | YES | YES | CreditCyclePanel |
| 36 | `get_dollar_tide` | commands/credit_cycle.rs:19 | YES | YES | CreditCyclePanel |
| 37 | `get_country_credit_detail` | commands/credit_cycle.rs:27 | YES | **NO** | 孤立，地图点击预留 |
| 38 | `get_policy_vectors` | commands/game_map.rs:8 | YES | YES | GameMapPanel |
| 39 | `get_bilateral_dynamics` | commands/game_map.rs:18 | YES | YES | GameMapPanel |
| 40 | `get_decision_calendar` | commands/game_map.rs:28 | YES | YES | GameMapPanel |
| 41 | `get_active_scenarios` | commands/game_map.rs:37 | YES | YES | GameMapPanel |
| 42 | `trigger_scenario_update` | commands/game_map.rs:47 | YES | YES | GameMapPanel |

**孤立后端 Command 汇总（无前端 invoke）**：
- 内部专用（正常）：`fetch_fred`, `fetch_market`, `update_api_status`
- 功能预留（Wave 4）：`get_available_indicators`, `analyze_company`, `get_country_credit_detail`
- 管理型工具（正常）：`reclassify_stale_news`（一次性批量修复工具，不需常驻面板入口）

---

## 二、事件总线（Backend Emit ↔ Frontend Listen）

| 事件名 | 后端 emit 位置 | 前端 listen 位置 | 状态 |
|--------|---------------|-----------------|------|
| `news-updated` | poll_loop.rs:245 | tauri-bridge.ts:375 → NewsFeedPanel | ✅ |
| `market-updated` | poll_loop.rs:301, 426 | tauri-bridge.ts:386 → 5 个面板 | ✅ |
| `macro-updated` | poll_loop.rs:335, 369, 768 | tauri-bridge.ts:397 → FredPanel/BisPanel | ✅ |
| `ai-summary-completed` | poll_loop.rs:264 | tauri-bridge.ts:674 → AiBriefPanel/NewsFeedPanel | ✅ |
| `cycle-reasoning-updated` | commands/ai.rs:176 | tauri-bridge.ts:573（已定义，无面板消费） | ⚠️ 部分连线 |
| `five-layer-reasoning-updated` | commands/ai.rs:250 + poll_loop.rs:717 | tauri-bridge.ts:1055 → CycleReasoningPanel/ForwardLookPanel | ✅（WR-001 已修复 commit c31b07d）|
| `deep-analysis-completed` | poll_loop.rs:614 | tauri-bridge.ts:1064 → IntelBriefPanel | ✅ |
| `scenario-updated` | poll_loop.rs:646 | tauri-bridge.ts:1073 → GameMapPanel | ✅ |
| `daily-brief-updated` | poll_loop.rs:869 | tauri-bridge.ts:1134 → DailyBriefPanel | ✅ |
| `alerts-triggered` | poll_loop.rs:903, 910 | tauri-bridge.ts:1163 → AlertToast | ✅ |
| `api-status-changed` | poll_loop.rs:1045 | tauri-bridge.ts:354 → App.tsx → store | ✅ |
| `poll-loop-ready` | poll_loop.rs:216 | 无（信号丢弃，无功能影响） | ⚠️ 可接受 |

**WR-001 已修复（commit c31b07d，2026-03-21）**：
poll_loop.rs:717 已改为 `app.emit("five-layer-reasoning-updated", &reasoning)`，与前端 listen 对齐。
后台定时 6h + 手动 trigger 两条路径现在均能推送到 CycleReasoningPanel / ForwardLookPanel。

---

## 三、Capability 注册验证

| Capability 文件 | 权限列表 | 覆盖范围 |
|----------------|---------|---------|
| `capabilities/default.json` | `core:default`, `core:event:default`, `shell:default` | 全部 45 个自定义 commands + 事件系统（含 reclassify_stale_news / get_indicator_trend / get_available_indicators）|
| `capabilities/data-engine.json` | `store:default`, `sql:default` | 持久化存储 + 数据库 |
| `capabilities/window.json` | start-dragging, minimize, maximize, close, set-focus | 窗口管理 |

**结论**：`core:default` 覆盖所有自定义 Command，Capability 注册 ✅ 无缺漏。

---

## 四、契约字段对齐检查

### 4.1 contracts/ 文件级别

| 契约文件 | TS 接口 | TS 字段数 | Rust struct | Rust 字段数 | 状态 |
|---------|---------|----------|-------------|------------|------|
| contracts/api-news.ts | `NewsItem` | 10 | models/news.rs `NewsItem` | 10 | ✅ |
| contracts/api-news.ts | `AiAnalysis` | 5 | 无（DB 内部） | — | ✅ 正常 |
| contracts/app-types.ts | `RssSource` | 5 | commands/settings.rs `RssSourceInfo` | 5 | ✅ |
| contracts/app-types.ts | `AiModelConfig` | 7 | commands/settings.rs `AiModelConfig` | 7 | ✅ |

### 4.2 tauri-bridge.ts 内部接口级别

| TS 接口 | TS 字段 | Rust struct | Rust 字段 | 状态 |
|--------|---------|-------------|-----------|------|
| `MacroDataItem` | 6 | `MacroData` | 6 | ✅ |
| `BackendApiStatus` | 7 | `ApiStatusResponse` | 7 | ✅ |
| `MarketDataItem` | 7 | `MarketSnap` | 7 | ✅ |
| `MarketRadarSignal` | 3 | `RadarSignal` | 3 | ✅（CC-001 已修复）|
| `MarketRadarData` | 4 | `MarketRadar` | 4 | ✅ |
| `AiBriefCategory` | 5 | `AiBriefItem` | 5 | ✅ |
| `CycleIndicators` | 11 | `CycleIndicators` | 11 | ✅（WR-002 已修复 commit c31b07d）|
| `CycleReasoning` | 9 | `CycleReasoning` | 9 | ✅ |
| `TurningSignal` | 3 | `TurningSignal` | 3 | ✅ |
| `FiveLayerReasoning` | 18 | `FiveLayerReasoning` | 18 | ✅ |
| `ReasoningStep` | 5 | `ReasoningStep` | 5 | ✅ |
| `LayerSummary` | 4 | `LayerSummary` | 4 | ✅ |
| `ForwardLook` | 7 | `ForwardLook` | 7 | ✅ |
| `CountryCreditData` | 14 | `CountryCreditData` | 14 | ✅ |
| `CountryCyclePosition` | 11 | `CountryCyclePosition` | 11 | ✅ |
| `DollarTide` | 8 | `DollarTide` | 8 | ✅ |
| `GlobalCycleOverview` | 12 | `GlobalCycleOverview` | 12 | ✅ |
| `TierSummary` | 5 | `TierSummary` | 5 | ✅ |
| `RiskAlert` | 4 | `RiskAlert` | 4 | ✅ |
| `DeepMotiveAnalysis` | 5 | `DeepMotiveAnalysis` | 5 | ✅ |
| `LayerImpact` | 5 | `LayerImpact` | 5 | ✅ |
| `DeepAnalysis` | 10 | `DeepAnalysis` | 10 | ✅ |
| `PolicyVector` | 8 | `PolicyVector` | 8 | ✅ |
| `BilateralDynamic` | 7 | `BilateralDynamic` | 7 | ✅ |
| `CalendarEvent` | 7 | `CalendarEvent` | 7 | ✅ |
| `Scenario` | 10 | `Scenario` | 10 | ✅ |
| `ScenarioMatrix` | 3 | `ScenarioMatrix` | 3 | ✅ |
| `NewsHeatmapEntry` | 5 | `NewsHeatmapEntry` | 5 | ✅ |
| `DailyBrief` | 8 | `DailyBrief` | 8 | ✅ |
| `AttentionItem` | 4 | `AttentionItem` | 4 | ✅ |
| `QtSuggestion` | 5 | `QtSuggestion` | 5 | ✅ |
| `DataSnapshot` | 10 | `DataSnapshot` | 10 | ✅ |
| `SectorAdjustment` | 3 | `SectorAdjustment` | 3 | ✅ |
| `Alert` | 8 | `Alert` | 8 | ✅ |
| `TrendPoint` | 3 | `TrendPoint` | 3 | ✅ |

**WR-002 已修复（commit c31b07d，2026-03-21）**：

tauri-bridge.ts `CycleIndicators` 接口已补充全部 11 字段，与 models/ai.rs:140 完全对齐：
`monetary, credit, economic, market, sentiment, geopolitical, commodities, crypto, fiscal, energy, calculatedAt`

---

## 五、Store Key 对齐检查

| Key | 前端写入（setSetting） | 后端 KNOWN_KEYS | 前端读取（getSettings） | 状态 |
|-----|-------------------|-----------------|-----------------------|------|
| `fred_api_key` | ApiKeysTab | YES | 正常返回 | ✅ |
| `eia_api_key` | ApiKeysTab | YES | 正常返回 | ✅ |
| `wto_api_key` | ApiKeysTab | YES | 正常返回 | ✅ |
| `groq_api_key` | (AiModelsTab 间接) | YES | 正常返回 | ✅ |
| `claude_api_key` | (AiModelsTab 间接) | YES | 正常返回 | ✅ |
| `ollama_base_url` | — | YES | 正常返回 | ✅ |
| `disabled_rss_urls` | — | YES | 正常返回 | ✅ |
| `rsshub_base_url` | DataSourcesTab:setSetting | YES（settings.rs:50 已添加）| 正常返回 | ✅（WR-003 已修复 commit c31b07d）|
| `ai_models` | save_ai_model 直接写 | 不在 KNOWN_KEYS | list_ai_models 命令读 | ✅ 专用命令，正常 |

**WR-003 已修复（commit c31b07d，2026-03-21）**：
settings.rs `KNOWN_KEYS` 已添加 `"rsshub_base_url"`，getSettings() 现在可正常返回该值。

---

## 六、Invoke 参数格式检查

全量扫描 tauri-bridge.ts 所有 invoke 调用（36 处）：

- 无参数 invoke：`get_news`, `get_news_count`, `fetch_rss`, `get_macro_data`, `get_market_data`, `get_market_radar`, `get_api_status`, `get_ai_brief`, `get_cycle_indicators`, `get_cycle_reasoning`, `trigger_cycle_reasoning`, `get_settings`, `summarize_pending_news`, `get_credit_cycle_overview`, `get_dollar_tide`, `get_policy_vectors`, `get_bilateral_dynamics`, `get_active_scenarios`, `trigger_scenario_update`, `get_five_layer_reasoning`, `trigger_five_layer_reasoning`, `get_daily_brief`, `get_alerts`, `list_ai_models`, `get_rss_sources` — 均正确 ✅
- 有参数 invoke：`set_setting { key, value }`, `delete_setting { key }`, `save_ai_model { model }`, `remove_ai_model { id }`, `get_news_heatmap { hours }`, `get_deep_analyses { limit }`, `get_decision_calendar { days }`, `get_indicator_trend { indicator, days }`, `test_connection { service, apiKey }`, `test_ai_model { modelId }`, `open_url { url }` — 全部使用 `{ paramName }` 包裹 ✅

**展开写法 `invoke(...{...spread})`**：全量扫描未发现 ✅。

---

## 七、CSS 主题选择器一致性

- 项目为纯暗色主题（dark only），使用 `:root` CSS Custom Properties（variables.css:7）
- App.tsx 无动态 `data-theme` 属性切换
- 无 `.dark-theme` / `.light-theme` 选择器
- 结论：选择器一致性 ✅，无 RT-003 类型风险

---

## 八、serde rename_all 检查

所有对前端输出的 Rust struct 均有 `#[serde(rename_all = "camelCase")]`：

- models/ 下全部 struct：✅（AiBriefItem, MonetaryCycle, CreditCycle, EconomicCycle, MarketCycle, SentimentCycle, GeopoliticalRisk, CommodityCycle, CryptoSignal, FiscalSnapshot, EnergyData, CycleIndicators, TurningSignal, CycleReasoning, ReasoningStep, LayerSummary, ForwardLook, FiveLayerReasoning, CountryCreditData, CountryCyclePosition, DollarTide, GlobalCycleOverview, TierSummary, RiskAlert, NewsCluster, DeepMotiveAnalysis, LayerImpact, DeepAnalysis, MacroData, NewsRow, NewsItem, ApiStatus, ApiStatusResponse, NewsHeatmapEntry, Signal, AiAnalysisRow, MarketSnap）
- services/ 下返回前端的 struct：✅（RadarSignal, MarketRadar, PolicyVector, BilateralDynamic, CalendarEvent, Scenario, AssetImpact, ScenarioMatrix, DailyBrief, AttentionItem, QtSuggestion, SectorAdjustment, DataSnapshot, Alert, TrendPoint）
- 内部 enum（camelCase 不适用）：snake_case 正确（CreditCyclePhase, CountryTier, TideState, ConfidenceGrade）
- 内部反序列化 struct（不输出前端）：FredResponse, FredObservation — 无 rename，✅ 符合架构决策

**结论**：serde rename_all 全部合规 ✅。

---

## 九、i18n 覆盖率检查

项目已接入 react-i18next，支持 zh-CN 和 en 两种语言（src/i18n.ts）。

- 确认使用 `useTranslation` 的面板：AiBriefPanel, BisPanel, FredPanel, FearGreedPanel, SupplyChainPanel, GulfFdiPanel, SituationCenterPanel, AiModelsTab, 其他 settings tabs
- 项目有 i18n 系统，非旧版"无 i18n 系统"状态（本次扫描更新了记忆中旧的记录）
- 中文品牌名"智库"出现在静态内容中，不适用 i18n 检查
- 本次未做逐 key 深度覆盖率审计，如需可单独执行

---

## 十、面板全景图（Level 1）

```
左栏 (5 面板)
├── SituationCenter [5 subtabs]
│   ├── cycle tab → CycleReasoningPanel [get_cycle_indicators, get_five_layer_reasoning, trigger_five_layer_reasoning]
│   ├── credit tab → CreditCyclePanel [get_credit_cycle_overview, get_dollar_tide]
│   ├── intel tab → IntelBriefPanel [get_deep_analyses]
│   ├── gameMap tab → GameMapPanel [get_policy_vectors, get_bilateral_dynamics, get_decision_calendar, get_active_scenarios, trigger_scenario_update]
│   └── forward tab → ForwardLookPanel [get_five_layer_reasoning]
├── NewsFeed [get_news, get_ai_brief, listen:news-updated, listen:ai-summary-completed]
├── FredPanel [get_macro_data, listen:macro-updated]
├── BisPanel [get_macro_data, listen:macro-updated]
└── DailyBrief [get_daily_brief, listen:daily-brief-updated]

右栏 (9 面板)
├── MarketRadar [get_market_radar, listen:market-updated]
├── Indices [get_market_data, listen:market-updated]
├── Forex [get_market_data, listen:market-updated]
├── OilEnergy [get_market_data, listen:market-updated]
├── Crypto [get_market_data, get_macro_data, listen:market-updated]
├── FearGreed [get_macro_data]
├── WtoPanel [get_macro_data]
├── SupplyChain [静态数据，待替换]
└── GulfFDI [静态数据，待替换]

浮层
├── AlertToast [listen:alerts-triggered]
├── CmdK Search
├── SettingsPage [get_settings, set_setting, delete_setting, test_connection, get_rss_sources, list_ai_models, save_ai_model, remove_ai_model, test_ai_model]
└── NewsDetailModal [open_url]

StatusBar [get_api_status, list_ai_models, listen:api-status-changed]
```

---

## 十一、横切面 A — 数据实体生命周期

### news_items 链路

```
RSS 源 → rss_fetcher.rs (poll_loop 30m) → DB: news 表
→ summarizer.rs → ai_analysis 表 (analysis_type='news_summary')
→ emit("news-updated") / emit("ai-summary-completed")
→ get_news → NewsRow → NewsItem → tauri-bridge → NewsFeedPanel
```

### five_layer_reasoning 链路（含断裂点）

```
手动触发 trigger_five_layer_reasoning
  → commands/ai.rs:250 → emit("five-layer-reasoning-updated") → 前端更新 ✅

轮询触发 poll_loop FiveLayerReasoning 任务 (每6小时)
  → poll_loop.rs:717 → emit("five-layer-updated") → 前端无 listen → 断裂 🔴
```

### credit_cycle 链路

```
BIS SDMX API / IMF API → bis_client.rs / imf_client.rs (poll_loop)
→ DB: macro_data 表
→ credit_cycle_engine.rs → global_aggregator.rs
→ get_credit_cycle_overview → GlobalCycleOverview → CreditCyclePanel

get_dollar_tide → dollar_tide.rs → DollarTide → CreditCyclePanel
```

### settings/store 链路

```
用户输入 API Key → ApiKeysTab → setSetting(keyId, val)
→ set_setting → settings.json store
→ 后端 ai_router / fred_client 读取 read_store_key()

用户设置 RSSHub URL → DataSourcesTab → setSetting('rsshub_base_url', val) ✅ 写入成功
→ getSettings() 读取 → 'rsshub_base_url' 在 KNOWN_KEYS → 正常返回 ✅（WR-003 已修复）
```

---

## 十二、横切面 B — 系统级故障场景

| 故障场景 | 影响范围 | 降级策略 | 现状 |
|---------|---------|---------|------|
| AI 引擎全部离线 | 无新摘要/推理 | cycle_reasoner 返回 default，daily_brief 用规则引擎 | ✅ 有降级 |
| FRED API 失败 | FredPanel 旧数据 | poll_loop 继续，DB 有历史 | ✅ |
| Yahoo Finance 失败 | 市场数据面板旧数据 | 同上 | ✅ |
| SQLite 连接失败 | 应用启动失败 | 无降级，直接 panic | ⚠️ |
| QuantTerminal 离线 | QT 推送失败 | 不影响主 app | ✅ 隔离 |
| `five-layer-updated` 事件断裂 | CycleReasoningPanel/ForwardLookPanel 不自动刷新 | 用户手动 trigger 仍可用 | ✅ WR-001 已修复 |
| `rsshub_base_url` 无法回显 | DataSourcesTab 每次打开为空 | 用户重新输入 | ✅ WR-003 已修复 |

---

## 十三、行动清单

### P0 — 阻塞（立即修复）

无（本版本无 P0 问题）

> 历史 P0：WR-001 已修复 commit c31b07d（2026-03-21）

### P1 — 重要（本 Sprint 修复）

无（本版本无 P1 问题）

> 历史 P1：WR-002 / WR-003 已修复 commit c31b07d（2026-03-21）

### P2 — 待关注

| ID | 问题 | 描述 | 退回 |
|----|------|------|------|
| WR-004 | 孤立 Command：get_available_indicators | 已注册，Bridge 无 invoke | @coder-fe：Wave 4 启用时补充 bridge 函数 |
| WR-005 | 孤立 Command：analyze_company | 已注册，Bridge 无 invoke | 同上 |
| WR-006 | panelId 'daily-brief' 不在 PanelId 类型 | App.tsx 用 `as any` 规避 | @coder-fe：contracts/app-types.ts PanelId 添加 `'daily-brief'` |
| WR-007 | listenCycleUpdated 已定义但无组件调用 | tauri-bridge.ts:573 | 确认是否已被 listenFiveLayerUpdated 取代，如是标记废弃 |
| WR-008 | 静态数据面板未接真实数据 | SupplyChainPanel, GulfFdiPanel | 按产品路线图安排 Wave N |
| WR-009 | 孤立 Command：reclassify_stale_news | 已注册，无前端 invoke 入口 | 管理工具，正常；如需 UI 入口可在 SettingsPage 添加触发按钮 |
| TP-001 | TrendIndicator 初次启动前 4min 内无数据 | 4min warmup sleep 前 sparkline 为空骨架 | @coder-fe：可在空状态显示 "--" 文本（优雅降级，不崩溃）|

---

## 十四、自省

### v3.0（2026-03-20）
- 全量 reverse 模式：42 Command、13 事件名、35 个 TS/Rust struct 字段对比、3 个 Capability 文件。
- 发现 WR-001/002/003（P0+P1），均于次日 commit c31b07d 修复。

### v3.1（2026-03-22）
- 增量扫描（3 commits：reclassify_stale_news + Sparkline/TrendIndicator + poll_loop startup snapshot）。
- 本次 12 项检查全部通过，无阻塞项。
- TP-001（初次启动 sparkline 空数据）已通过 219553f 修复（4min wait + initial snapshot + 2h early snapshot）。
- reclassify_stale_news 登记为管理型孤立 command（正常，非断裂）。
- Command 总数从 42 → 45（+reclassify_stale_news +get_indicator_trend +get_available_indicators 已在 v3.0 后新增）。
- 后续关注：WR-006（panelId 'daily-brief' as any）和 WR-007（listenCycleUpdated 废弃确认）建议下一 session 处理。
