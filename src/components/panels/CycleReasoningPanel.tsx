import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getCycleIndicators, getFiveLayerReasoning, listenFiveLayerUpdated } from '@services/tauri-bridge';
import type { CycleIndicators, FiveLayerReasoning } from '@services/tauri-bridge';
import './CycleReasoningPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

/** Format ISO timestamp to full date + AM/PM time. e.g. "2026-03-16 02:35 PM" */
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

/** Map cycle position / phase key to i18n cycle key. */
function phaseKey(phase: string): string {
  const map: Record<string, string> = {
    early_expansion: 'cycle.early_expansion',
    mid_expansion: 'cycle.mid_expansion',
    late_expansion: 'cycle.late_expansion',
    recession: 'cycle.recession',
    recovery: 'cycle.recovery',
    hiking: 'cycle.hiking',
    pausing: 'cycle.pausing',
    cutting: 'cycle.cutting',
    qe: 'cycle.qe',
    qt: 'cycle.qt',
    panic: 'cycle.panic',
    fear: 'cycle.fear',
    caution: 'cycle.caution',
    neutral: 'cycle.neutral',
    optimism: 'cycle.optimism',
    euphoria: 'cycle.euphoria',
    bull: 'cycle.bull',
    bear: 'cycle.bear',
    correction: 'cycle.correction',
    low: 'cycle.low',
    moderate: 'cycle.moderate_risk',
    elevated: 'cycle.elevated',
    high: 'cycle.high',
    critical: 'cycle.critical',
    hawkish: 'cycle.hawkish',
    dovish: 'cycle.dovish',
    unavailable: 'cycle.unavailable',
    unknown: 'cycle.unknown',
  };
  return map[phase] ?? 'cycle.unknown';
}

/** Return CSS modifier class for a phase/position string. */
function phaseClass(phase: string): string {
  if (['early_expansion', 'mid_expansion', 'recovery', 'bull', 'optimism', 'cutting', 'qe', 'dovish'].includes(phase)) {
    return 'cycle-phase--positive';
  }
  if (['late_expansion', 'pausing', 'neutral', 'caution', 'moderate'].includes(phase)) {
    return 'cycle-phase--neutral';
  }
  if (['recession', 'bear', 'correction', 'panic', 'fear', 'elevated', 'high', 'critical', 'hiking', 'qt', 'hawkish'].includes(phase)) {
    return 'cycle-phase--negative';
  }
  return 'cycle-phase--unknown';
}

/** Return CSS modifier for confidence level. */
function confidenceClass(confidence: number): string {
  if (confidence >= 0.7) return 'cycle-confidence--high';
  if (confidence >= 0.5) return 'cycle-confidence--medium';
  return 'cycle-confidence--low';
}

/** Map reasoning step layer string to i18n key. */
function layerI18nKey(layer: string): string {
  const map: Record<string, string> = {
    physical:     'cycle.physical',
    credit:       'cycle.creditLayer',
    dollar:       'cycle.dollarLayer',
    geopolitical: 'cycle.geopoliticalLayer',
    sentiment:    'cycle.sentimentLayer',
  };
  return map[layer] ?? '';
}

