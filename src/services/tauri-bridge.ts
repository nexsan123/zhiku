/**
 * tauri-bridge.ts — Service layer bridging frontend to Tauri backend commands.
 *
 * When running inside Tauri (desktop):  calls invoke() / listen()
 * When running in browser dev mode:     returns mock data, no invoke calls
 *
 * All backend command names and parameter shapes are documented alongside
 * the Rust command signatures in src-tauri/src/commands/.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { NewsItem } from '@contracts/api-news';
import type { ApiServiceName, ApiServiceStatus } from '@contracts/app-types';
import {
  MOCK_NEWS,
  MOCK_SIGNALS,
  MOCK_MARKET_VERDICT,
  MOCK_INDICES,
  MOCK_FOREX,
  MOCK_OIL,
  MOCK_CRYPTO,
  MOCK_STABLECOINS,
  MOCK_FEAR_GREED,
} from '@utils/mocks/panel-data';
import { MOCK_API_STATUS } from '@utils/mocks/api-status';

// ============================================================
// Environment detection
// ============================================================

/** Returns true when running inside Tauri WebView (not plain browser). */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

// ============================================================
// Internal types for backend response shapes
// Rust structs use serde(rename_all = "camelCase") so field names
// arrive in JS as camelCase.
// ============================================================

/** MacroData row from the macro_data table. Matches Rust MacroData struct. */
export interface MacroDataItem {
  id: number;
  indicator: string;
  value: number;
  period: string | null;
  source: string;
  fetchedAt: string;
}

/**
 * API status row returned by get_api_status command.
 * Matches Rust ApiStatus struct (camelCase via serde).
 */
export interface BackendApiStatus {
  service: string;
  status: string;
  lastCheck: string | null;
  lastError: string | null;
  responseMs: number | null;
}

// ============================================================
// Market data types — matches Rust MarketSnap and RadarSignal structs
// Rust uses serde(rename_all = "camelCase") so snake_case fields arrive as camelCase.
// ============================================================

export interface MarketDataItem {
  id: number;
  symbol: string;
  price: number;
  changePct: number | null;   // Rust: change_pct Option<f64>
  volume: number | null;      // Rust: volume Option<f64>
  timestamp: string;
  source: string;
}

export interface MarketRadarSignal {
  name: string;
  bullish: boolean | null;    // Rust: bullish Option<bool>
  detail: string;
}

export interface MarketRadarData {
  signals: MarketRadarSignal[];
  verdict: string;
  bullishPct: number;         // Rust: bullish_pct f64
  timestamp: string;
}

// ============================================================
// Helper — derive a display-friendly source name from a URL
// ============================================================

export function hostnameFromUrl(url: string): string {
  try {
    const { hostname } = new URL(url);
    // Strip "www." prefix and return, e.g. "reuters.com"
    return hostname.replace(/^www\./, '');
  } catch {
    return url;
  }
}

// ============================================================
// Helper — relative time string from ISO 8601 timestamp
// ============================================================

export function formatTimeAgo(isoString: string): string {
  const date = new Date(isoString);
  if (isNaN(date.getTime())) return 'unknown time';
  const diffMs = Date.now() - date.getTime();
  const diffMins = Math.floor(diffMs / 60_000);
  if (diffMins < 1) return 'just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  return `${diffDays}d ago`;
}

// ============================================================
// News commands
// ============================================================

/**
 * Fetch all news from DB (latest 200).
 * Backend: invoke('get_news') → Vec<NewsItem>
 */
export async function getNews(): Promise<NewsItem[]> {
  if (!isTauri()) {
    // Return mock data shaped as NewsItem contract
    return MOCK_NEWS.map((m) => ({
      id: m.id,
      title: m.title,
      summary: m.title, // mock has no separate summary
      sourceUrl: `https://${m.source.toLowerCase()}.com`,
      category: m.category === 'macro' ? 'macro_policy' : m.category,
      country: 'us',
      publishedAt: new Date(Date.now() - Math.random() * 3_600_000).toISOString(),
      fetchedAt: new Date().toISOString(),
    })) as NewsItem[];
  }

  const data = await invoke<NewsItem[]>('get_news');
  return data;
}

