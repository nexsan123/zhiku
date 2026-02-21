// 页面路由
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

// 数据源状态
export type DataSourceStatus = 'connected' | 'fetching' | 'error' | 'idle';

// AI 引擎
export type AiEngine = 'ollama' | 'claude';
