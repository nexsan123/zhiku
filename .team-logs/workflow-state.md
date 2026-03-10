# 智库 · 工作流状态

> Lead 在关键时刻更新此文件，context compaction 后以此为准

## 当前阶段：Phase 5 完成 — 全阶段代码到位

### 圣旨
- **当前圣旨**：edict-002-finance-reshaping（2026-03-08 颁发）
- **前任圣旨**：edict-001-mvp（已被替代）

### 阶段进度
| 阶段 | 内容 | 状态 |
|------|------|------|
| Phase 1 | 项目骨架（Tauri v2 + React 19 + 暗色主题） | ✅ 完成 |
| Phase 2 | 数据引擎（RSS+FRED+Yahoo+EIA+F&G+CoinGecko+SmartPollLoop） | ✅ 完成 |
| Phase 3 | AI 引擎（Ollama→Groq兜底+Claude深度+周期推理+6h定时） | ✅ 完成 |
| Phase 4 | 前端面板（三栏布局+15面板+状态灯+i18n） | ✅ 完成 |
| Phase 5 | 地图+集成（deck.gl+3图层+QT REST :9601+QT WS :9600+Cmd+K） | ✅ 完成 |

### Phase 5 交付物
- ✅ deck.gl + MapLibre GL 暗色地图（CartoDB dark-matter 免费瓦片）
- ✅ 3 图层：金融中心(19) + 央行(13) + 海湾FDI(6)，可切换
- ✅ 悬浮提示（glassmorphism tooltip）
- ✅ REST API 服务器 :9601（5端点：signals/macro-score/market-radar/ai-brief/cycle）
- ✅ WebSocket 服务器 :9600（broadcast: signal.new/cycle.update）
- ✅ market_context.db 共享 SQLite（60s 写入，QT 3s mtime 轮询）
- ✅ Cmd+K 命令面板（29项：14面板+8国家+7数据源）
- ✅ StatusBar 新增 qt_rest + qt_ws 状态灯

### 跨层一致性检查结果
- ✅ 事件名对齐（4/4 事件名前后端一致）
- ✅ CSP 放行 CartoDB 瓦片 CDN
- ✅ 端口一致（REST=9601, WS=9600）
- ✅ i18n 全覆盖（map + cmdK keys）
- ✅ AiBriefCategory 字段对齐修复（CC-002 已修复）
- ⚠️ ai-summary-completed 事件无前端 listener（orphan, 非阻断）
- ⚠️ btc-etf PanelId 声明但未实现（stale, 非阻断）

### 下一步行动
- [ ] 运行 `npm run tauri dev` 视觉验证全功能
- [ ] 配置 API Keys（FRED, Groq, Claude）验证真实数据流
- [ ] 与 QuantTerminal 联调（REST + WS + market_context.db）
