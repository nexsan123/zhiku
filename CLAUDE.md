# 智库 · 内核 v7.0

> 本项目遵循永乐大典体系
> 框架位置：F:\Claude Team\
> 详细流程参考：`.claude/references/team-flow.md`
> 体系错误追踪：`.team-logs/system-errors.md`

## 五条铁律（违反即退回，无条件执行）

> 从 97 个 session 的数据中提炼（v7.0，2026-03-18）。

1. **先看现场再动手（DB-01）** — 修 bug 先拿 runtime log，禁止只看代码猜原因。数据展示 bug 全链路排查：API → DB → 后端 → 前端
2. **完成必须附证据（V-01）** — 禁止模糊词，运行时验证才算完成
3. **两次失败必须停（DB-03）** — 停下追溯全链路，先说假设等确认再写代码
4. **执行前拆分阶段（S-01）** — 大任务拆阶段 → 定义退出条件 → 用户确认后执行
5. **子阶段要有证据（S-02）** — 证据不足返回重修，N 通过后才可进入 N+1

## 操作指南（建议遵循，小任务可灵活）

- **决策三问**：架构级决策回答 — 做了会怎样？不做会怎样？换种做法呢？
- **偏离纠正**：偏离立即停下，两次失败停下分析根因
- **检查验证**：搜索空结果 ≠ 不存在，必须二次确认
- **品质提醒**：Swiss Cheese 不可跳过，全权限 ≠ 跳过验证

## Agent 分级授权（v7.0）

| Agent | 授权模式 | 理由 |
|-------|---------|------|
| @ui-designer | **plan** | 必须先产出可视化结构图供用户确认 |
| @coder-be / @coder-fe | **auto** | scope 明确，编译 hook 兜底 |
| @cross-checker / @reviewer / @e2e-verifier / @debugger | **auto** | 验证/调查只读，plan 无意义 |

## Session 交接

每次 session 结束更新 `HANDOFF.md`，开始时读取恢复上下文。

---

## 项目核心

**全球金融情报中枢**：借鉴 World Monitor (koala73/worldmonitor) 的 UI 设计与产品模式，用 Tauri v2 + React 19 + Rust 重写金融板块。定位为情报辅助层，内置金融周期推理引擎，供应 QuantTerminal 量化策略调节因子。

## 当前圣旨

**edict-005-fiscal-balance-source-reform** (2026-03-11)，详见 `.team-logs/edict-005-fiscal-balance-source-reform.md`
> 父级：edict-002 (仍有效) → edict-003 (AI引擎) → edict-004 (世界局势推理) → edict-005 (国家资产负债表+源改革)

## 环境声明

```yaml
language: TypeScript (frontend) + Rust (backend/Tauri)
runtime: Node.js >= 18 + Rust stable
os: Windows (WSL2 Ubuntu)
shell: Bash (WSL) / PowerShell (Windows)
ai_tool: Claude Code (Agent Teams)
package_manager: npm
framework: Tauri v2
project_type: desktop-app
map_library: deck.gl + MapLibre GL
ai_engine: Groq (批量) → Ollama (兜底) + DeepSeek (深度推理) → Claude (交叉验证)
data_sources: RSS (55+中文) + FRED + Yahoo Finance + EIA + BIS + WTO + CoinGecko + mempool + alternative.me
quant_integration: REST localhost:9601 + WebSocket ws://localhost:9600 → F:\QuantTerminal
```

**AI 必须遵守此环境声明。** 生成的代码、路径、命令必须兼容上述环境。

## UI 架构（仿 World Monitor）

```
TitleBar:   智库 | FINANCE | Cmd+K | SOURCES | INTEL | 通知 | 窗口控制
Body:       左栏(320px,可收起) + 中心地图(flex,deck.gl) + 右栏(320px,可收起)
StatusBar:  API状态灯(Ollama/Groq/Claude/FRED/Yahoo/EIA...) | Ready | 时间
```

- 左右栏独立滚动，地图固定
- 面板毛玻璃背景 (glassmorphism-spec.md)，可折叠展开
- 暗色主题（已有 design/theme.ts + variables.css）

### 面板清单 (16个)

左栏: News Feed / AI Brief / FRED Indicators / BIS Rates / WTO Trade / Supply Chain
右栏: Market Radar / Indices / Forex / Oil & Energy / Crypto / BTC ETF / Fear & Greed / Gulf FDI
浮层: AI Deduction / Cmd+K Search

## 数据源分层

| 层级 | 数据源 | API Key |
|------|--------|---------|
| L1 必须 | FRED, RSS (55+中文), Yahoo Finance | FRED 需要 |
| L2 重要 | EIA, BIS, alternative.me (F&G) | EIA 需要 |
| L3 增强 | WTO, CoinGecko, mempool.space, 静态策划数据 | 不需要 |

