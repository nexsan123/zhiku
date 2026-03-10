# 智库 · 工作流状态

> Lead 在关键时刻更新此文件，context compaction 后以此为准

## 当前阶段：代码到位，待验证 + 补缺

### 圣旨
- **当前圣旨**：edict-002-finance-reshaping（2026-03-08 颁发）
- **前任圣旨**：edict-001-mvp（已被替代）

### 阶段进度（真实状态 2026-03-09）
| 阶段 | 内容 | 代码 | 运行验证 | 差距 |
|------|------|:----:|:--------:|------|
| Phase 1 | 项目骨架 | ✅ | ✅ | 无 |
| Phase 2 | 数据引擎 | ✅ | ❌ | RSS 仅 6 源（圣旨要求 55+），缺 BIS/WTO/mempool 客户端 |
| Phase 3 | AI 引擎 | ✅ | ❌ | 未配置 API Key，未运行验证 |
| Phase 4 | 前端面板 | ✅ | ⚠️ | 7/14 面板真功能，3 个假联通，4 个纯静态壳 |
| Phase 5 | 地图+集成 | ✅ | ⚠️ | 地图渲染+互动可用，QT 联调未做 |

### 皇上待验证清单（2026-03-09）
- [ ] 点击新闻标题 → 系统浏览器打开原文（若打开内嵌窗口则需换 plugin-shell 方案）
- [ ] 面板数据行 hover → 应有浅色背景高亮
- [ ] 面板标题栏点击 → 应可折叠/展开
- [ ] Ctrl+[ / Ctrl+] → 左右栏整体折叠
- [ ] Ctrl+K → 命令面板弹出
- [ ] 地图点击光圈 → MapDetailCard 浮层显示

### 未完成事项（按优先级）

#### P0 — 缺失功能（圣旨要求但代码不存在）
| # | 缺失项 | 圣旨要求 | 现状 |
|---|--------|---------|------|
| 1 | RSS 源不足 | 55+ 含中文 | 仅 6 个英文源（Reuters/BBC/CNBC/MarketWatch/NYT） |
| 2 | BIS 数据客户端 | L2 重要 | 无 bis_client.rs，前端 BisPanel 纯硬编码 |
| 3 | WTO 数据客户端 | L3 增强 | 无 wto_client.rs，前端 WtoPanel 纯硬编码 |
| 4 | mempool.space 客户端 | L3 增强 | 无 mempool_client.rs，market_radar signal5 标注 pending |
| 5 | AiBrief 前端假联通 | 后端有命令 | tauri-bridge 永远返回 mock 数据 |
| 6 | MarketRadar 前端假联通 | 后端有命令 | tauri-bridge 永远返回 mock 数据 |
| 7 | CycleReasoning 前端假联通 | 后端有命令 | tauri-bridge 永远返回 mock 数据 |

#### P1 — 未验证（代码到位但从未运行）
| # | 验收项（edict-002） | 方法 | 状态 |
|---|---------------------|------|------|
| 1 | 数据采集 | SELECT count(*) FROM news > 0 | ❌ 未验证 |
| 2 | 数据去重 | 重复插入同一 URL，count 不变 | ❌ 未验证 |
| 3 | AI 兜底 | 关闭 Ollama 后触发摘要 → 自动切 Groq | ❌ 未验证 |
| 4 | AI 深度 | 周期推理返回 cycle+turning_signals+confidence JSON | ❌ 未验证 |
| 5 | 前端布局 | 三栏 + 面板可折叠/滚动 + 地图居中 | ⚠️ 已启动待确认 |
| 6 | 状态灯 | 关闭 Ollama → 底栏指示灯变红 | ❌ 未验证 |
| 7 | 地图 | deck.gl 渲染 + ≥3 图层可切换 | ⚠️ 已启动待确认 |
| 8 | QT REST | curl localhost:9601/api/v1/cycle 返回 JSON | ❌ 未验证 |
| 9 | QT WS | WS 客户端连接 :9600 收到消息 | ❌ 未验证 |

#### P2 — 配置项
| # | 项目 | 状态 |
|---|------|------|
| 1 | FRED API Key | 未配置 |
| 2 | Groq API Key | 未配置 |
| 3 | Claude API Key | 未配置 |
| 4 | EIA API Key | 未配置 |
| 5 | Ollama 本地安装 | 未确认 |

#### P3 — 已知缺陷（非阻断）
- ai-summary-completed 事件：后端发射但前端无 listener（orphan）
- btc-etf PanelId：contracts 中声明但无面板实现（stale）
- SupplyChain 面板：纯静态硬编码，无数据源计划
- GulfFDI 面板：纯静态硬编码（圣旨说"静态策划"可接受）

### Git 提交记录
| Commit | 内容 |
|--------|------|
| `bfa7fae` | Phase 1 骨架 |
| `48bcd7f` | Phase 2-3-5 后端（数据引擎+AI引擎+QT集成） |
| `1d0a269` | Phase 4 前端（16面板+三栏布局+i18n） |
| `042d611` | Phase 5 前端（地图+MapDetailCard+Cmd+K） |
| `252beca` | 修复：面板交互（新闻可点击+hover高亮） |