/**
 * Get total count of news articles in DB.
 * Backend: invoke('get_news_count') → i64
 */
export async function getNewsCount(): Promise<number> {
  if (!isTauri()) return MOCK_NEWS.length;
  return invoke<number>('get_news_count');
}

/**
 * Trigger RSS fetch for all configured feeds.
 * Backend: invoke('fetch_rss') → usize (newly inserted count)
 */
export async function fetchRss(): Promise<number> {
  if (!isTauri()) return 0;
  return invoke<number>('fetch_rss');
}

// ============================================================
// Macro data commands
// ============================================================

/**
 * Fetch all macro data from DB.
 * Backend: invoke('get_macro_data') → Vec<MacroData>
 */
export async function getMacroData(): Promise<MacroDataItem[]> {
  if (!isTauri()) {
    // Mock uses uppercase FRED series IDs to match what the backend actually stores.
    // fear_greed_index and oil indicators use lowercase (non-FRED sources).
    const now = new Date().toISOString();
    return [
      // FRED indicators
      { id: 1,  indicator: 'FEDFUNDS',               value: 5.25,   period: '2026-01', source: 'FRED',         fetchedAt: now },
      { id: 2,  indicator: 'CPIAUCSL',               value: 3.1,    period: '2026-01', source: 'FRED',         fetchedAt: now },
      { id: 3,  indicator: 'UNRATE',                 value: 3.9,    period: '2026-01', source: 'FRED',         fetchedAt: now },
      // Other macro indicators
      { id: 4,  indicator: 'fear_greed_index',        value: 72,     period: null,      source: 'alternative.me', fetchedAt: now },
      { id: 5,  indicator: 'wti_crude',               value: 72.43,  period: null,      source: 'EIA',         fetchedAt: now },
      { id: 6,  indicator: 'brent_crude',             value: 76.21,  period: null,      source: 'EIA',         fetchedAt: now },
      // BIS central bank policy rates (BIS_CBPOL_<country-code>)
      { id: 10, indicator: 'BIS_CBPOL_US',            value: 5.25,   period: '2026-02', source: 'BIS',         fetchedAt: now },
      { id: 11, indicator: 'BIS_CBPOL_EU',            value: 4.50,   period: '2026-02', source: 'BIS',         fetchedAt: now },
      { id: 12, indicator: 'BIS_CBPOL_JP',            value: 0.10,   period: '2026-02', source: 'BIS',         fetchedAt: now },
      { id: 13, indicator: 'BIS_CBPOL_UK',            value: 5.25,   period: '2026-02', source: 'BIS',         fetchedAt: now },
      { id: 14, indicator: 'BIS_CBPOL_CN',            value: 3.45,   period: '2026-02', source: 'BIS',         fetchedAt: now },
      // BTC network health (mempool.space)
      { id: 20, indicator: 'BTC_HASHRATE',            value: 718.5,  period: null,      source: 'mempool',     fetchedAt: now },
      { id: 21, indicator: 'BTC_FEE_MEDIUM',          value: 12,     period: null,      source: 'mempool',     fetchedAt: now },
      { id: 22, indicator: 'BTC_FEE_FAST',            value: 18,     period: null,      source: 'mempool',     fetchedAt: now },
      { id: 23, indicator: 'BTC_DIFFICULTY_PROGRESS', value: 67.3,   period: null,      source: 'mempool',     fetchedAt: now },
      { id: 24, indicator: 'BTC_DIFFICULTY_CHANGE',   value: 2.41,   period: null,      source: 'mempool',     fetchedAt: now },
    ];
  }
  return invoke<MacroDataItem[]>('get_macro_data');
}

// ============================================================
// Market data commands
// ============================================================

/**
 * Build mock MarketDataItem list covering indices, forex, oil, and crypto.
 * Used in browser dev mode only — Tauri env calls invoke('get_market_data').
 */
