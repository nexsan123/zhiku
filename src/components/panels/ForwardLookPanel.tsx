import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw, TrendingUp, TrendingDown, Minus, Eye } from 'lucide-react';
import { getFiveLayerReasoning, listenFiveLayerUpdated } from '@services/tauri-bridge';
import type { FiveLayerReasoning, LayerSummary } from '@services/tauri-bridge';
import './ForwardLookPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

/** Format ISO timestamp to readable date + time. e.g. "2026-03-16 02:35 PM" */
function formatTimestamp(iso: string): string {
  try {
    const d = new Date(iso);
    const year = d.getFullYear();
    const month = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    const time = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: true });
    return `${year}-${month}-${day} ${time}`;
  } catch {
    return iso;
  }
}

/** Format probability number (0-1) as percent string with fallback. */
function formatProbability(p: number): string {
  if (!Number.isFinite(p)) return '--';
  return `${Math.round(p * 100)}%`;
}

/** Return CSS trend class for a layer summary trend value. */
function trendClass(trend: string): string {
  if (trend === 'improving') return 'forward-trend--improving';
  if (trend === 'deteriorating') return 'forward-trend--deteriorating';
  return 'forward-trend--stable';
}

/** Return trend icon component for a layer summary trend value. */
function TrendIcon({ trend }: { trend: string }) {
  if (trend === 'improving') return <TrendingUp size={11} />;
  if (trend === 'deteriorating') return <TrendingDown size={11} />;
  return <Minus size={11} />;
}

/** Return layer display name from i18n. */
function useLayerName() {
  const { t } = useTranslation();
  return (layer: string): string => {
    const map: Record<string, string> = {
      physical: t('forward.physical'),
      credit: t('forward.credit'),
      dollar: t('forward.dollar'),
      geopolitical: t('forward.geopolitical'),
      sentiment: t('forward.sentiment'),
    };
    return map[layer] ?? layer;
  };
}

/** Return trend label from i18n. */
function useTrendLabel() {
  const { t } = useTranslation();
  return (trend: string): string => {
    if (trend === 'improving') return t('forward.improving');
    if (trend === 'deteriorating') return t('forward.deteriorating');
    return t('forward.stable');
  };
}

export function ForwardLookPanel() {
  const { t } = useTranslation();
  const getLayerName = useLayerName();
  const getTrendLabel = useTrendLabel();

  const [data, setData] = useState<FiveLayerReasoning | null>(null);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const result = await getFiveLayerReasoning();
      setData(result);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }
  }, []);

  useEffect(() => {
    void load();
    let cleanup: (() => void) | null = null;
    const p = listenFiveLayerUpdated((updated) => setData(updated));
    void p.then((fn) => { cleanup = fn; });
    return () => {
      if (cleanup) { cleanup(); }
      else { void p.then((fn) => fn()); }
    };
  }, [load]);

  // ---- Loading state ----
  if (loadState === 'loading') {
    return (
      <div className="forward-panel__state">
        <RefreshCw size={14} className="forward-panel__spinner" />
        <span className="forward-panel__state-text">{t('forward.loading')}</span>
      </div>
    );
  }

  // ---- Error state ----
  if (loadState === 'error') {
    return (
      <div className="forward-panel__state forward-panel__state--error">
        <p className="forward-panel__state-text">{t('forward.failed')}</p>
        <button className="forward-panel__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  // ---- Empty state: no data yet, or backend hasn't deployed new fields ----
  // Defensive: data may be null (backend returned nothing) or forwardLook may be
  // absent on older backend builds that don't yet include the new fields.
  const forwardLook = (data as (FiveLayerReasoning & { forwardLook?: unknown }) | null)?.forwardLook as FiveLayerReasoning['forwardLook'] | undefined;
  const layerSummaries: LayerSummary[] = Array.isArray((data as (FiveLayerReasoning & { layerSummaries?: unknown }) | null)?.layerSummaries)
    ? (data!.layerSummaries as LayerSummary[])
    : [];

  if (!data || !forwardLook) {
    return (
      <div className="forward-panel__state">
        <Eye size={24} />
        <p className="forward-panel__state-text">{t('forward.noData')}</p>
      </div>
    );
  }

  return (
    <div className="forward-panel">

      {/* ---- Section 1: Five-Layer Overview ---- */}
      {layerSummaries.length > 0 && (
        <div className="forward-section">
          <h4 className="forward-section__title">{t('forward.layerOverview')}</h4>
          <div className="forward-layers">
            {layerSummaries.map((ls, i) => (
              <div key={ls.layer ?? i} className="forward-layer-row">
                <span className="forward-layer-row__name">
                  {getLayerName(ls.layer)}
                </span>
                <div className="forward-layer-row__body">
                  <span className="forward-layer-row__summary">
                    {ls.summary}
                  </span>
                  {ls.keyChange && (
                    <span className="forward-layer-row__change">{ls.keyChange}</span>
                  )}
                </div>
                <span className={`forward-layer-row__trend ${trendClass(ls.trend)}`}>
                  <TrendIcon trend={ls.trend} />
                  {getTrendLabel(ls.trend)}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* ---- Section 2: Forward Look ---- */}
      <div className="forward-section">
        <h4 className="forward-section__title">{t('forward.forwardLook')}</h4>

        {/* 30d / 90d outlook */}
        <div className="forward-outlook">
          {forwardLook.outlook30d && (
            <div className="forward-outlook__item">
              <div className="forward-outlook__label">{t('forward.outlook30d')}</div>
              <p className="forward-outlook__text">{forwardLook.outlook30d}</p>
            </div>
          )}
          {forwardLook.outlook90d && (
            <div className="forward-outlook__item">
              <div className="forward-outlook__label">{t('forward.outlook90d')}</div>
              <p className="forward-outlook__text">{forwardLook.outlook90d}</p>
            </div>
          )}
        </div>

        {/* Key catalysts */}
        {Array.isArray(forwardLook.keyCatalysts) && forwardLook.keyCatalysts.length > 0 && (
          <div className="forward-catalysts-wrap">
            <div className="forward-catalysts-wrap__label">{t('forward.catalysts')}</div>
            <div className="forward-catalysts">
              {forwardLook.keyCatalysts.map((c, i) => (
                <span key={i} className="forward-catalyst-tag">{c}</span>
              ))}
            </div>
          </div>
        )}

        {/* Three scenarios */}
        <div className="forward-scenarios">
          {/* Baseline */}
          <div className="forward-scenario forward-scenario--baseline">
            <div className="forward-scenario__header">
              <span className="forward-scenario__label">{t('forward.baseline')}</span>
              <span className="forward-scenario__prob-badge">
                {formatProbability(forwardLook.baselineProbability)}
              </span>
            </div>
            <p className="forward-scenario__text">{forwardLook.baselineScenario}</p>
          </div>

          {/* Upside */}
          <div className="forward-scenario forward-scenario--upside">
            <div className="forward-scenario__header">
              <span className="forward-scenario__label">{t('forward.upside')}</span>
            </div>
            <p className="forward-scenario__text">{forwardLook.upsideScenario}</p>
          </div>

          {/* Downside */}
          <div className="forward-scenario forward-scenario--downside">
            <div className="forward-scenario__header">
              <span className="forward-scenario__label">{t('forward.downside')}</span>
            </div>
            <p className="forward-scenario__text">{forwardLook.downsideScenario}</p>
          </div>
        </div>
      </div>

      {/* ---- Timestamp ---- */}
      <div className="forward-timestamp">
        <span>{t('forward.lastUpdate')}: {formatTimestamp(data.timestamp)}</span>
      </div>

    </div>
  );
}
