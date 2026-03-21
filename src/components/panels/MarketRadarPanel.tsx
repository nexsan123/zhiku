import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getMarketRadar, listenMarketUpdated } from '@services/tauri-bridge';
import type { MarketRadarData } from '@services/tauri-bridge';
import { TrendIndicator } from '@components/common/TrendIndicator';
import './MarketRadarPanel.css';

/**
 * Map signal names to backend indicator IDs for sparkline display.
 * Only signals listed here will get a sparkline; others show nothing.
 */
const SIGNAL_TREND_MAP: Record<string, string> = {
  'VIX Level': 'vix',
  'S&P 500': 'sp500_price',
};

type LoadState = 'loading' | 'loaded' | 'error';

/** Derive CSS modifier and i18n key from bullish: boolean | null */
function signalKey(bullish: boolean | null): { modifier: string; labelKey: string } {
  if (bullish === true) return { modifier: 'bullish', labelKey: 'signal.bullish' };
  if (bullish === false) return { modifier: 'bearish', labelKey: 'signal.bearish' };
  return { modifier: 'neutral', labelKey: 'signal.neutral' };
}

export function MarketRadarPanel() {
  const { t } = useTranslation();
  const [data, setData] = useState<MarketRadarData | null>(null);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const result = await getMarketRadar();
      setData(result);
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
      <div className="market-radar__state">
        <RefreshCw size={14} className="market-radar__spinner" />
        <span className="market-radar__state-text">{t('state.loadingSignals')}</span>
      </div>
    );
  }

  if (loadState === 'error' || !data) {
    return (
      <div className="market-radar__state market-radar__state--error">
        <p className="market-radar__state-text">{t('state.failedRadar')}</p>
        <button className="market-radar__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  if (data.signals.length === 0) {
    return (
      <div className="market-radar__state">
        <p className="market-radar__state-text">{t('state.noSignals')}</p>
      </div>
    );
  }

  const bullishCount = data.signals.filter((s) => s.bullish === true).length;
  const bearishCount = data.signals.filter((s) => s.bullish === false).length;

  return (
    <div className="market-radar">
      <ul className="market-radar__signals" aria-label="Market signals">
        {data.signals.map((signal) => {
          const { modifier, labelKey } = signalKey(signal.bullish);
          const trendKey = SIGNAL_TREND_MAP[signal.name];
          return (
            <li key={signal.name} className="market-radar__row">
              <div className="market-radar__signal-left">
                <span className="market-radar__name">{signal.name}</span>
                <span className="market-radar__detail">{signal.detail}</span>
              </div>
              {trendKey ? (
                <div className="market-radar__spark">
                  <TrendIndicator indicator={trendKey} days={30} width={72} height={24} />
                </div>
              ) : (
                <div className="market-radar__spark" aria-hidden="true" />
              )}
              <span className={`market-radar__verdict market-radar__verdict--${modifier}`}>
                {t(labelKey)}
              </span>
            </li>
          );
        })}
      </ul>

      <div className="market-radar__footer">
        <div className="market-radar__counts">
          <span className="market-radar__count--bull">▲ {bullishCount}</span>
          <span className="market-radar__count--bear">▼ {bearishCount}</span>
        </div>
        <div
          className={`market-radar__overall market-radar__overall--${data.verdict.toLowerCase()}`}
          aria-label={`${t('signal.verdict')}${data.verdict}`}
        >
          {t('signal.verdict')}<strong>{data.verdict}</strong>
        </div>
      </div>
    </div>
  );
}