function buildMockMarketData(): MarketDataItem[] {
  const now = new Date().toISOString();

  const indices: MarketDataItem[] = MOCK_INDICES.map((m, i) => ({
    id: i + 1,
    symbol: `^${m.ticker}`,
    price: parseFloat(m.price.replace(/,/g, '')),
    changePct: parseFloat(m.changePercent),
    volume: null,
    timestamp: now,
    source: 'mock',
  }));

  const forex: MarketDataItem[] = MOCK_FOREX.map((m, i) => ({
    id: 100 + i,
    // Convert "EUR/USD" → "EURUSD=X" to match Yahoo Finance symbol convention
    symbol: m.pair.replace('/', '') + '=X',
    price: parseFloat(m.price),
    changePct: parseFloat(m.changePercent),
    volume: null,
    timestamp: now,
    source: 'mock',
  }));

  const oil: MarketDataItem[] = MOCK_OIL.map((m, i) => ({
    id: 200 + i,
    symbol: m.symbol,  // already "CL=F" / "BZ=F"
    price: parseFloat(m.price.replace('$', '')),
    changePct: m.trend === 'up' ? 1.2 : -0.8,
    volume: null,
    timestamp: now,
    source: 'mock',
  }));

  const crypto: MarketDataItem[] = MOCK_CRYPTO.map((m, i) => ({
    id: 300 + i,
    symbol: `${m.symbol}-USD`,
    price: parseFloat(m.price.replace(/[$,]/g, '')),
    changePct: parseFloat(m.changePercent),
    volume: null,
    timestamp: now,
    source: 'mock',
  }));

  // Stablecoins — price around 1.0 so peg derivation works
  const stablecoins: MarketDataItem[] = MOCK_STABLECOINS.map((s, i) => {
    const price = s.peg === 'on-peg' ? 1.0002 : s.peg === 'slight-depeg' ? 0.9971 : 0.9820;
    return {
      id: 400 + i,
      symbol: `${s.symbol}-USD`,
      price,
      changePct: 0,
      volume: null,
      timestamp: now,
      source: 'mock',
    };
  });

  return [...indices, ...forex, ...oil, ...crypto, ...stablecoins];
}

/**
 * Get market data (indices, forex, crypto, energy futures).
 * Backend: invoke('get_market_data') → Vec<MarketSnap>
 */
export async function getMarketData(): Promise<MarketDataItem[]> {
  if (isTauri()) return invoke<MarketDataItem[]>('get_market_data');
  return buildMockMarketData();
}

/**
 * Get 7-signal market radar verdict.
 * Backend: invoke('get_market_radar') → MarketRadarResult
 */
export async function getMarketRadar(): Promise<MarketRadarData> {
  if (isTauri()) return invoke<MarketRadarData>('get_market_radar');
  const bullishCount = MOCK_SIGNALS.filter((s) => s.bullish === true).length;
  return {
    signals: MOCK_SIGNALS,
    verdict: MOCK_MARKET_VERDICT,
    bullishPct: (bullishCount / MOCK_SIGNALS.length) * 100,
    timestamp: new Date().toISOString(),
  };
}

// ============================================================
// MOCK_FEAR_GREED re-export for FearGreedPanel non-Tauri fallback
// ============================================================

export { MOCK_FEAR_GREED };

// ============================================================
// API Status commands
// ============================================================

/**
 * Get status of all tracked API services from DB.
 * Backend: invoke('get_api_status') → Vec<ApiStatus>
 */
export async function getApiStatus(): Promise<BackendApiStatus[]> {
  if (!isTauri()) {
    return Object.values(MOCK_API_STATUS).map((s) => ({
      service: s.service,
      status: s.status,
      lastCheck: s.lastCheck ?? null,
      lastError: s.lastError ?? null,
      responseMs: s.responseMs ?? null,
    }));
  }
  return invoke<BackendApiStatus[]>('get_api_status');
}

// ============================================================
// Event listeners
// ============================================================

