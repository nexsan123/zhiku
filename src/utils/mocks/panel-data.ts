/**
 * Mock 面板数据 — Phase 4 占位符
 * 联调时由 Tauri command / event 替换
 */

export interface MockNewsItem {
  id: string;
  title: string;
  source: string;
  timeAgo: string;
  category: 'geopolitical' | 'macro' | 'market' | 'corporate';
}

export const MOCK_NEWS: MockNewsItem[] = [
  {
    id: '1',
    title: 'Fed Signals Potential Rate Cut in H2 2026 as Inflation Eases',
    source: 'Reuters',
    timeAgo: '3m ago',
    category: 'macro',
  },
  {
    id: '2',
    title: 'China PMI Beats Expectations at 51.2, Signaling Manufacturing Recovery',
    source: 'Bloomberg',
    timeAgo: '18m ago',
    category: 'market',
  },
  {
    id: '3',
    title: 'ECB Holds Rates Steady, Warns of Persistent Services Inflation',
    source: 'FT',
    timeAgo: '1h ago',
    category: 'macro',
  },
  {
    id: '4',
    title: 'Oil Drops 2% on OPEC+ Output Uncertainty and USD Strength',
    source: 'WSJ',
    timeAgo: '2h ago',
    category: 'market',
  },
  {
    id: '5',
    title: 'US Treasury 10Y Yield Crosses 4.5% on Strong Jobs Data',
    source: 'CNBC',
    timeAgo: '3h ago',
    category: 'macro',
  },
];

export interface MockSignal {
  name: string;
  bullish: boolean | null;  // matches MarketRadarSignal.bullish (Option<bool>)
  detail: string;
}

export const MOCK_SIGNALS: MockSignal[] = [
  { name: 'Fed Policy', bullish: true, detail: 'Dovish pivot signals' },
  { name: 'Credit Spreads', bullish: null, detail: 'Range-bound, neutral' },
  { name: 'Yield Curve', bullish: false, detail: 'Inverted 2s10s' },
  { name: 'PMI Composite', bullish: true, detail: '52.1, expansion' },
  { name: 'Dollar Index', bullish: false, detail: 'DXY rising' },
  { name: 'VIX Level', bullish: null, detail: '18, neutral territory' },
  { name: 'Earnings Revisions', bullish: true, detail: '+3.2% revision' },
];

export const MOCK_MARKET_VERDICT = 'BUY';

export interface MockIndex {
  name: string;
  ticker: string;
  price: string;
  change: string;
  changePercent: string;
  positive: boolean;
}

export const MOCK_INDICES: MockIndex[] = [
  { name: 'S&P 500', ticker: 'SPX', price: '5,892.34', change: '+24.6', changePercent: '+0.42%', positive: true },
  { name: 'NASDAQ', ticker: 'NDX', price: '21,456.78', change: '+112.3', changePercent: '+0.53%', positive: true },
  { name: 'Dow Jones', ticker: 'DJI', price: '43,210.55', change: '-88.4', changePercent: '-0.20%', positive: false },
  { name: 'CSI 300', ticker: 'CSI3', price: '3,921.10', change: '+45.2', changePercent: '+1.17%', positive: true },
  { name: 'Nikkei 225', ticker: 'NKY', price: '38,654.20', change: '-124.5', changePercent: '-0.32%', positive: false },
];

// ============================================================
// Forex mock data
// ============================================================
export interface MockForexPair {
  pair: string;
  price: string;
  changePercent: string;
  positive: boolean;
}

export const MOCK_FOREX: MockForexPair[] = [
  { pair: 'EUR/USD', price: '1.0834', changePercent: '+0.12%', positive: true },
  { pair: 'USD/JPY', price: '149.52', changePercent: '-0.23%', positive: false },
  { pair: 'GBP/USD', price: '1.2641', changePercent: '+0.05%', positive: true },
  { pair: 'USD/CNY', price: '7.2485', changePercent: '-0.08%', positive: false },
];

// ============================================================
// Oil & Energy mock data
// ============================================================
export interface MockOilData {
  name: string;
  symbol: string;
  price: string;
  trend: 'up' | 'down';
}

export const MOCK_OIL: MockOilData[] = [
  { name: 'WTI Crude', symbol: 'CL=F', price: '$72.43', trend: 'up' },
  { name: 'Brent Crude', symbol: 'BZ=F', price: '$76.21', trend: 'down' },
];

// ============================================================
// Crypto mock data
// ============================================================
export interface MockCryptoAsset {
  name: string;
  symbol: string;
  price: string;
  changePercent: string;
  positive: boolean;
}

export interface MockStablecoin {
  symbol: string;
  peg: 'on-peg' | 'slight-depeg' | 'depegged';
}

export const MOCK_CRYPTO: MockCryptoAsset[] = [
  { name: 'Bitcoin', symbol: 'BTC', price: '$67,452', changePercent: '+2.34%', positive: true },
  { name: 'Ethereum', symbol: 'ETH', price: '$3,521', changePercent: '-0.87%', positive: false },
];

export const MOCK_STABLECOINS: MockStablecoin[] = [
  { symbol: 'USDT', peg: 'on-peg' },
  { symbol: 'USDC', peg: 'on-peg' },
  { symbol: 'DAI', peg: 'slight-depeg' },
];

// ============================================================
// Fear & Greed mock data
// ============================================================
export interface MockFearGreed {
  value: number; // 0–100
  label: string;
}

export const MOCK_FEAR_GREED: MockFearGreed = { value: 72, label: 'Greed' };
