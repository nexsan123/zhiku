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
  freshness?: string;       // "live" | "recent" | "aging" | "stale" | "expired" | "unknown"
  minutesAgo?: number | null;
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

/**
 * Listen for 'macro-updated' Tauri events (FRED/EIA/BIS/IMF data refresh).
 * Returns a cleanup function (call on component unmount).
 */
export function listenMacroUpdated(callback: () => void): Promise<() => void> {
  if (!isTauri()) {
    return Promise.resolve(() => undefined);
  }
  return listen('macro-updated', () => callback());
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
  tradeEvents: string[];
  tradeCount: number;
  macroPolicyEvents: string[];
  macroPolicyCount: number;
  centralBankEvents: string[];
  centralBankCount: number;
  energyEvents: string[];
  energyCount: number;
}

export interface CommodityCycle {
  oilPrice: number;
  oilTrend: number;
  goldPrice: number;
  goldTrend: number;
  copperPrice: number;
  copperTrend: number;
  natgasPrice: number;
  natgasTrend: number;
  phase: string;
}

export interface CryptoSignal {
  btcPrice: number;
  btcTrend: number;
  phase: string;
}

export interface FiscalSnapshot {
  usDebtGdp: number;
  cnDebtGdp: number;
  usFiscalBalance: number;
  cnFiscalBalance: number;
  usGdpGrowth: number;
  cnGdpGrowth: number;
}

export interface EnergyData {
  wtiPrice: number;
  brentPrice: number;
  spread: number;
}