## AI 引擎（丙方案 → 丁方案）

```
批量（高频）: Groq (Llama 3.1 8B, 14400 req/day 免费) → Ollama 14B 兜底
深度（低频）: DeepSeek (128K上下文, 低成本, 中文强) → Claude 兜底/交叉验证
```

> **DeepSeek 为主的理由**：128K 上下文可注入知识库，推理能力强，成本低，
> 中文训练数据与西方 RSS 源形成天然偏见对冲。Claude 保留为交叉验证角色。

### 金融周期推理引擎

Layer 1 原始数据 → Layer 2 Rust指标计算(6类) → Layer 3 DeepSeek推理 → Layer 4 结构化JSON → QuantTerminal

推理频率: 周期定位日频 / 转折预警6h / 情绪快照1h / P0事件即时

## 项目专属铁律

| # | 铁律 | 说明 |
|---|------|------|
| ZK-01 | AI 分析必须可追溯 | 每条 AI 分析/总结必须关联原始新闻源 URL |
| ZK-02 | 数据质量优先 | 向 QuantTerminal 推送的数据必须经过验证，禁止推送垃圾数据 |
| ZK-03 | 三引擎可切换 | Ollama / Groq / Claude 必须可切换，不锁死单一供应商 |
| ZK-04 | 纵切优先 | 金融板块打通全链路后再扩展，禁止铺太宽做半成品 |
| ZK-05 | 数据去重 | 新闻采集必须有去重和时效性校验机制 |
| ZK-06 | API 状态可视 | 所有外部 API/AI 引擎状态必须实时显示状态灯 |
| ZK-07 | 推理必有链 | AI 推理输出必须包含 confidence + reasoning_chain + source_urls |

## 团队成员

| Agent | 角色 | 管辖 |
|-------|------|------|
| @ui-designer | 前端 UI 设计师 | `design/` + `src/styles/variables.css` |
| @coder-be | Rust/Tauri 后端 | `src-tauri/` |
| @coder-fe | TypeScript 前端 + 联调 | `src/` |
| @cross-checker | 跨角色一致性检查 | 只读 |
| @reviewer | 独立审查 | 只读 |
| @e2e-verifier | 端到端运行时验证 | 只读 + 可运行命令 |
| @debugger | 项目质量猎手 | 只读 + 可运行命令 |

## 外部依赖

- **QuantTerminal**：`F:\QuantTerminal`，REST :9601 + WS :9600
- **Claude API**：需要 API Key（加密存储本地）
- **FRED API**：需要 API Key（免费注册）
- **EIA API**：需要 API Key（免费注册）
- **Groq API**：需要 API Key（免费注册，14400 req/day）
- **Ollama**：需用户本地安装（免费）
- **Yahoo Finance / BIS / WTO / CoinGecko / mempool / alternative.me**：免费，无需 Key

## 阶段路线

| 阶段 | 内容 | 状态 | 实况（2026-03-13 审计） |
|------|------|------|------------------------|
| Phase 1 | 项目骨架 | ✅ 完成 | Tauri v2 + React 19 + SQLite 6 表 |
| Phase 2 | 数据引擎（RSS+FRED+Yahoo+SQLite+SmartPollLoop） | ✅ 完成 | 16 源全通：RSS(55+)/Yahoo/FRED/EIA/BIS/IMF/CoinGecko/F&G/mempool；WTO idle(需 Key) |
| Phase 3 | AI 引擎（统一路由+周期推理+情景推演） | ✅ 完成 | ai_router 统一调度 → openai_compat(Groq/Ollama/Deepseek) + claude_client；summarizer/cycle_reasoner/deep_analyzer/scenario_engine 全链路验证 |
| Phase 4 | 前端面板（三栏布局+16面板+状态灯） | ✅ 完成 | 16 面板全部接真数据 + 实时事件刷新；StatusBar 14 服务状态灯 |
| Phase 5 | 地图+集成（deck.gl+图层+QuantTerminal API） | ✅ 完成 | deck.gl 3 层(信贷周期/标签/双边弧线)；QT REST :9601(9端点) + WS :9600(3事件) + market_context.db(WAL共享) 实跑验证通过 |

### 剩余运维项

| 项目 | 状态 | 说明 |
|------|------|------|
| Claude API Key | 未配置 | 深度推理链路不可用，仅 Groq 批量在跑 |
| 中文 RSS 源 (21个) | 待自建 RSSHub | 需云 VPS 或本地 Docker，皇上暂缓 |
| WTO API | idle | 需注册 API Key |
| policy_calendar.json | 静态 | 2026 年 12 事件，日后需更新机制 |
