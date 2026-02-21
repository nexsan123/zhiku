import type { NewsCategory, AiEngine } from './app-types';

// 新闻条目（Phase 2 完善字段）
export interface NewsItem {
  id: string;
  title: string;
  summary: string;
  sourceUrl: string;
  category: NewsCategory;
  country: string;
  publishedAt: string;  // ISO 8601, TZ-aware
  fetchedAt: string;
}

// AI 分析结果（Phase 3 完善字段）
export interface AiAnalysis {
  id: string;
  newsId: string;
  engine: AiEngine;
  summary: string;
  sourceUrl: string;  // 必须关联原始新闻 URL（ZK-01 铁律）
  createdAt: string;
}