export interface CycleIndicators {
  monetary: MonetaryCycle;
  credit: CreditCycle;
  economic: EconomicCycle;
  market: MarketCycle;
  sentiment: SentimentCycle;
  geopolitical: GeopoliticalRisk;
  commodities: CommodityCycle;
  crypto: CryptoSignal;
  fiscal: FiscalSnapshot;
  energy: EnergyData;
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
  geopolitical: {
    riskLevel: 'moderate',
    keyEvents: ['中东紧张局势', '美中贸易摩擦'],
    eventCount: 2,
    tradeEvents: [],
    tradeCount: 0,
    macroPolicyEvents: [],
    macroPolicyCount: 0,
    centralBankEvents: [],
    centralBankCount: 0,
    energyEvents: [],
    energyCount: 0,
  },
  commodities: { oilPrice: 78.5, oilTrend: -1.2, goldPrice: 2050.0, goldTrend: 0.8, copperPrice: 3.85, copperTrend: 0.3, natgasPrice: 2.1, natgasTrend: -0.5, phase: 'contraction' },
  crypto: { btcPrice: 67000, btcTrend: 5.2, phase: 'bull' },
  fiscal: { usDebtGdp: 122.3, cnDebtGdp: 51.2, usFiscalBalance: -6.2, cnFiscalBalance: -3.8, usGdpGrowth: 2.5, cnGdpGrowth: 4.9 },
  energy: { wtiPrice: 78.5, brentPrice: 82.1, spread: 3.6 },
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

// ==================== Settings ====================

export async function getSettings(): Promise<Record<string, string>> {
  if (!isTauri()) return {};
  try {
    return await invoke<Record<string, string>>('get_settings');
  } catch {
    return {};
  }
}

export async function setSetting(key: string, value: string): Promise<void> {
  if (!isTauri()) return;
  await invoke('set_setting', { key, value });
}

export async function deleteSetting(key: string): Promise<void> {
  if (!isTauri()) return;
  await invoke('delete_setting', { key });
}

export async function testConnection(service: string, apiKey?: string): Promise<{ success: boolean; message: string; responseMs: number }> {
  if (!isTauri()) {
    return { success: true, message: 'Mock OK (browser mode)', responseMs: 42 };
  }
  try {
    return await invoke<{ success: boolean; message: string; responseMs: number }>('test_connection', {
      service,
      apiKey: apiKey ?? null,
    });
  } catch (e) {
    return { success: false, message: String(e), responseMs: 0 };
  }
}

export async function getRssSources(): Promise<import('@contracts/app-types').RssSource[]> {
  if (!isTauri()) {
    return [
      { url: 'https://feeds.reuters.com/reuters/businessNews', name: 'Reuters Business', tier: 1, language: 'en', enabled: true },
      { url: 'https://feeds.bbci.co.uk/news/business/rss.xml', name: 'BBC Business', tier: 1, language: 'en', enabled: true },
      { url: 'https://rss.nytimes.com/services/xml/rss/nyt/Business.xml', name: 'NYT Business', tier: 2, language: 'en', enabled: true },
    ];
  }
  try {
    return await invoke<import('@contracts/app-types').RssSource[]>('get_rss_sources');
  } catch {
    return [];
  }
}

// ==================== AI Model Management ====================

export async function listAiModels(): Promise<import('@contracts/app-types').AiModelConfig[]> {
  if (!isTauri()) {
    return [
      { id: 'default-ollama', provider: 'ollama', displayName: 'Ollama', apiKey: '', modelName: 'llama3.1:8b', endpointUrl: 'http://localhost:11434', enabled: true },
      { id: 'default-groq', provider: 'groq', displayName: 'Groq', apiKey: '', modelName: 'llama-3.1-8b-instant', endpointUrl: '', enabled: true },
    ];
  }
  try {
    return await invoke<import('@contracts/app-types').AiModelConfig[]>('list_ai_models');
  } catch {
    return [];
  }
}

export async function saveAiModel(model: import('@contracts/app-types').AiModelConfig): Promise<void> {
  if (!isTauri()) return;
  await invoke('save_ai_model', { model });
}

export async function removeAiModel(id: string): Promise<void> {
  if (!isTauri()) return;
  await invoke('remove_ai_model', { id });
}

export async function testAiModel(modelId: string): Promise<{ success: boolean; message: string; responseMs: number }> {
  if (!isTauri()) {
    return { success: true, message: 'Mock OK (browser mode)', responseMs: 42 };
  }
  try {
    return await invoke<{ success: boolean; message: string; responseMs: number }>('test_ai_model', { modelId });
  } catch (e) {
    return { success: false, message: String(e), responseMs: 0 };
  }
}

export async function summarizePendingNews(): Promise<number> {
  if (!isTauri()) return 0;
  try {
    return await invoke<number>('summarize_pending_news');
  } catch {
    return 0;
  }
}

export function listenAiSummaryCompleted(callback: () => void): Promise<() => void> {
  if (!isTauri()) return Promise.resolve(() => undefined);
  return listen('ai-summary-completed', () => callback());
}

// ============================================================
// Credit Cycle types (edict-004 Phase E)
// ============================================================

export interface CountryCreditData {
  // Liability side (BIS)
  creditGdpGap: number | null;
  debtServiceRatio: number | null;
  creditGrowthYoy: number | null;
  creditImpulse: number | null;
  propertyPriceTrend: number | null;
  policyRate: number | null;
  rateDirection: string;
  // Income side (IMF WEO, edict-005)
  imfGdpGrowth: number | null;
  imfFiscalBalance: number | null;
  imfCurrentAccount: number | null;
  imfGovDebt: number | null;
  imfGovRevenue: number | null;
}

export interface CountryCyclePosition {
  countryCode: string;
  countryName: string;
  tier: string; // "core" | "important" | "monitor"
  indicators: CountryCreditData;
  phase: string; // "easing" | "leveraging" | "overheating" | "tightening" | "deleveraging" | "clearing" | "unknown"
  phaseLabel: string;
  confidence: number;
  confidenceGrade: string;
  reliability: number;
  dollarTideRiskModifier: number;
  dataPeriod: string;
}

export interface DollarTide {
  dxyTrend3m: number;
  dxyTrend6m: number;
  fedPolicy: string;
  m2Growth: number;
  yieldSpread: number;
  tideState: string; // "rising" | "neutral" | "ebbing"
  tideLabel: string;
  confidence: number;
}

export interface TierSummary {
  tier: string;
  dominantPhase: string;
  dominantPhaseLabel: string;
  avgCreditGap: number;
  warningCount: number;
}

export interface RiskAlert {
  countryCode: string;
  alert: string;
  severity: string;
  confidenceGrade: string;
}

export interface GlobalCycleOverview {
  countries: CountryCyclePosition[];
  globalPhase: string;
  globalPhaseLabel: string;
  globalPercentile: number;
  dollarTide: DollarTide;
  coreSummary: TierSummary;
  importantSummary: TierSummary;
  monitorSummary: TierSummary;
  riskAlerts: RiskAlert[];
  confidence: number;
  calculatedAt: string;
  dataPeriod: string;
}

const MOCK_GLOBAL_CYCLE: GlobalCycleOverview = {
  countries: [
    { countryCode: 'US', countryName: 'United States', tier: 'core', indicators: { creditGdpGap: 2.1, debtServiceRatio: 15.3, creditGrowthYoy: 3.2, creditImpulse: 0.5, propertyPriceTrend: 4.1, policyRate: 5.25, rateDirection: 'pausing', imfGdpGrowth: 2.8, imfFiscalBalance: -6.3, imfCurrentAccount: -3.0, imfGovDebt: 123.0, imfGovRevenue: 30.2 }, phase: 'tightening', phaseLabel: '收水期', confidence: 0.75, confidenceGrade: 'reasonable', reliability: 0.95, dollarTideRiskModifier: 0, dataPeriod: '2025-Q3' },
    { countryCode: 'CN', countryName: 'China', tier: 'core', indicators: { creditGdpGap: -3.5, debtServiceRatio: 20.1, creditGrowthYoy: 8.5, creditImpulse: -1.2, propertyPriceTrend: -2.3, policyRate: 3.45, rateDirection: 'cutting', imfGdpGrowth: 4.6, imfFiscalBalance: -7.1, imfCurrentAccount: 1.5, imfGovDebt: 83.0, imfGovRevenue: 26.8 }, phase: 'deleveraging', phaseLabel: '去杠杆', confidence: 0.60, confidenceGrade: 'reasonable', reliability: 0.70, dollarTideRiskModifier: 1, dataPeriod: '2025-Q3' },
    { countryCode: 'XM', countryName: 'Euro Area', tier: 'core', indicators: { creditGdpGap: -1.2, debtServiceRatio: 12.0, creditGrowthYoy: 1.8, creditImpulse: 0.3, propertyPriceTrend: -0.5, policyRate: 4.50, rateDirection: 'pausing', imfGdpGrowth: 0.8, imfFiscalBalance: -3.5, imfCurrentAccount: 2.8, imfGovDebt: 88.0, imfGovRevenue: 46.1 }, phase: 'tightening', phaseLabel: '收水期', confidence: 0.72, confidenceGrade: 'reasonable', reliability: 0.92, dollarTideRiskModifier: 0, dataPeriod: '2025-Q3' },
    { countryCode: 'JP', countryName: 'Japan', tier: 'core', indicators: { creditGdpGap: 5.2, debtServiceRatio: 8.5, creditGrowthYoy: 2.1, creditImpulse: 0.8, propertyPriceTrend: 3.2, policyRate: 0.10, rateDirection: 'hiking', imfGdpGrowth: 1.0, imfFiscalBalance: -5.8, imfCurrentAccount: 3.5, imfGovDebt: 252.0, imfGovRevenue: 35.4 }, phase: 'leveraging', phaseLabel: '加杠杆', confidence: 0.68, confidenceGrade: 'reasonable', reliability: 0.93, dollarTideRiskModifier: 0, dataPeriod: '2025-Q3' },
    { countryCode: 'GB', countryName: 'United Kingdom', tier: 'important', indicators: { creditGdpGap: -0.8, debtServiceRatio: 14.2, creditGrowthYoy: 1.5, creditImpulse: -0.2, propertyPriceTrend: -1.1, policyRate: 5.25, rateDirection: 'pausing', imfGdpGrowth: 1.3, imfFiscalBalance: -4.5, imfCurrentAccount: -3.1, imfGovDebt: 101.0, imfGovRevenue: 39.2 }, phase: 'tightening', phaseLabel: '收水期', confidence: 0.74, confidenceGrade: 'reasonable', reliability: 0.91, dollarTideRiskModifier: 0, dataPeriod: '2025-Q3' },
    { countryCode: 'KR', countryName: 'South Korea', tier: 'important', indicators: { creditGdpGap: 3.8, debtServiceRatio: 22.5, creditGrowthYoy: 5.2, creditImpulse: -0.5, propertyPriceTrend: -3.1, policyRate: 3.50, rateDirection: 'cutting', imfGdpGrowth: 2.2, imfFiscalBalance: -2.1, imfCurrentAccount: 4.5, imfGovDebt: 55.0, imfGovRevenue: 27.5 }, phase: 'overheating', phaseLabel: '过热期', confidence: 0.65, confidenceGrade: 'reasonable', reliability: 0.88, dollarTideRiskModifier: 1, dataPeriod: '2025-Q3' },
    { countryCode: 'TR', countryName: 'Turkey', tier: 'monitor', indicators: { creditGdpGap: 8.5, debtServiceRatio: 28.3, creditGrowthYoy: 42.0, creditImpulse: 3.2, propertyPriceTrend: 15.0, policyRate: 45.00, rateDirection: 'pausing', imfGdpGrowth: 4.0, imfFiscalBalance: -5.2, imfCurrentAccount: -4.8, imfGovDebt: 42.0, imfGovRevenue: 18.5 }, phase: 'overheating', phaseLabel: '过热期', confidence: 0.45, confidenceGrade: 'speculative', reliability: 0.55, dollarTideRiskModifier: 2, dataPeriod: '2025-Q3' },
    { countryCode: 'SA', countryName: 'Saudi Arabia', tier: 'monitor', indicators: { creditGdpGap: 1.2, debtServiceRatio: 5.1, creditGrowthYoy: 8.0, creditImpulse: 1.5, propertyPriceTrend: 6.2, policyRate: 6.00, rateDirection: 'pausing', imfGdpGrowth: 1.5, imfFiscalBalance: -2.0, imfCurrentAccount: 5.2, imfGovDebt: 26.0, imfGovRevenue: 42.0 }, phase: 'leveraging', phaseLabel: '加杠杆', confidence: 0.50, confidenceGrade: 'reasonable', reliability: 0.60, dollarTideRiskModifier: 0, dataPeriod: '2025-Q3' },
  ],
  globalPhase: 'tightening',
  globalPhaseLabel: '收水期',
  globalPercentile: 62,
  dollarTide: { dxyTrend3m: 1.2, dxyTrend6m: 2.5, fedPolicy: 'pausing', m2Growth: -1.8, yieldSpread: -0.15, tideState: 'ebbing', tideLabel: '退潮', confidence: 0.70 },
  coreSummary: { tier: 'core', dominantPhase: 'tightening', dominantPhaseLabel: '收水期', avgCreditGap: 0.65, warningCount: 0 },
  importantSummary: { tier: 'important', dominantPhase: 'tightening', dominantPhaseLabel: '收水期', avgCreditGap: -0.5, warningCount: 0 },
  monitorSummary: { tier: 'monitor', dominantPhase: 'leveraging', dominantPhaseLabel: '加杠杆', avgCreditGap: 3.2, warningCount: 1 },
  riskAlerts: [
    { countryCode: 'TR', alert: 'Monitor tier overheating + dollar ebbing — EM stress risk', severity: 'warning', confidenceGrade: 'reasonable' },
  ],
  confidence: 0.68,
  calculatedAt: new Date().toISOString(),
  dataPeriod: '2025-Q3',
};

export async function getCreditCycleOverview(): Promise<GlobalCycleOverview> {
  if (!isTauri()) return MOCK_GLOBAL_CYCLE;
  return invoke<GlobalCycleOverview>('get_credit_cycle_overview');
}

export async function getDollarTide(): Promise<DollarTide> {
  if (!isTauri()) return MOCK_GLOBAL_CYCLE.dollarTide;
  return invoke<DollarTide>('get_dollar_tide');
}

// ============================================================
// Deep Analysis / Intelligence types (edict-004 Phase E)
// ============================================================

export interface DeepMotiveAnalysis {
  primaryMotive: string;
  secondaryMotive: string;
  confidence: number;
  confidenceGrade: string;
}

export interface LayerImpact {
  physical: string;
  credit: string;
  dollar: string;
  geopolitical: string;
  sentiment: string;
}

export interface DeepAnalysis {
  clusterId: string;
  clusterTopic: string;
  newsCount: number;
  surface: string;
  connection: string;
  deepAnalysis: DeepMotiveAnalysis;
  layerImpact: LayerImpact;
  keyObservation: string;
  sourceUrls: string[];
  analyzedAt: string;
}

const MOCK_DEEP_ANALYSES: DeepAnalysis[] = [
  {
    clusterId: 'c001',
    clusterTopic: 'US-China Tech Decoupling',
    newsCount: 5,
    surface: 'US restricts chip exports to China, China retaliates with rare earth controls.',
    connection: 'Escalating tech decoupling with direct supply chain implications.',
    deepAnalysis: { primaryMotive: 'Tech supremacy and economic containment', secondaryMotive: 'Domestic political signaling ahead of elections', confidence: 0.72, confidenceGrade: 'reasonable' },
    layerImpact: { physical: 'Rare earth supply disruption', credit: 'Minimal direct credit impact', dollar: 'USD slightly bullish on safe-haven flows', geopolitical: 'Major escalation in bilateral tensions', sentiment: 'Risk-off sentiment in Asia' },
    keyObservation: 'Semiconductor supply chain bifurcation accelerating — watch TSMC capex and inventory data.',
    sourceUrls: ['https://reuters.com/tech/1', 'https://reuters.com/tech/2'],
    analyzedAt: new Date(Date.now() - 3600000).toISOString(),
  },
  {
    clusterId: 'c002',
    clusterTopic: 'Middle East Energy Supply Risk',
    newsCount: 4,
    surface: 'Red Sea shipping disruptions continue, Houthi attacks escalate.',
    connection: 'Persistent threat to global energy transit routes.',
    deepAnalysis: { primaryMotive: 'Regional power projection and proxy warfare', secondaryMotive: 'Oil price floor maintenance by producing nations', confidence: 0.58, confidenceGrade: 'reasonable' },
    layerImpact: { physical: 'Oil supply routes disrupted', credit: 'Energy sector credit spreads widening', dollar: 'Oil priced in USD supports dollar', geopolitical: 'Multi-party regional conflict', sentiment: 'Energy sector fear premium' },
    keyObservation: 'Insurance premiums for Red Sea transit at 10-year high — rerouting adds 7-14 days to Asia-Europe shipping.',
    sourceUrls: ['https://bbc.com/news/1'],
    analyzedAt: new Date(Date.now() - 7200000).toISOString(),
  },
];

export async function getDeepAnalyses(limit?: number): Promise<DeepAnalysis[]> {
  if (!isTauri()) return MOCK_DEEP_ANALYSES;
  return invoke<DeepAnalysis[]>('get_deep_analyses', { limit: limit ?? 10 });
}

// ============================================================
// Game Map / Policy Vector types (edict-004 Phase E)
// ============================================================

export interface PolicyVector {
  id: string;
  name: string;
  nameZh: string;
  activity: number;
  activityLabel: string;
  affectedAssets: string[];
  latestHeadline: string;
  newsCount7d: number;
}

export interface BilateralDynamic {
  id: string;
  name: string;
  nameZh: string;
  tension: number;
  tensionLabel: string;
  recentHeadlines: string[];
  newsCount7d: number;
}

export interface CalendarEvent {
  date: string;
  eventType: string;
  title: string;
  description: string;
  affectedAssets: string[];
  impactDirection: string;
  policyVector: string;
}

export interface Scenario {
  id: string;
  policyVector: string;
  title: string;
  description: string;
  probability: number;
  previousProbability: number;
  changeReason: string;
  assetImpacts: { symbol: string; direction: string; magnitude: string }[];
  confidenceGrade: string;
  updatedAt: string;
}

export interface ScenarioMatrix {
  scenarios: Scenario[];
  activeVectors: string[];
  generatedAt: string;
}

const MOCK_POLICY_VECTORS: PolicyVector[] = [
  { id: 'trade', name: 'Trade Policy', nameZh: '贸易政策', activity: 0.72, activityLabel: 'high', affectedAssets: ['^GSPC', 'USDCNY=X'], latestHeadline: 'US considering additional tariffs on Chinese EVs', newsCount7d: 8 },
  { id: 'tech', name: 'Tech Controls', nameZh: '科技管制', activity: 0.65, activityLabel: 'high', affectedAssets: ['^IXIC', '000001.SS'], latestHeadline: 'New chip export restrictions announced', newsCount7d: 6 },
  { id: 'financial', name: 'Financial/Monetary', nameZh: '金融货币', activity: 0.55, activityLabel: 'moderate', affectedAssets: ['^GSPC', 'DX-Y.NYB', 'GC=F'], latestHeadline: 'Fed signals patience on rate cuts', newsCount7d: 12 },
  { id: 'energy', name: 'Energy Policy', nameZh: '能源政策', activity: 0.30, activityLabel: 'low', affectedAssets: ['CL=F', 'NG=F'], latestHeadline: 'SPR refill continues at steady pace', newsCount7d: 3 },
  { id: 'crypto', name: 'Crypto Regulation', nameZh: '加密监管', activity: 0.45, activityLabel: 'moderate', affectedAssets: ['BTC-USD', 'ETH-USD'], latestHeadline: 'SEC approves spot ETH ETF applications', newsCount7d: 5 },
  { id: 'military', name: 'Military/Security', nameZh: '军事安全', activity: 0.40, activityLabel: 'moderate', affectedAssets: ['GC=F', 'CL=F'], latestHeadline: 'NATO exercises in Baltic Sea expand', newsCount7d: 4 },
];

const MOCK_CALENDAR: CalendarEvent[] = [
  { date: '2026-03-19', eventType: 'fomc_meeting', title: 'FOMC Meeting', description: 'Federal Reserve interest rate decision', affectedAssets: ['^GSPC', 'DX-Y.NYB', 'GC=F', 'BTC-USD'], impactDirection: 'uncertain', policyVector: 'financial' },
  { date: '2026-03-25', eventType: 'sanctions_review', title: 'OFAC Sanctions Review', description: 'Quarterly sanctions compliance update', affectedAssets: ['CL=F'], impactDirection: 'bearish', policyVector: 'military' },
  { date: '2026-04-15', eventType: 'trade_review', title: 'Section 301 Review', description: 'US-China tariff review deadline', affectedAssets: ['^GSPC', 'USDCNY=X'], impactDirection: 'uncertain', policyVector: 'trade' },
];

const MOCK_SCENARIOS: ScenarioMatrix = {
  scenarios: [
    { id: 's1', policyVector: 'trade', title: 'Tariff Escalation', description: 'US raises tariffs to 60% on Chinese goods', probability: 0.35, previousProbability: 0.30, changeReason: 'Recent hawkish rhetoric from USTR', assetImpacts: [{ symbol: '^GSPC', direction: 'bearish', magnitude: 'moderate' }, { symbol: 'USDCNY=X', direction: 'bearish', magnitude: 'large' }], confidenceGrade: 'reasonable', updatedAt: new Date().toISOString() },
    { id: 's2', policyVector: 'trade', title: 'Negotiated De-escalation', description: 'Partial tariff rollback in exchange for purchase commitments', probability: 0.25, previousProbability: 0.28, changeReason: 'Election pressure for economic stability', assetImpacts: [{ symbol: '^GSPC', direction: 'bullish', magnitude: 'moderate' }], confidenceGrade: 'speculative', updatedAt: new Date().toISOString() },
    { id: 's3', policyVector: 'tech', title: 'Full Chip Ban', description: 'Complete semiconductor export ban to China', probability: 0.15, previousProbability: 0.12, changeReason: 'Intelligence reports on military chip usage', assetImpacts: [{ symbol: '^IXIC', direction: 'bearish', magnitude: 'large' }], confidenceGrade: 'speculative', updatedAt: new Date().toISOString() },
  ],
  activeVectors: ['trade', 'tech', 'financial'],
  generatedAt: new Date().toISOString(),
};

export async function getPolicyVectors(): Promise<PolicyVector[]> {
  if (!isTauri()) return MOCK_POLICY_VECTORS;
  return invoke<PolicyVector[]>('get_policy_vectors');
}

const MOCK_BILATERAL_DYNAMICS: BilateralDynamic[] = [
  { id: 'us-cn', name: 'US–China', nameZh: '美中关系', tension: 0.78, tensionLabel: 'strained', recentHeadlines: ['Chip export ban expanded', 'Rare earth retaliation threat'], newsCount7d: 12 },
  { id: 'us-ru', name: 'US–Russia', nameZh: '美俄关系', tension: 0.85, tensionLabel: 'hostile', recentHeadlines: ['New sanctions package', 'Arctic military buildup'], newsCount7d: 7 },
  { id: 'us-me', name: 'US–Middle East', nameZh: '美国-中东', tension: 0.52, tensionLabel: 'cautious', recentHeadlines: ['Red Sea security operations', 'Iran nuclear talks stall'], newsCount7d: 5 },
  { id: 'us-eu', name: 'US–Europe', nameZh: '美欧关系', tension: 0.30, tensionLabel: 'cooperative', recentHeadlines: ['NATO defense spending accord', 'Trade framework renewal'], newsCount7d: 4 },
];

export async function getBilateralDynamics(): Promise<BilateralDynamic[]> {
  if (!isTauri()) return MOCK_BILATERAL_DYNAMICS;
  return invoke<BilateralDynamic[]>('get_bilateral_dynamics');
}

// ============================================================
// News Heatmap — geographic distribution of news by country
// ============================================================

export interface NewsHeatmapEntry {
  countryCode: string;
  newsCount: number;
  avgSentiment: number;
  topKeywords: string[];
  latestTitle: string;
}

const MOCK_NEWS_HEATMAP: NewsHeatmapEntry[] = [
  { countryCode: 'US', newsCount: 8, avgSentiment: 0.45, topKeywords: ['Fed', 'rates', 'inflation'], latestTitle: 'Fed signals rate pause...' },
  { countryCode: 'CN', newsCount: 5, avgSentiment: 0.35, topKeywords: ['trade', 'tariff', 'PBoC'], latestTitle: 'China trade surplus narrows...' },
  { countryCode: 'JP', newsCount: 3, avgSentiment: 0.62, topKeywords: ['BOJ', 'yen', 'Nikkei'], latestTitle: 'BOJ maintains policy...' },
  { countryCode: 'GB', newsCount: 2, avgSentiment: 0.50, topKeywords: ['BOE', 'inflation', 'gilt'], latestTitle: 'UK CPI data released...' },
];

/**
 * Fetch news heatmap aggregated by country for the past N hours.
 * Backend: invoke('get_news_heatmap', { hours }) → Vec<NewsHeatmapEntry>
 * Rust struct uses snake_case; serde(rename_all="camelCase") converts on the wire.
 */
export async function getNewsHeatmap(hours?: number): Promise<NewsHeatmapEntry[]> {
  if (!isTauri()) return MOCK_NEWS_HEATMAP;
  return invoke<NewsHeatmapEntry[]>('get_news_heatmap', { hours: hours ?? 1 });
}

export async function getDecisionCalendar(days?: number): Promise<CalendarEvent[]> {
  if (!isTauri()) return MOCK_CALENDAR;
  return invoke<CalendarEvent[]>('get_decision_calendar', { days: days ?? 90 });
}

export async function getActiveScenarios(): Promise<ScenarioMatrix> {
  if (!isTauri()) return MOCK_SCENARIOS;
  return invoke<ScenarioMatrix>('get_active_scenarios');
}

export async function triggerScenarioUpdate(): Promise<ScenarioMatrix> {
  if (!isTauri()) return MOCK_SCENARIOS;
  return invoke<ScenarioMatrix>('trigger_scenario_update');
}

// ============================================================
// Five-Layer Reasoning types (edict-004 Phase E)
// ============================================================

export interface ReasoningStep {
  step: number;
  layer: string;
  finding: string;
  evidence: string[];
  confidence: number;
}

export interface LayerSummary {
  layer: string;
  summary: string;
  trend: string;  // improving | stable | deteriorating
  keyChange: string;
}

export interface ForwardLook {
  outlook30d: string;
  outlook90d: string;
  keyCatalysts: string[];
  baselineProbability: number;
  baselineScenario: string;
  upsideScenario: string;
  downsideScenario: string;
}

export interface FiveLayerReasoning {
  globalCyclePhase: string;
  globalCyclePhaseZh: string;
  dollarTideState: string;
  dollarTideLabel: string;
  cyclePosition: string;
  monetaryPolicyStage: string;
  sentimentStage: string;
  reasoningSteps: ReasoningStep[];
  turningSignals: TurningSignal[];
  sectorRecommendations: string[];
  tailRisks: string[];
  riskAlerts: string[];
  confidence: number;
  confidenceGrade: string;
  narrative: string;
  timestamp: string;
  layerSummaries: LayerSummary[];
  forwardLook: ForwardLook;
}

export async function getFiveLayerReasoning(): Promise<FiveLayerReasoning | null> {
  if (!isTauri()) return null;
  return invoke<FiveLayerReasoning | null>('get_five_layer_reasoning');
}

export async function triggerFiveLayerReasoning(): Promise<FiveLayerReasoning | null> {
  if (!isTauri()) return null;
  return invoke<FiveLayerReasoning>('trigger_five_layer_reasoning');
}

export function listenFiveLayerUpdated(callback: (r: FiveLayerReasoning) => void): Promise<() => void> {
  if (!isTauri()) return Promise.resolve(() => undefined);
  return listen<FiveLayerReasoning>('five-layer-reasoning-updated', (event) => callback(event.payload));
}

/**
 * Listen for 'deep-analysis-completed' Tauri events (new deep analysis clusters ready).
 * Returns a cleanup function (call on component unmount).
 */
export function listenDeepAnalysisCompleted(callback: () => void): Promise<() => void> {
  if (!isTauri()) return Promise.resolve(() => undefined);
  return listen('deep-analysis-completed', () => callback());
}

/**
 * Listen for 'scenario-updated' Tauri events (scenario matrix recalculated).
 * Returns a cleanup function (call on component unmount).
 */
export function listenScenarioUpdated(callback: () => void): Promise<() => void> {
  if (!isTauri()) return Promise.resolve(() => undefined);
  return listen('scenario-updated', () => callback());
}

// ============================================================
// Daily Brief types (get_daily_brief / daily-brief-updated)
// ============================================================

export interface AttentionItem {
  priority: 'high' | 'medium' | 'low';
  category: string;
  content: string;
  reason: string;
}

export interface SectorAdjustment {
  sector: string;
  weight: number;
  reason: string;
}

export interface QtSuggestion {
  positionBias: number;
  riskMultiplier: number;
  urgency: string;
  sectorAdjustments: SectorAdjustment[];
  reasoning: string;
}

export interface DataSnapshot {
  cyclePhase: string;
  cycleConfidence: number;
  fearGreed: number;
  fedRate: number;
  cpiYoy: number;
  gdpGrowth: number;
  creditSpread: number;
  sp500Trend: number;
  geopoliticalRisk: string;
  geopoliticalEvents: number;
}

export interface DailyBrief {
  id: string;
  headline: string;
  keyContradictions: string[];
  attentionItems: AttentionItem[];
  qtSuggestion: QtSuggestion;
  dataSnapshot: DataSnapshot;
  generatedAt: string;
  model: string;
}

export async function getDailyBrief(): Promise<DailyBrief | null> {
  if (isTauri()) {
    return invoke<DailyBrief | null>('get_daily_brief');
  }
  return null;
}

export function listenDailyBriefUpdated(callback: () => void): Promise<() => void> {
  if (isTauri()) {
    return listen('daily-brief-updated', () => callback());
  }
  return Promise.resolve(() => undefined);
}

// ============================================================
// Alert types (get_alerts / alerts-triggered)
// ============================================================

export interface Alert {
  id: string;
  severity: 'critical' | 'warning' | 'info';
  category: string;
  title: string;
  detail: string;
  indicatorValue: number;
  threshold: number;
  createdAt: string;
}

export async function getAlerts(): Promise<Alert[]> {
  if (isTauri()) {
    return invoke<Alert[]>('get_alerts');
  }
  return [];
}

export function listenAlertsTriggered(callback: (alerts: Alert[]) => void): Promise<() => void> {
  if (isTauri()) {
    return listen<Alert[]>('alerts-triggered', (event) => callback(event.payload));
  }
  return Promise.resolve(() => undefined);
}

// ============================================================
// Indicator trend (get_indicator_trend)
// ============================================================

export interface TrendPoint {
  value: number;
  label: string | null;
  timestamp: string;
}

export async function getIndicatorTrend(indicator: string, days?: number): Promise<TrendPoint[]> {
  if (isTauri()) {
    return invoke<TrendPoint[]>('get_indicator_trend', { indicator, days: days ?? 30 });
  }
  return [];
}
