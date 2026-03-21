import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getMacroData, listenMacroUpdated } from '@services/tauri-bridge';
import type { MacroDataItem } from '@services/tauri-bridge';
import { TrendIndicator } from '@components/common/TrendIndicator';
import './FredPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

interface FredIndicatorConfig {
  /** FRED series ID as stored by the backend (e.g. "FEDFUNDS"). */
  seriesId: string;
  /** i18n key for the indicator display name. */
  nameKey: string;
  unit: string;
  /** Value at or above this → warning color. null = no threshold. */
  warnHigh: number | null;
  /** Value at or above this → error color. null = no threshold. */
  dangerHigh: number | null;
  /** Backend indicator name for trend sparkline. null = no sparkline. */
  trendIndicator: string | null;
}

const FRED_INDICATORS: FredIndicatorConfig[] = [
  { seriesId: 'FEDFUNDS', nameKey: 'fred.fedfunds',     unit: '%', warnHigh: 5.5, dangerHigh: 7.0, trendIndicator: 'fed_rate' },
  { seriesId: 'CPIAUCSL', nameKey: 'fred.cpi',          unit: '%', warnHigh: 4.0, dangerHigh: 6.0, trendIndicator: 'cpi_yoy' },
  { seriesId: 'UNRATE',   nameKey: 'fred.unemployment',  unit: '%', warnHigh: 5.0, dangerHigh: 7.0, trendIndicator: null },
  { seriesId: 'GDP',      nameKey: 'fred.gdp',          unit: 'T', warnHigh: null, dangerHigh: null, trendIndicator: 'gdp_growth' },
  { seriesId: 'M2SL',     nameKey: 'fred.m2',           unit: 'B', warnHigh: null, dangerHigh: null, trendIndicator: null },
];

type ValueStatus = 'normal' | 'warning' | 'danger' | 'na';

function getValueStatus(value: number, config: FredIndicatorConfig): ValueStatus {
  if (config.dangerHigh !== null && value >= config.dangerHigh) return 'danger';
  if (config.warnHigh !== null && value >= config.warnHigh) return 'warning';
  return 'normal';
}

function formatValue(value: number | undefined, unit: string): string {
  if (value === undefined || isNaN(value)) return '--';
  if (unit === 'T') return `$${(value / 1000).toFixed(1)}T`;
  if (unit === 'B') return `$${value.toFixed(0)}B`;
  return `${value.toFixed(2)}${unit}`;
}

function formatPeriod(period: string | null | undefined): string {
  if (!period) return '--';
  return period;
}

export function FredPanel() {
  const { t } = useTranslation();
  const [rows, setRows] = useState<MacroDataItem[]>([]);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const data = await getMacroData();
      setRows(data);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }
  }, []);

  useEffect(() => {
    void load();
    const timer = setInterval(() => void load(), 5 * 60 * 1000);
    let cleanup: (() => void) | null = null;
    const unlistenPromise = listenMacroUpdated(() => void load());
    void unlistenPromise.then((fn) => { cleanup = fn; });
    return () => {
      clearInterval(timer);
      if (cleanup) { cleanup(); } else { void unlistenPromise.then((fn) => fn()); }
    };
  }, [load]);

  if (loadState === 'loading') {
    return (
      <div className="fred__state">
        <RefreshCw size={14} className="fred__spinner" />
        <span className="fred__state-text">{t('fred.loading')}</span>
      </div>
    );
  }

  if (loadState === 'error') {
    return (
      <div className="fred__state fred__state--error">
        <p className="fred__state-text">{t('fred.failed')}</p>
        <button className="fred__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  return (
    <div className="fred">
      <ul className="fred__list" aria-label="FRED macro indicators">
        {FRED_INDICATORS.map((config) => {
          const row = rows.find((r) => r.indicator === config.seriesId);
          const status: ValueStatus = row ? getValueStatus(row.value, config) : 'na';
          return (
            <li key={config.seriesId} className="fred__row">
              <div className="fred__left">
                <span className="fred__name">{t(config.nameKey)}</span>
                <span className="fred__period">
                  {row ? formatPeriod(row.period) : t('fred.noData')}
                </span>
              </div>
              <div className="fred__spark">
                {config.trendIndicator && (
                  <TrendIndicator
                    indicator={config.trendIndicator}
                    days={30}
                    width={80}
                    height={28}
                  />
                )}
              </div>
              <div className="fred__right">
                <span className={`fred__value fred__value--${status}`}>
                  {row ? formatValue(row.value, config.unit) : '--'}
                </span>
              </div>
            </li>
          );
        })}
      </ul>

      <div className="fred__footer">
        <span className="fred__footer-note">{t('fred.staticNote')}</span>
      </div>
    </div>
  );
}
