# 智库 · 内核 v6.0

> 本项目遵循永乐大典体系
> 框架位置：F:\Claude Team\

## 项目核心

**全球情报中枢**：AI 驱动的国际金融信息采集、分析与推演桌面平台，为 QuantTerminal 量化项目提供高质量情报输入。

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
map_library: TBD (Leaflet / MapBox / D3 — 待设计阶段决定)
ai_engine: Claude API + Local Model (Ollama)
data_sources: NewsAPI + GDELT + Official RSS + AI Scraping
quant_integration: REST API / WebSocket → F:\QuantTerminal
```

**AI 必须遵守此环境声明。** 生成的代码、路径、命令必须兼容上述环境。

## 项目专属铁律

| # | 铁律 | 说明 |
|---|------|------|
| ZK-01 | AI 分析必须可追溯 | 每条 AI 分析/总结必须关联原始新闻源 URL |
| ZK-02 | 数据质量优先 | 向 QuantTerminal 推送的数据必须经过验证，禁止推送垃圾数据 |
| ZK-03 | 双引擎可切换 | Claude API 和本地模型必须可切换，不锁死单一供应商 |
| ZK-04 | 纵切优先 | 一国打通全链路后再扩展，禁止铺太宽做半成品 |
| ZK-05 | 数据去重 | 新闻采集必须有去重和时效性校验机制 |

## MVP 范围（甲案：纵切法）

```
第一刀：美国金融数据全链路
  采集（Fed RSS + NewsAPI）→ SQLite 存储 → AI 总结/推演 → 地图上点亮美国 → API 推送 QuantTerminal

第二刀：扩展到 G7（待第一刀完成后规划）
第三刀：扩展到全球（待第二刀完成后规划）
```

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

- **QuantTerminal**：`F:\QuantTerminal`，通过本地 API 对接
- **Claude API**：需要 API Key（人工干预点 H-4）
- **NewsAPI**：需要 API Key（人工干预点 H-4）
- **本地模型**：Ollama（需用户本地安装）
