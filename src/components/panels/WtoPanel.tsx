import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getMacroData } from '@services/tauri-bridge';
import type { MacroDataItem } from '@services/tauri-bridge';
import './WtoPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

// Static curated trade friction events shown when WTO API data is unavailable.
const TRADE_EVENT_KEYS = [
  'wto.event1',
  'wto.event2',
  'wto.event3',
  'wto.event4',
] as const;

function formatTradeValue(item: MacroDataItem): string {
  // WTO merchandise trade values are typically in USD billions
  const v = item.value;
  if (isNaN(v)) return '--';
  if (v >= 1000) return `$${(v / 1000).toFixed(1)}T`;
  return `$${v.toFixed(0)}B`;
}

export function WtoPanel() {
  const { t } = useTranslation();
  const [exportsRow, setExportsRow] = useState<MacroDataItem | null>(null);
  const [importsRow, setImportsRow] = useState<MacroDataItem | null>(null);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const data = await getMacroData();
      setExportsRow(data.find((d) => d.indicator === 'WTO_MERCH_EXPORTS') ?? null);
      setImportsRow(data.find((d) => d.indicator === 'WTO_MERCH_IMPORTS') ?? null);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  // ---- Loading state ----
  if (loadState === 'loading') {
    return (
      <div className="wto__state">
        <RefreshCw size={14} className="wto__spinner" />
        <span className="wto__state-text">{t('wto.loading')}</span>
      </div>
    );
  }

  // ---- Error state ----
  if (loadState === 'error') {
    return (
      <div className="wto__state wto__state--error">
        <p className="wto__state-text">{t('wto.failed')}</p>
        <button className="wto__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  // ---- Loaded ----
  const hasLiveData = exportsRow !== null || importsRow !== null;

  return (
    <div className="wto">
      {/* Global trade volume card — live data if available, else static */}
      <div className="wto__overview-card">
        <span className="wto__overview-label">{t('wto.tradeVolume')}</span>
        {hasLiveData ? (
          <div className="wto__overview-live">
            {exportsRow && (
              <span className="wto__overview-value">
                {t('wto.exports')}: {formatTradeValue(exportsRow)}
              </span>
            )}
            {importsRow && (
              <span className="wto__overview-value">
                {t('wto.imports')}: {formatTradeValue(importsRow)}
              </span>
            )}
          </div>
        ) : (
          <span className="wto__overview-value">{t('wto.tradeVolumeValue')}</span>
        )}
      </div>

      {/* Trade friction events */}
      <div className="wto__section">
        <h4 className="wto__section-title">{t('wto.tradeEvents')}</h4>
        <ul className="wto__events" aria-label="Trade friction events">
          {TRADE_EVENT_KEYS.map((key) => (
            <li key={key} className="wto__event-item">
              <span className="wto__event-bullet" aria-hidden="true">▸</span>
              <span className="wto__event-text">{t(key)}</span>
            </li>
          ))}
        </ul>
      </div>

      <div className="wto__footer">
        <span className="wto__footer-note">
          {hasLiveData ? t('wto.staticNote') : t('wto.noApiKey')}
        </span>
      </div>
    </div>
  );
}
