import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getMacroData, isTauri, MOCK_FEAR_GREED } from '@services/tauri-bridge';
import './FearGreedPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

interface FearGreedData {
  value: number;
}

/** Map 0–100 to i18n key and CSS modifier. */
function getCategory(value: number): { labelKey: string; modifier: string } {
  if (value <= 25) return { labelKey: 'fearGreed.extremeFear', modifier: 'extreme-fear' };
  if (value <= 45) return { labelKey: 'fearGreed.fear', modifier: 'fear' };
  if (value <= 55) return { labelKey: 'fearGreed.neutral', modifier: 'neutral' };
  if (value <= 75) return { labelKey: 'fearGreed.greed', modifier: 'greed' };
  return { labelKey: 'fearGreed.extremeGreed', modifier: 'extreme-greed' };
}

export function FearGreedPanel() {
  const { t } = useTranslation();
  const [data, setData] = useState<FearGreedData | null>(null);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const macroRows = await getMacroData();
      const fgRow = macroRows.find((r) => r.indicator === 'fear_greed_index');
      if (fgRow) {
        setData({ value: fgRow.value });
      } else {
        // Indicator not yet in DB — use mock as neutral fallback (only use .value)
        setData({ value: MOCK_FEAR_GREED.value });
      }
      setLoadState('loaded');
    } catch {
      if (isTauri()) {
        // In Tauri env: a real error occurred — surface it so user can retry
        setLoadState('error');
      } else {
        // In browser dev env: defensive fallback (getMacroData() returns mock, won't throw)
        setData({ value: MOCK_FEAR_GREED.value });
        setLoadState('loaded');
      }
    }
  }, []);

  useEffect(() => {
    void load();
    const timer = setInterval(() => void load(), 5 * 60 * 1000);
    return () => clearInterval(timer);
  }, [load]);

  if (loadState === 'loading') {
    return (
      <div className="fear-greed__state">
        <RefreshCw size={14} className="fear-greed__spinner" />
        <span className="fear-greed__state-text">{t('state.loadingIndex')}</span>
      </div>
    );
  }

  if (loadState === 'error') {
    return (
      <div className="fear-greed__state fear-greed__state--error">
        <p className="fear-greed__state-text">{t('state.failedIndex')}</p>
        <button className="fear-greed__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  if (!data) return null;

  const { labelKey, modifier } = getCategory(data.value);
  const clampedValue = Math.min(100, Math.max(0, data.value));

  return (
    <div className="fear-greed">
      <div className="fear-greed__header">
        <span className={`fear-greed__value fear-greed__value--${modifier}`}>
          {clampedValue}
        </span>
        <span className={`fear-greed__label fear-greed__label--${modifier}`}>
          {t(labelKey)}
        </span>
      </div>

      {/* Gradient progress bar */}
      <div
        className="fear-greed__bar-track"
        role="meter"
        aria-valuenow={clampedValue}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={`Fear & Greed Index: ${clampedValue} — ${t(labelKey)}`}
      >
        <div className="fear-greed__bar-gradient" />
        <div
          className="fear-greed__bar-cursor"
          style={{ left: `${clampedValue}%` }}
          aria-hidden="true"
        />
      </div>

      <div className="fear-greed__scale">
        <span>{t('fearGreed.fear')}</span>
        <span>{t('fearGreed.neutral')}</span>
        <span>{t('fearGreed.greed')}</span>
      </div>
    </div>
  );
}