export function CycleReasoningPanel() {
  const { t } = useTranslation();
  const [indicators, setIndicators] = useState<CycleIndicators | null>(null);
  const [reasoning, setReasoning] = useState<FiveLayerReasoning | null>(null);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const [ind, rea] = await Promise.all([getCycleIndicators(), getFiveLayerReasoning()]);
      setIndicators(ind);
      setReasoning(rea);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }
  }, []);

  useEffect(() => {
    void load();
    let cleanup: (() => void) | null = null;
    const unlistenPromise = listenFiveLayerUpdated((r) => { setReasoning(r); });
    void unlistenPromise.then((fn) => { cleanup = fn; });
    return () => {
      if (cleanup) { cleanup(); }
      else { void unlistenPromise.then((fn) => fn()); }
    };
  }, [load]);

  // ---- Loading state ----
  if (loadState === 'loading') {
    return (
      <div className="cycle-panel__state">
        <RefreshCw size={14} className="cycle-panel__spinner" />
        <span className="cycle-panel__state-text">{t('cycle.loading')}</span>
      </div>
    );
  }

  // ---- Error state ----
  if (loadState === 'error') {
    return (
      <div className="cycle-panel__state cycle-panel__state--error">
        <p className="cycle-panel__state-text">{t('cycle.failed')}</p>
        <button className="cycle-panel__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  // ---- Empty state ----
  if (!indicators || !reasoning) {
    return (
      <div className="cycle-panel__state">
        <p className="cycle-panel__state-text">{t('cycle.noData')}</p>
        <button className="cycle-panel__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  const confidencePct = Math.round(reasoning.confidence * 100);

  return (
    <div className="cycle-panel">
      {/* ---- Header: cycle position + confidence ---- */}
      <div className="cycle-header">
        <div className="cycle-header__left">
          <span className="cycle-header__label">{t('cycle.position')}</span>
          <span className={`cycle-phase ${phaseClass(reasoning.cyclePosition)}`}>
            {t(phaseKey(reasoning.cyclePosition))}
          </span>
        </div>
        <div className="cycle-header__right">
          <span className="cycle-header__label">{t('cycle.confidence')}</span>
          <span className={`cycle-confidence ${confidenceClass(reasoning.confidence)}`}>
            {confidencePct}%
          </span>
        </div>
      </div>

      {/* ---- Context subheader: global phase + dollar tide + grade ---- */}
      <div className="cycle-context">
        <span className="cycle-context__item">
          <span className="cycle-context__label">{t('cycle.globalPhase')}</span>
          <span className="cycle-context__value">{reasoning.globalCyclePhaseZh}</span>
        </span>
        <span className="cycle-context__item">
          <span className="cycle-context__label">{t('cycle.dollarTide')}</span>
          <span className="cycle-context__value">{reasoning.dollarTideLabel}</span>
        </span>
        <span className="cycle-context__item">
          <span className="cycle-context__label">{t('cycle.grade')}</span>
          <span className="cycle-context__value">{reasoning.confidenceGrade}</span>
        </span>
      </div>

      {/* ---- Confidence bar ---- */}
      <div className="cycle-confidence-bar" aria-label={`${t('cycle.confidenceLabel')} ${confidencePct}%`}>
        <div
          className={`cycle-confidence-bar__fill ${confidenceClass(reasoning.confidence)}`}
          style={{ width: `${confidencePct}%` }}
        />
      </div>

      {/* ---- 6 Cycle Indicators ---- */}
      <div className="cycle-section">
        <h4 className="cycle-section__title">{t('cycle.indicators')}</h4>
        <div className="cycle-indicators">
          {/* Monetary */}
          <div className="cycle-indicator-row">
            <span className="cycle-indicator-row__name">{t('cycle.monetary')}</span>
            <div className="cycle-indicator-row__values">
              <span className="cycle-indicator-row__detail">
                {indicators.monetary.fedRate}% · M2 {indicators.monetary.m2Growth > 0 ? '+' : ''}{indicators.monetary.m2Growth}%
              </span>
              <span className={`cycle-phase cycle-phase--sm ${phaseClass(indicators.monetary.policyStance)}`}>
                {t(phaseKey(indicators.monetary.policyStance))}
              </span>
            </div>
          </div>

          {/* Credit */}
          <div className="cycle-indicator-row">
            <span className="cycle-indicator-row__name">{t('cycle.credit')}</span>
            <div className="cycle-indicator-row__values">
              <span className="cycle-indicator-row__detail">
                {indicators.credit.yieldCurve}
              </span>
              <span className={`cycle-phase cycle-phase--sm ${phaseClass(indicators.credit.phase)}`}>
                {t(phaseKey(indicators.credit.phase))}
              </span>
            </div>
          </div>

          {/* Economic */}
          <div className="cycle-indicator-row">
            <span className="cycle-indicator-row__name">{t('cycle.economic')}</span>
            <div className="cycle-indicator-row__values">
              <span className="cycle-indicator-row__detail">
                GDP {indicators.economic.gdpGrowth > 0 ? '+' : ''}{indicators.economic.gdpGrowth}% · {t('cycle.unemployment')} {indicators.economic.unemployment}%
              </span>
              <span className={`cycle-phase cycle-phase--sm ${phaseClass(indicators.economic.phase)}`}>
                {t(phaseKey(indicators.economic.phase))}
              </span>
            </div>
          </div>

          {/* Market — brief ref only; detail available in Market Radar panel */}
          <div className="cycle-indicator-row">
            <span className="cycle-indicator-row__name">{t('cycle.market')}</span>
            <div className="cycle-indicator-row__values">
              <span className="cycle-indicator-row__detail" title={t('cycle.marketRef')}>
                {t('cycle.marketRef')}
              </span>
              <span className={`cycle-phase cycle-phase--sm ${phaseClass(indicators.market.phase)}`}>
                {t(phaseKey(indicators.market.phase))}
              </span>
            </div>
          </div>

          {/* Sentiment */}
          <div className="cycle-indicator-row">
            <span className="cycle-indicator-row__name">{t('cycle.sentiment')}</span>
            <div className="cycle-indicator-row__values">
              <span className="cycle-indicator-row__detail">
                {t('cycle.fearGreedShort')} {indicators.sentiment.fearGreed} · {t('cycle.sentimentShort')} {(indicators.sentiment.newsSentimentAvg * 100).toFixed(0)}%
              </span>
              <span className={`cycle-phase cycle-phase--sm ${phaseClass(indicators.sentiment.phase)}`}>
                {t(phaseKey(indicators.sentiment.phase))}
              </span>
            </div>
          </div>

          {/* Geopolitical */}
          <div className="cycle-indicator-row">
            <span className="cycle-indicator-row__name">{t('cycle.geopolitical')}</span>
            <div className="cycle-indicator-row__values">
              <span className="cycle-indicator-row__detail">
                {indicators.geopolitical.eventCount} {t('cycle.eventsUnit')}
              </span>
              <span className={`cycle-phase cycle-phase--sm ${phaseClass(indicators.geopolitical.riskLevel)}`}>
                {t(phaseKey(indicators.geopolitical.riskLevel))}
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* ---- Turning Signals ---- */}
      {(reasoning.turningSignals ?? []).length > 0 && (
        <div className="cycle-section">
          <h4 className="cycle-section__title">{t('cycle.turningSignals')}</h4>
          <ul className="cycle-signals" aria-label={t('cycle.turningSignals')}>
            {reasoning.turningSignals.map((sig, i) => (
              <li
                key={i}
                className={`cycle-signal-item cycle-signal-item--${sig.direction}`}
              >
                <span className="cycle-signal-item__direction">
                  {sig.direction === 'bullish' ? '▲' : '▼'}
                </span>
                <span className="cycle-signal-item__text">{sig.signal}</span>
                <span className="cycle-signal-item__strength">
                  {t(phaseKey(sig.strength))}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* ---- Sector Recommendations ---- */}
      {(reasoning.sectorRecommendations ?? []).length > 0 && (
        <div className="cycle-section">
          <h4 className="cycle-section__title">{t('cycle.sectorRec')}</h4>
          <div className="cycle-sectors">
            {reasoning.sectorRecommendations.map((sector) => (
              <span key={sector} className="cycle-sector-tag">{sector}</span>
            ))}
          </div>
        </div>
      )}

      {/* ---- Tail Risks ---- */}
      {(reasoning.tailRisks ?? []).length > 0 && (
        <div className="cycle-section">
          <h4 className="cycle-section__title">{t('cycle.tailRisks')}</h4>
          <ul className="cycle-risks">
            {reasoning.tailRisks.map((risk) => (
              <li key={risk} className="cycle-risk-item">{risk}</li>
            ))}
          </ul>
        </div>
      )}

      {/* ---- Risk Alerts ---- */}
      {(reasoning.riskAlerts ?? []).length > 0 && (
        <div className="cycle-section">
          <h4 className="cycle-section__title">{t('cycle.riskAlerts')}</h4>
          <ul className="cycle-alerts">
            {reasoning.riskAlerts.map((alert, i) => (
              <li key={i} className="cycle-alert-item">{alert}</li>
            ))}
          </ul>
        </div>
      )}

      {/* ---- Five-Layer Reasoning Steps ---- */}
      {(reasoning.reasoningSteps ?? []).length > 0 && (
        <div className="cycle-section">
          <h4 className="cycle-section__title">{t('cycle.reasoningSteps')}</h4>
          <div className="cycle-steps">
            {reasoning.reasoningSteps.map((step) => {
              const layerKey = layerI18nKey(step.layer);
              const layerName = layerKey ? t(layerKey) : step.layer;
              const stepConfPct = Math.round(step.confidence * 100);
              return (
                <div key={step.step} className="cycle-step">
                  <div className="cycle-step__header">
                    <span className="cycle-step__num">{t('cycle.step', { n: step.step })}</span>
                    <span className="cycle-step__layer">{layerName}</span>
                  </div>
                  <p className="cycle-step__finding">{step.finding}</p>
                  {step.evidence.length > 0 && (
                    <div className="cycle-step__evidence">
                      {step.evidence.map((e, i) => (
                        <span key={i} className="cycle-step__evidence-tag">{e}</span>
                      ))}
                    </div>
                  )}
                  <div
                    className={`cycle-step__confidence ${confidenceClass(step.confidence)}`}
                    style={{ width: `${stepConfPct}%` }}
                    aria-label={`${t('cycle.confidence')} ${stepConfPct}%`}
                  />
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* ---- Narrative / Intelligence Brief ---- */}
      {reasoning.narrative && (
        <div className="cycle-section">
          <h4 className="cycle-section__title">{t('cycle.narrative')}</h4>
          <p className="cycle-narrative">{reasoning.narrative}</p>
        </div>
      )}

      {/* ---- Timestamp ---- */}
      <div className="cycle-timestamp">
        <span>{t('cycle.lastUpdate')}: {formatTimestamp(reasoning.timestamp)}</span>
      </div>
    </div>
  );
}
