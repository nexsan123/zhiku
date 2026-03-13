import type { ApiServiceStatus } from '@contracts/app-types';

/**
 * Mock API 状态数据 — Phase 4 占位符
 * 联调时由 Tauri event 推送替换
 */
export const MOCK_API_STATUS: Record<string, ApiServiceStatus> = {
  ollama: {
    service: 'ollama',
    status: 'online',
    lastCheck: new Date().toISOString(),
    responseMs: 45,
  },
  groq: {
    service: 'groq',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  claude: {
    service: 'claude',
    status: 'offline',
    lastCheck: new Date().toISOString(),
    lastError: 'API key not configured',
  },
  fred: {
    service: 'fred',
    status: 'online',
    lastCheck: new Date().toISOString(),
    responseMs: 120,
  },
  yahoo: {
    service: 'yahoo',
    status: 'online',
    lastCheck: new Date().toISOString(),
    responseMs: 88,
  },
  eia: {
    service: 'eia',
    status: 'idle',
  },
  bis: {
    service: 'bis',
    status: 'idle',
  },
  imf: {
    service: 'imf',
    status: 'idle',
  },
  wto: {
    service: 'wto',
    status: 'idle',
  },
  coingecko: {
    service: 'coingecko',
    status: 'idle',
  },
};
