// ============================================================
// Market Context Contract — 智库 → QuantTerminal
// 智库 writes MarketContext rows to a shared SQLite file.
// QuantTerminal reads them via 3-second mtime polling.
// ============================================================

export type EventRisk = 'none' | 'low' | 'medium' | 'high' | 'critical';

export interface CalendarEvent {
  name: string;
  scheduledAt: string;
  impact: 'low' | 'medium' | 'high';
  affectedSymbols: string[];
}

/** Market context row written by 智库 */
export interface MarketContext {
  id: number;
  timestamp: string;
  regime: string;
  eventRisk: EventRisk;
  vixLevel: number | null;
  sectorBias: string | null;
  newsSentiment: number | null;
  upcomingEvents: CalendarEvent[];
  summary: string;
  source: string;
  schemaVersion: number;
}

// --- WebSocket Event Types (ws://localhost:9600) ---

export type WsEventType = 'signal.new' | 'macro.update' | 'cycle.update' | 'alert.p0';

export interface WsMessage {
  event: WsEventType;
  payload: unknown;
  timestamp: string;
}

// --- REST API Endpoints (http://localhost:9601) ---
// GET /api/v1/signals      → recent signal events
// GET /api/v1/macro-score   → latest macro indicators
// GET /api/v1/market-radar  → 7-signal radar verdict
// GET /api/v1/ai-brief      → AI brief summaries
// GET /api/v1/cycle          → latest cycle reasoning

// --- Shared SQLite Schema (智库 creates market_context.db) ---
// CREATE TABLE IF NOT EXISTS market_context (
//     id INTEGER PRIMARY KEY AUTOINCREMENT,
//     timestamp TEXT NOT NULL,
//     regime TEXT NOT NULL DEFAULT 'neutral',
//     event_risk TEXT NOT NULL DEFAULT 'none',
//     vix_level REAL,
//     sector_bias TEXT,
//     news_sentiment REAL,
//     upcoming_events TEXT DEFAULT '[]',
//     summary TEXT NOT NULL DEFAULT '',
//     source TEXT NOT NULL DEFAULT 'zhiku',
//     schema_version INTEGER NOT NULL DEFAULT 1
// );
