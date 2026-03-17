import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getMarketData, listenMarketUpdated } from '@services/tauri-bridge';
import type { MarketDataItem } from '@services/tauri-bridge';
import './IndicesPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

function formatPrice(value: number): string {
  return value.toLocaleString('en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

function formatChangePct(value: number | null): string {
  if (value === null) return '--';
  const sign = value >= 0 ? '+' : '';
  return `${sign}${value.toFixed(2)}%`;
}

export function IndicesPanel() {
  const { t } = useTranslation();
  const [items, setItems] = useState<MarketDataItem[]>([]);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const data = await getMarketData();
      // Indices have symbols starting with ^ (e.g. ^SPX, ^NDX)
      const indices = data.filter((d) => d.symbol.startsWith('^'));
      setItems(indices);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }
  }, []);

  useEffect(() => {
    void load();
    let cleanup: (() => void) | null = null;
    const unlistenPromise = listenMarketUpdated(() => void load());
    void unlistenPromise.then((fn) => { cleanup = fn; });
    return () => {
      if (cleanup) { cleanup(); }
      else { void unlistenPromise.then((fn) => fn()); }
    };
  }, [load]);

  if (loadState === 'loading') {
    return (
      <div className="indices-list__state">
        <RefreshCw size={14} className="indices-list__spinner" />
        <span className="indices-list__state-text">{t('state.loadingIndices')}</span>
      </div>
    );
  }

  if (loadState === 'error') {
    return (
      <div className="indices-list__state indices-list__state--error">
        <p className="indices-list__state-text">{t('state.failedIndices')}</p>
        <button className="indices-list__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  if (items.length === 0) {
    return (
      <div className="indices-list__state">
        <p className="indices-list__state-text">{t('state.noIndices')}</p>
      </div>
    );
  }

  return (
    <ul className="indices-list" aria-label="Market indices">
      {items.map((item) => {
        const pct = item.changePct ?? 0;
        return (
          <li key={item.symbol} className="indices-list__row">
            <div className="indices-list__left">
              {/* Backend has no name field — display ticker derived from symbol */}
              <span className="indices-list__name">{item.symbol.replace('^', '')}</span>
              <span className="indices-list__ticker">{item.source}</span>
            </div>
            <div className="indices-list__right">
              <span className="indices-list__price">{formatPrice(item.price)}</span>
              <span
                className={`indices-list__change ${
                  pct >= 0
                    ? 'indices-list__change--up'
                    : 'indices-list__change--down'
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
