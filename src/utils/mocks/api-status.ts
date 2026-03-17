import type { ApiServiceStatus } from '@contracts/app-types';

/**
 * Initial API status — all services start as 'checking' with a timestamp.
 * Backend events will overwrite these as real data arrives.
 */
export const MOCK_API_STATUS: Record<string, ApiServiceStatus> = {
  ollama: {
    service: 'ollama',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  groq: {
    service: 'groq',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  claude: {
    service: 'claude',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  fred: {
    service: 'fred',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  yahoo: {
    service: 'yahoo',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  eia: {
    service: 'eia',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  bis: {
    service: 'bis',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  imf: {
    service: 'imf',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  wto: {
    service: 'wto',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  coingecko: {
    service: 'coingecko',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  rss: {
    service: 'rss',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  fear_greed: {
    service: 'fear_greed',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  qt_rest: {
    service: 'qt_rest',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
  qt_ws: {
    service: 'qt_ws',
    status: 'checking',
    lastCheck: new Date().toISOString(),
  },
};