const VALID_STATUSES = new Set<string>(['online', 'offline', 'checking', 'idle']);

function sanitizeStatus(raw: string): ApiServiceStatus['status'] {
  return VALID_STATUSES.has(raw)
    ? (raw as ApiServiceStatus['status'])
    : 'offline';
}

/**
 * Listen for 'api-status-changed' Tauri events.
 * Returns a cleanup function (call on component unmount).
 */
export function listenApiStatusChanged(
  callback: (status: ApiServiceStatus) => void,
): Promise<() => void> {
  if (!isTauri()) {
    // In browser dev mode, return a no-op cleanup
    return Promise.resolve(() => undefined);
  }

  return listen<BackendApiStatus>('api-status-changed', (event) => {
    const raw = event.payload;
    const mapped: ApiServiceStatus = {
      service: raw.service as ApiServiceName,
      status: sanitizeStatus(raw.status),
      lastCheck: raw.lastCheck ?? undefined,
      lastError: raw.lastError ?? undefined,
      responseMs: raw.responseMs ?? undefined,
    };
    callback(mapped);
  });
}

/**
 * Listen for 'news-updated' Tauri events (new articles inserted).
 * Returns a cleanup function (call on component unmount).
 */
export function listenNewsUpdated(callback: () => void): Promise<() => void> {
  if (!isTauri()) {
    return Promise.resolve(() => undefined);
  }
  return listen('news-updated', () => callback());
}

/**
 * Listen for 'market-updated' Tauri events.
 * Returns a cleanup function (call on component unmount).
 */
export function listenMarketUpdated(callback: () => void): Promise<() => void> {
  if (!isTauri()) {
    return Promise.resolve(() => undefined);
  }
  return listen('market-updated', () => callback());
}

// ============================================================
// AI Brief commands
// ============================================================

/**
 * AI-generated summary per news category.
 * Matches Rust AiBriefItem struct (camelCase via serde).
 */
export interface AiBriefCategory {
  category: string;
  count: number;
  avgSentiment: number;
  topKeywords: string[];
  latestSummary: string;
}

/** Mock AI Brief data used in browser dev mode. */
const MOCK_AI_BRIEF: AiBriefCategory[] = [
  {
    category: 'geopolitical',
    count: 8,
    avgSentiment: 0.3,
    topKeywords: ['中东', '制裁', '贸易战'],
    latestSummary: '中东紧张局势持续，多国扩大制裁范围，贸易摩擦风险上升，供应链压力加剧。',
  },
  {
    category: 'macro_policy',
    count: 12,
    avgSentiment: 0.5,
    topKeywords: ['美联储', '利率', 'CPI'],
    latestSummary: '美联储维持利率不变，市场等待通胀数据确认降息时间窗口，预期分歧较大。',
  },
  {
    category: 'market',
    count: 15,
    avgSentiment: 0.7,
    topKeywords: ['纳斯达克', 'AI股', '科技板块'],
    latestSummary: 'AI 相关科技股领涨，纳斯达克创阶段新高，资金持续流入成长型资产。',
  },
  {
    category: 'corporate',
    count: 6,
    avgSentiment: 0.55,
    topKeywords: ['财报季', '盈利', '预期'],
    latestSummary: '财报季整体符合预期，头部企业 AI 资本开支超预期，带动板块情绪乐观。',
  },
];

/**
 * Fetch AI-generated brief summaries grouped by news category.
 * Backend: invoke('get_ai_brief') → Vec<AiBriefCategory>  (Phase 3)
 */
export async function getAiBrief(): Promise<AiBriefCategory[]> {
  if (!isTauri()) {
    return MOCK_AI_BRIEF;
  }
  return invoke<AiBriefCategory[]>('get_ai_brief');
}

// ============================================================
// Cycle Indicators & Reasoning (Phase 3.5-3.6)
// ============================================================

export interface MonetaryCycle {
  fedRate: number;
  m2Growth: number;
  rateDirection: string;
  policyStance: string;
}

