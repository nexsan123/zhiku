# 智库 · 工作流状态

> Lead 在关键时刻更新此文件，context compaction 后以此为准

## 当前阶段：edict-003 阶段 A 执行中

### 圣旨
- **当前圣旨**：edict-003-ai-engine-phase-a（2026-03-10 颁发）
- **父圣旨**：edict-002-finance-reshaping（仍有效）
- **前任圣旨**：edict-001-mvp（已被替代）

### edict-003 阶段 A 进度
| 任务 | 内容 | 状态 |
|------|------|------|
| 任务 1 | 设置页面（数据源管理 + AI 模型管理 + API Key） | 🔄 开始 |
| 任务 2 | 第一条 AI 链路（Groq 新闻摘要跑通） | 🔄 开始 |

### 阶段进度（真实状态 2026-03-09 更新）
| 阶段 | 内容 | 代码 | 运行验证 | 差距 |
|------|------|:----:|:--------:|------|
| Phase 1 | 项目骨架 | ✅ | ✅ | 无 |
| Phase 2 | 数据引擎 | ✅ | ❌ | RSS 57源已到位，BIS/WTO/mempool 客户端已创建，未运行验证 |
| Phase 3 | AI 引擎 | ✅ | ❌ | 未配置 API Key，未运行验证 |
| Phase 4 | 前端面板 | ✅ | ❌ | 10/14 面板真功能（+BIS+WTO+BTC Network），3 个已联通后端，1 个纯静态（SupplyChain） |
| Phase 5 | 地图+集成 | ✅ | ⚠️ | 地图渲染+互动可用，新闻点击已修复(shell open)，QT 联调未做 |

### 本轮修复记录（2026-03-09）
| Commit | 内容 |
|--------|------|
| `252beca` | 面板交互：新闻可点击 + hover 高亮 |
| `9964a41` | 新闻点击修复：plugin-shell open + capability |
| `5123752` | 新增 BIS/WTO/mempool 后端客户端 + poll_loop 12 tasks |
| `29c53b4` | RSS 源扩充 6→57（33英+24中） |
| `695f729` | BIS/WTO/Crypto 面板接通真实 macro_data |

### 皇上待验证清单
- [ ] 点击新闻标题 → 系统浏览器打开原文（已改用 plugin-shell，需重新验证）
- [ ] 面板数据行 hover → 应有浅色背景高亮
- [ ] 面板标题栏点击 → 应可折叠/展开
- [ ] Ctrl+[ / Ctrl+] → 左右栏整体折叠
- [ ] Ctrl+K → 命令面板弹出
- [ ] 地图点击光圈 → MapDetailCard 浮层显示
- [ ] BisPanel → 显示静态参考值（后端 BIS 数据尚未写入时）
- [ ] CryptoPanel → BTC Network 小节是否显示

### 剩余未完成事项

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
| 5 | WTO API Key | 未配置（WtoPanel 会显示提示） |
| 6 | Ollama 本地安装 | 未确认 |

#### P3 — 已知缺陷（非阻断）
- SupplyChain 面板：纯静态硬编码，无数据源
- GulfFDI 面板：纯静态硬编码（圣旨说"静态策划"可接受）
- ai-summary-completed 事件：后端发射但前端无 listener（orphan）
- btc-etf PanelId：contracts 中声明但无面板实现（stale）
- BIS 方向推断：inferDir() 始终返回 hold（需后端提供连续两期数据）
- BTC_HASHRATE 单位：前端假设 EH/s，需确认后端存储单位
- RSS 中 16 个源标注 TODO 需验证可用性

### Git 提交记录（全部）
| Commit | 内容 |
|--------|------|
| `bfa7fae` | Phase 1 骨架 |
| `48bcd7f` | Phase 2-3-5 后端 |
| `1d0a269` | Phase 4 前端 |
| `042d611` | Phase 5 前端（地图+Cmd+K） |
| `252beca` | 面板交互修复 |
| `70cd339` | workflow-state 真实状态更新 |
| `9964a41` | 新闻点击 plugin-shell 修复 |
| `5123752` | BIS/WTO/mempool 后端客户端 |
| `29c53b4` | RSS 57 源 |
| `695f729` | BIS/WTO/Crypto 面板接通 |
