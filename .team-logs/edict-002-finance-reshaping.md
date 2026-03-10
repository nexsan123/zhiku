# 【圣旨】edict-002 · 智库 Finance 全球金融情报中枢重塑

> 钦定日期：2026-03-08
> 替代：edict-001-mvp（方向不变，架构重塑）
> 状态：已颁发

## 旨意

以 World Monitor (koala73/worldmonitor) 产品设计与 UI 为蓝本，用 Tauri v2 + React 19 + Rust 技术栈重写金融板块，构建情报辅助层，内置金融周期推理引擎，供应 QuantTerminal 量化策略调节因子。

## 背景

- 智库项目已完成 Phase 1 骨架（Tauri v2 + React 19 + 暗色主题 + 5 页面占位符），实际功能为零
- World Monitor（33.9k stars）提供了成熟的金融情报 UI 范式：三栏布局、面板堆叠、地图中心、图层切换
- 皇上决定走乙案：借鉴 WM 产品设计 + UI，自己技术栈重写，先做金融板块
- 智库定位为情报辅助层，输出情报信号而非原始行情数据

## 核心决策

### 技术栈

| 层 | 技术 |
|---|------|
| 前端 | React 19 + Zustand + deck.gl + MapLibre GL |
| 后端 | Rust (Tauri v2) + tokio + reqwest + sqlx |
| 数据库 | SQLite (tauri-plugin-sql) |
| AI 批量 | Ollama (本地 14B) → Groq (免费兜底) |
| AI 深度 | Claude API |
| 地图 | deck.gl + MapLibre GL (替换 react-simple-maps) |
| 图表 | 内联 SVG sparkline |

### 数据源分层

| 层级 | 数据源 | 内容 |
|------|--------|------|
| L1 必须 | FRED | 利率、通胀、就业、GDP、M2 |
| L1 必须 | RSS (55条英文 + 中文补充) | 金融新闻 |
| L1 必须 | Yahoo Finance | 主要指数、外汇、加密报价 |
| L2 重要 | EIA | WTI/Brent 油价、美国产量、库存 |
| L2 重要 | BIS | 央行政策利率、实际汇率、信贷/GDP |
| L2 重要 | alternative.me | Fear & Greed 情绪指数 |
| L3 增强 | WTO | 贸易限制、关税、贸易流量、壁垒 |
| L3 增强 | CoinGecko | 稳定币健康度、加密市值 |
| L3 增强 | mempool.space | BTC 算力、网络状态 |
| L3 增强 | 静态策划 | 92交易所 + 13央行 + 19金融中心 + 64海湾FDI |

### AI 引擎（丙方案）

- 批量任务（高频低质）：Ollama → Groq 兜底
  - 新闻摘要、情绪打分、关键词提取、去重
- 深度任务（低频高质）：Claude API（无替代）
  - 金融周期推理、地缘推演、政策影响分析、情报综合报告
- API 状态灯：各数据源/AI 引擎实时状态可视化

### 金融周期推理引擎

- Layer 1: 原始数据（全部数据源）
- Layer 2: 指标计算（Rust 本地，6 类周期指标）
  - 货币周期 / 信用周期 / 经济周期 / 市场周期 / 情绪周期 / 地缘风险
- Layer 3: AI 周期推理（Claude API）
  - 周期定位（扩张早/中/晚期、衰退、复苏）
  - 货币政策阶段（加息/暂停/降息/QE/QT）
  - 市场情绪阶段（恐慌→狂热 6 级）
  - 关键转折信号 + 板块建议 + 尾部风险 + 置信度
- Layer 4: 结构化 JSON 输出 → QuantTerminal
- 推理频率：周期定位日频 / 转折预警 6h / 情绪快照 1h / P0 事件即时

### UI 布局（仿 World Monitor）

