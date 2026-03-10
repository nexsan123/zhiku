// 页面路由（保留向后兼容）
export type PageId = 'map' | 'finance' | 'ai' | 'notifications' | 'settings';

// 通知级别
export enum NotificationPriority {
  P0_CRITICAL = 'P0',  // 弹窗
  P1_IMPORTANT = 'P1', // 角标
  P2_ROUTINE = 'P2',   // 静默
}

// 新闻分类
export enum NewsCategory {
  GEOPOLITICAL = 'geopolitical',
  MACRO_POLICY = 'macro_policy',
  MARKET = 'market',
  CORPORATE = 'corporate',
}

// 数据源状态（保留向后兼容）
export type DataSourceStatus = 'connected' | 'fetching' | 'error' | 'idle';

// AI 引擎（保留向后兼容）
export type AiEngine = 'ollama' | 'claude';

// ============================================================
// Phase 4 新增类型
// ============================================================

// 面板 ID 枚举 — 左栏 7 + 右栏 8
export type PanelId =
  | 'cycle-reasoning'
  | 'news-feed'
  | 'ai-brief'
  | 'fred-indicators'
  | 'bis-rates'
  | 'wto-trade'
  | 'supply-chain'
  | 'market-radar'
  | 'indices'
  | 'forex'
  | 'oil-energy'
  | 'crypto'
  | 'btc-etf'
  | 'fear-greed'
  | 'gulf-fdi';

// API 服务名称
export type ApiServiceName =
  | 'ollama'
  | 'groq'
  | 'claude'
  | 'fred'
  | 'yahoo'
  | 'eia'
  | 'bis'
  | 'wto'
  | 'coingecko'
  | 'rss'
  | 'fear_greed'
  | 'qt_rest'
  | 'qt_ws';

// 单个 API 服务状态
export interface ApiServiceStatus {
  service: ApiServiceName;
  status: 'online' | 'offline' | 'checking' | 'idle';
  lastCheck?: string;
  lastError?: string;
  responseMs?: number;
}

// 面板折叠状态
export interface PanelState {
  expanded: boolean;
}
