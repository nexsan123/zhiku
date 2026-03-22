# Session 交接文档

> 每次 session 结束时更新。下次 session 开始时读取此文件恢复上下文。

## 当前状态

- **分支**: master
- **最近 commit**: `037c51f` docs: add PROJECT-MAP.md full project panorama
- **GitHub**: https://github.com/nexsan123/zhiku (private)
- **Mac 运行中**: app 在 MacBook 上持续运行积累数据（2026-03-22 启动）

## 本次 Session 完成（2026-03-22）

### 1. 连线扫描 + 3 个断裂修复

| ID | 优先级 | 问题 | 修复 | commit |
|----|--------|------|------|--------|
| WR-001 | P0 | poll_loop 事件名 `five-layer-updated` 与前端不匹配 | → `five-layer-reasoning-updated` | `c31b07d` |
| WR-002 | P1 | tauri-bridge CycleIndicators 7→11 字段缺失 | 补 commodities/crypto/fiscal/energy | `c31b07d` |
| WR-003 | P1 | settings.rs KNOWN_KEYS 缺 rsshub_base_url | 添加 | `c31b07d` |

### 2. 新闻分类数据质量修复

- 新建 `reclassify_stale_news` Tauri command（轻量级，只发标题问分类）
- Python 脚本批量执行：1999 条处理，679 条从 market 修正到正确分类
- 分布改善：macro_policy +265, geopolitical +204, energy +149, crypto +53
- commit: `83ea4f8`

### 3. SVG Sparkline 趋势图

- 新建 Sparkline.tsx（纯 SVG，零依赖）+ TrendIndicator.tsx（封装数据获取）
- 集成到 FredPanel（fed_rate/cpi_yoy/gdp_growth）、FearGreedPanel（fear_greed）、MarketRadarPanel（vix）
- TrendTracker 启动即拍快照（4min warmup → immediate → 2h → 6h loop）
- commits: `d6726b3`, `219553f`

### 4. RSSHub 部署

- 阿里云轻量 VPS（美西硅谷，4核8GB）：`47.89.210.88:1200`
- 智库 settings.json 已配置 rsshub_base_url
- 验证通过：财新/一财/财联社/华尔街见闻 全部可抓

### 5. 项目全景图

- 新建 `PROJECT-MAP.md`（12章节 445行，@cross-checker 实扫产出）
- 覆盖：架构总览/数据源/DB/AI引擎/面板/Command/QT对接/知识库/事件总线/状态/路线
- GitHub 仓库创建并推送：`https://github.com/nexsan123/zhiku`

### 6. Mac 部署

- MacBook 克隆项目，安装依赖，编译通过
- 配置 API Key（Groq/DeepSeek/FRED/EIA）+ RSSHub URL
- app 持续运行中，积累数据

## 当前阶段

**Phase 6：实战运行**（2026-03-22 开始）
- Mac 挂机运行 1-2 周积累数据
- 目标：indicator_history 积累 120+ 数据点，消化 3259 条积压新闻，中文源入库
- 1-2 周后回来做数据质量审计 + reasoning_scorecard 回测

## 下一步（Phase 6 完成后）

1. **数据质量审计** — 检查 AI 推理准确率（reasoning_scorecard 回测）
2. **QT 实战验证** — QuantTerminal 消费 adjustment-factors，评估对策略的影响
3. **Company-Level Intelligence (XL)** — 详见 memory `project_next_phase.md`
4. **geopolitical_graph 增强** — KB-006 不对称边修复

## 未解决问题

- Claude API Key 未配置（深度推理交叉验证不可用）
- Ollama 未安装（本地 AI 兜底不可用）
- WTO API Key 未注册
- 2 个面板为静态展示（SupplyChainPanel / GulfFdiPanel）
- listenCycleUpdated 孤立定义待清理