- TitleBar: 智库 | FINANCE | Cmd+K | SOURCES | INTEL | 通知 | 窗口控制
- Body: 左栏(320px) + 中心地图(flex) + 右栏(320px)，左右独立滚动可收起
- StatusBar: API 状态灯（Ollama/Groq/Claude/FRED/Yahoo/EIA...）+ 就绪状态 + 时间
- 面板: 16 个（左栏 6 + 右栏 8 + 浮层 2），毛玻璃背景，可折叠

### QuantTerminal 集成

- REST: localhost:9601/api/v1/{signals|macro-score|market-radar|ai-brief|cycle}
- WS: ws://localhost:9600 → signal.new / macro.update / cycle.update / alert.p0

### 现有代码处置

- 保留（60%）：Tauri配置 + Rust入口 + Cargo依赖 + 设计令牌 + CSS变量 + 毛玻璃规范 + 团队配置
- 修改（25%）：TitleBar + StatusBar + App.tsx + Store + 契约类型 + app-layout.md + package.json
- 删除（15%）：Sidebar + 5个空页面占位符 + map-page.md 设计稿

## 底线

### 禁止
- ❌ 未经实际执行就报告"已完成"
- ❌ 伪造测试结果或虚假日志
- ❌ 推送未经验证的垃圾数据给 QuantTerminal（ZK-02）
- ❌ AI 分析不关联原始新闻源 URL（ZK-01）
- ❌ 硬编码 API Key
- ❌ AI 推理输出无置信度和推理链
- ❌ 跳过 Ollama→Groq 兜底逻辑直接调 Claude 做批量任务

### 必须
- ✅ 实际运行代码并展示真实输出
- ✅ 每条 AI 分析附带 source_urls + confidence + reasoning_chain
- ✅ API 状态灯实时显示各数据源/AI 引擎状态
- ✅ 数据去重 + 时效性校验（ZK-05）
- ✅ Ollama 失败时自动切换 Groq，Claude 宕机时状态灯变红
- ✅ 周期推理输出结构化 JSON

## 验收标准

| 检验项 | 方法 | 预期结果 |
|--------|------|---------|
| 数据采集 | SELECT count(*) FROM news | > 0 |
| 数据去重 | 重复插入同一 URL | count 不变 |
| AI 批量 | 关闭 Ollama 后触发摘要 | 自动切 Groq，状态灯变红 |
| AI 深度 | 调用周期推理 | 返回含 cycle + turning_signals + confidence 的 JSON |
| 前端布局 | 启动应用 | 三栏布局 + 面板可折叠/滚动 + 地图居中 |
| 状态灯 | 关闭 Ollama | 底栏指示灯变红 |
| 地图 | 启动应用 | deck.gl 地图渲染 + ≥3 图层可切换 |
| QT REST | curl localhost:9601/api/v1/cycle | 返回结构化 JSON |
| QT WS | WS 客户端连接 :9600 | 收到 signal.new 消息 |

## 阶段路线

| 阶段 | 内容 | 子阶段 |
|------|------|--------|
| Phase 2 | 数据引擎 | 2.1 RSS采集 / 2.2 去重 / 2.3 FRED / 2.4 Yahoo / 2.5 SmartPollLoop / 2.6 其他数据源 |
| Phase 3 | AI 引擎 | 3.1 Ollama / 3.2 Groq兜底 / 3.3 Claude深度 / 3.4 7-Signal / 3.5 指标计算 / 3.6 周期推理 / 3.7 定时触发 |
| Phase 4 | 前端面板 | 4.1 三栏布局 / 4.2 TitleBar / 4.3 StatusBar状态灯 / 4.4 L1面板 / 4.5 L2面板 / 4.6 L3面板 / 4.7 面板交互 / 4.8 推理面板 |
| Phase 5 | 地图+集成 | 5.1 deck.gl / 5.2 图层系统 / 5.3 QT REST / 5.4 QT WS / 5.5 Cmd+K搜索 |

**验真令**：所有"已完成"声明必须附带实际执行的命令和输出。

**止损令**：同一问题修改超过 3 次未果，必须暂停并报告。
