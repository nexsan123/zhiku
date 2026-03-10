import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getMarketData } from '@services/tauri-bridge';
import type { MarketDataItem } from '@services/tauri-bridge';
import './ForexPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

/**
 * Convert Yahoo Finance forex symbol to display pair label.
 * e.g. "EURUSD=X" → "EUR/USD", "USDJPY=X" → "USD/JPY"
 */
function symbolToPair(symbol: string): string {
  const base = symbol.replace('=X', '');
  if (base.length === 6) {
    return `${base.slice(0, 3)}/${base.slice(3)}`;
  }
  return base;
}

function formatChangePct(value: number | null): string {
  if (value === null) return '--';
  const sign = value >= 0 ? '+' : '';
  return `${sign}${value.toFixed(2)}%`;
}

export function ForexPanel() {
  const { t } = useTranslation();
  const [items, setItems] = useState<MarketDataItem[]>([]);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const data = await getMarketData();
      // Forex pairs use Yahoo Finance symbol convention: e.g. EURUSD=X
      const forex = data.filter((d) => d.symbol.endsWith('=X'));
      setItems(forex);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (loadState === 'loading') {
    return (
      <div className="forex-panel__state">
        <RefreshCw size={14} className="forex-panel__spinner" />
        <span className="forex-panel__state-text">{t('state.loadingRates')}</span>
      </div>
    );
  }

  if (loadState === 'error') {
    return (
      <div className="forex-panel__state forex-panel__state--error">
        <p className="forex-panel__state-text">{t('state.failedForex')}</p>
        <button className="forex-panel__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  if (items.length === 0) {
    return (
      <div className="forex-panel__state">
        <p className="forex-panel__state-text">{t('state.noForex')}</p>
      </div>
    );
  }

  return (
    <ul className="forex-panel" aria-label="Foreign exchange rates">
      {items.map((item, idx) => {
        const pct = item.changePct ?? 0;
        return (
          <li
            key={item.symbol}
            className={`forex-panel__row ${idx < items.length - 1 ? 'forex-panel__row--divider' : ''}`}
          >
            <span className="forex-panel__pair">{symbolToPair(item.symbol)}</span>
            <div className="forex-panel__right">
              <span className="forex-panel__price">{item.price.toFixed(4)}</span>
              <span
                className={`forex-panel__change ${
                  pct >= 0 ? 'forex-panel__change--up' : 'forex-panel__change--down'
                }`}
              >
                {formatChangePct(item.changePct)}
              </span>
            </div>
          </li>
        );
      })}
    </ul>
  );
}
