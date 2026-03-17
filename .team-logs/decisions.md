# 智库 · 决策记录

> 记录所有重要设计决策、退回记录、变更历史

---

## [2026-02-17] 项目启动决策

- **方案选择**：甲案（纵切法）— 一国打通全链路，再横向扩展
- **MVP 范围**：金融板块
- **首个目标国**：美国
- **量化接口方案**：API 为主（REST/WebSocket），MCP 为辅
- **AI 引擎**：Claude API + 本地模型（双引擎可切换）
- **决策依据**：符合 Q-04「做完一个确定一个」，QuantTerminal 可早期受益

## [2026-02-17] 七轮访谈 + 圣旨钦定

- **圣旨编号**：edict-001-mvp
- **访谈覆盖**：七轮全走（核心意图 / 用户场景 / UI体验 / 数据内容 / 技术环境 / 集成边界 / 底线验收）
- **全景蓝图**：7 张交付物（访谈摘要 / 架构图 / 页面流转图 / 布局草图 / 技术栈表 / 集成图 / 阶段证据矩阵）
- **关键决策**：
  - 地图库：react-simple-maps（暗色可定制，免费开源）
  - 量化通信：WebSocket Server ws://localhost:9600（智库为 Server，QT 为 Client）
  - AI 策略：Ollama 14B 做批量摘要，Claude API 做深度分析
  - 数据优先级：地缘政治 > 宏观政策 > 市场行情 > 企业行业
  - 通知分级：P0 弹窗 / P1 角标 / P2 静默
  - 敏感数据：发送外部 API 前必须弹窗授权
- **阶段拆分**：6 大阶段，22 子阶段，每子阶段有完成标准 + 证据类型
- **体系修复**：司礼监 SKILL.md v3.2 → v3.5（INT-01 七轮访谈 / BP-01 全景蓝图 / EM-01 证据矩阵）

## [2026-03-08] 项目重塑决策 · edict-002

- **触发原因**：皇上发现 World Monitor (koala73/worldmonitor, 33.9k stars)，决定借鉴其 UI 和产品设计重塑智库
- **方案选择**：乙案 — 借鉴 WM 产品设计 + UI，用自己技术栈重写（非 fork）
- **范围限定**：先做金融板块，供应 QuantTerminal
- **定位确认**：情报辅助层（输出情报信号，非原始行情）
- **关键变更**：
  - 地图：react-simple-maps → deck.gl + MapLibre GL
  - 布局：Sidebar 导航 → WM 风格三栏面板（左栏+地图+右栏）
  - AI 引擎：双引擎 → 丙方案（Ollama→Groq兜底 + Claude深度）
  - 新增：金融周期推理引擎（6类指标 → Claude推理 → 结构化JSON → QT）
  - 新增：API 状态灯可视化
  - 数据源：L1(FRED+RSS+Yahoo) / L2(EIA+BIS+F&G) / L3(WTO+CoinGecko+FDI)
  - RSS 补充中文金融源
- **现有代码处置**：保留60% / 修改25% / 删除15%（Sidebar + 5空页面）
- **edict-001 状态**：被 edict-002 替代（方向一致，架构升级）

## [2026-03-10] AI 情报引擎方向确定 · edict-003

- **触发原因**：皇上审查项目后，确认 AI 引擎是项目核心能力（非 UI）
- **核心定位修正**：从"全球金融情报中枢"精确为"QuantTerminal 的情报前置层"，灵魂是 AI 推理
- **AI 引擎目标**：情报分析师级（最高层次）
  - 不是摘要机器，不是推理引擎，是完整的情报分析师
  - 数据源 AI 审计（入库审计 + 运行中监控）
  - 多源交叉验证 + 地缘推演 + 黑天鹅预警 + 研报生成
- **关键架构决策**：多模型可插拔 AI 调度层
  - 不锁死 Claude/Groq/Ollama，要支持 N 个 AI 模型提供商
  - 统一 AiProvider trait 接口
  - 智能路由：按任务类型/成本/可用性选模型
  - 多模型交叉验证：同一任务多模型对比，分歧大则仲裁
- **执行节奏**：A → B → C → D
  - A：设置页面 + Groq 跑通第一条 AI 链路（当前）
  - B：统一 AiProvider trait，多模型可插拔
  - C：多模型路由 + 交叉验证
  - D：数据源 AI 审计 + 实时情报流 + 研报
- **设置页面需求**：
  - TitleBar 按钮打开
  - 数据源管理（查看/增删/状态灯）
  - AI 模型管理（配置/测试/状态灯）
  - API Key 配置
- **情报产出优先级**：实时情报流优先，其他（简报/研报/预警）都要
- **edict-002 状态**：仍有效，edict-003 为其子任务

## [2026-03-13] determine_phase() 收入指标决策

- **问题**：IMF 收入侧指标（GDP增速、经常账户、财政赤字等 5 项）已入库展示，是否集成到 Rust 规则引擎 `determine_phase()`？
- **决策**：**不集成，等 Phase 3 AI 引擎**
- **理由**：
  1. `determine_phase()` 是硬编码规则引擎，用 6 个 BIS 信贷指标判断周期相位，逻辑已经够复杂。再叠加 5 个 IMF 指标，规则组合爆炸，维护成本高
  2. IMF 收入指标半年更新一次，是宏观背景而非周期信号 — 适合 AI 推理而非规则匹配
  3. Phase 3 AI 引擎（Claude）能综合权衡这些指标的相互影响，比 if-else 规则更准确
  4. 当前 BIS 6 指标已足够判定信贷周期相位，收入指标是增强而非必须
- **结论**：IMF 收入指标保持 display-only，Phase 3 AI 引擎中作为推理输入

## [2026-03-13] 数据源运行时审计 + 修复

- **BIS API**：v2 (`data.bis.org`) 全球废弃 → 迁移至 v1 (`stats.bis.org`)，v1 支持 CSV 无需 XML。WS_CREDIT 在 v1 改名为 WS_TC
- **T0 RSS**：8 源中 2 正常、3 换 URL、1 改 UA、2 删除（无 RSS feed）。修复后 6/6 全通
- **RSSHub**：公共 rsshub.app 被 Cloudflare 拦截 → 改为可配置 base URL，推荐自建 Docker 实例
- **死源清理**：Reuters (全球停 RSS)、Barron's (域名死)、Forbes Markets (Datadome)、SEC Enforcement (无 feed)、OFAC (无 feed)
- **CLAUDE.md**：圣旨引用从 edict-002 更新为 edict-005
