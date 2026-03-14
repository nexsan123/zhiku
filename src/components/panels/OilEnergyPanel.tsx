import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getMarketData, listenMarketUpdated } from '@services/tauri-bridge';
import type { MarketDataItem } from '@services/tauri-bridge';
import './OilEnergyPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

const OIL_SYMBOLS = new Set(['CL=F', 'BZ=F']);

// Maps oil symbol to i18n key (replaces hardcoded OIL_LABELS constant)
const OIL_LABEL_KEY: Record<string, string> = {
  'CL=F': 'oil.wti',
  'BZ=F': 'oil.brent',
};

export function OilEnergyPanel() {
  const { t } = useTranslation();
  const [items, setItems] = useState<MarketDataItem[]>([]);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const data = await getMarketData();
      const oil = data.filter((d) => OIL_SYMBOLS.has(d.symbol));
      setItems(oil);
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
      <div className="oil-panel__state">
        <RefreshCw size={14} className="oil-panel__spinner" />
        <span className="oil-panel__state-text">{t('state.loadingPrices')}</span>
      </div>
    );
  }

  if (loadState === 'error') {
    return (
      <div className="oil-panel__state oil-panel__state--error">
        <p className="oil-panel__state-text">{t('state.failedOil')}</p>
        <button className="oil-panel__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  if (items.length === 0) {
    return (
      <div className="oil-panel__state">
        <p className="oil-panel__state-text">{t('state.noOil')}</p>
      </div>
    );
  }

  return (
    <ul className="oil-panel" aria-label="Oil and energy prices">
      {items.map((item, idx) => {
        const pct = item.changePct ?? 0;
        const trendUp = pct >= 0;
        const labelKey = OIL_LABEL_KEY[item.symbol];
        return (
          <li
            key={item.symbol}
            className={`oil-panel__row ${idx < items.length - 1 ? 'oil-panel__row--divider' : ''}`}
          >
            <div className="oil-panel__left">
              <span className="oil-panel__name">
                {labelKey ? t(labelKey) : item.symbol}
              </span>
              <span className="oil-panel__symbol">{item.symbol}</span>
            </div>
            <div className="oil-panel__right">
              <span className="oil-panel__price">${item.price.toFixed(2)}</span>
              <span
                className={`oil-panel__trend ${trendUp ? 'oil-panel__trend--up' : 'oil-panel__trend--down'}`}
                aria-label={trendUp ? t('market.trendUp') : t('market.trendDown')}
              >
                {trendUp ? '▲' : '▼'}
              </span>
            </div>
          </li>
        );
      })}
    </ul>
  );
}