export interface CreditCycle {
  creditSpread: number;
  yieldCurve: string;
  phase: string;
}

export interface EconomicCycle {
  gdpGrowth: number;
  unemployment: number;
  cpiInflation: number;
  phase: string;
}

export interface MarketCycle {
  sp500Trend: number;
  vixLevel: number;
  dxyTrend: number;
  phase: string;
}

export interface SentimentCycle {
  fearGreed: number;
  newsSentimentAvg: number;
  phase: string;
}

export interface GeopoliticalRisk {
  riskLevel: string;
  keyEvents: string[];
  eventCount: number;
}

export interface CycleIndicators {
  monetary: MonetaryCycle;
  credit: CreditCycle;
  economic: EconomicCycle;
  market: MarketCycle;
  sentiment: SentimentCycle;
  geopolitical: GeopoliticalRisk;
  calculatedAt: string;
}

export interface TurningSignal {
  signal: string;
  direction: string;
  strength: string;
}

export interface CycleReasoning {
  cyclePosition: string;
  monetaryPolicyStage: string;
  sentimentStage: string;
  turningSignals: TurningSignal[];
  sectorRecommendations: string[];
  tailRisks: string[];
  confidence: number;
  reasoningChain: string;
  timestamp: string;
}

/** Mock cycle indicators for browser dev mode. */
const MOCK_CYCLE_INDICATORS: CycleIndicators = {
  monetary: { fedRate: 5.25, m2Growth: -2.1, rateDirection: 'pausing', policyStance: 'hawkish' },
  credit: { creditSpread: 0, yieldCurve: 'unavailable', phase: 'unknown' },
  economic: { gdpGrowth: 2.8, unemployment: 3.9, cpiInflation: 3.1, phase: 'mid_expansion' },
  market: { sp500Trend: 4.2, vixLevel: 16.5, dxyTrend: -1.3, phase: 'bull' },
  sentiment: { fearGreed: 72, newsSentimentAvg: 0.65, phase: 'optimism' },
  geopolitical: { riskLevel: 'moderate', keyEvents: ['中东紧张局势', '美中贸易摩擦'], eventCount: 2 },
  calculatedAt: new Date().toISOString(),
};

const MOCK_CYCLE_REASONING: CycleReasoning = {
  cyclePosition: 'mid_expansion',
  monetaryPolicyStage: 'pausing',
  sentimentStage: 'optimism',
  turningSignals: [
    { signal: '就业市场韧性超预期', direction: 'bullish', strength: 'moderate' },
    { signal: 'CPI 回落速度放缓', direction: 'bearish', strength: 'weak' },
  ],
  sectorRecommendations: ['科技', '可选消费', '金融'],
  tailRisks: ['地缘政治黑天鹅事件', '通胀反弹迫使重启加息'],
  confidence: 0.72,
  reasoningChain: '联邦基金利率维持高位但暂停加息，GDP增速健康，失业率低，通胀回落但仍高于目标。市场处于牛市阶段，VIX处于低位，情绪偏乐观。综合判断处于中期扩张阶段。',
  timestamp: new Date().toISOString(),
};

export async function getCycleIndicators(): Promise<CycleIndicators> {
  if (!isTauri()) return MOCK_CYCLE_INDICATORS;
  return invoke<CycleIndicators>('get_cycle_indicators');
}

export async function getCycleReasoning(): Promise<CycleReasoning | null> {
  if (!isTauri()) return MOCK_CYCLE_REASONING;
  return invoke<CycleReasoning | null>('get_cycle_reasoning');
}

export async function triggerCycleReasoning(): Promise<CycleReasoning> {
  if (!isTauri()) return MOCK_CYCLE_REASONING;
  return invoke<CycleReasoning>('trigger_cycle_reasoning');
}

export function listenCycleUpdated(callback: (reasoning: CycleReasoning) => void): Promise<() => void> {
  if (!isTauri()) return Promise.resolve(() => undefined);
  return listen<CycleReasoning>('cycle-reasoning-updated', (event) => callback(event.payload));
}
