import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getCreditCycleOverview } from '@services/tauri-bridge';
import type { GlobalCycleOverview, CountryCyclePosition } from '@services/tauri-bridge';
import './CreditCyclePanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

/** Phase color class. */
function phaseColorClass(phase: string): string {
  if (['easing', 'leveraging'].includes(phase)) return 'cc-phase--warm';
  if (['overheating'].includes(phase)) return 'cc-phase--hot';
  if (['tightening'].includes(phase)) return 'cc-phase--cool';
  if (['deleveraging', 'clearing'].includes(phase)) return 'cc-phase--cold';
  return 'cc-phase--unknown';
}

/** Confidence grade color class. */
function gradeClass(grade: string): string {
  if (grade === 'high') return 'cc-grade--high';
  if (grade === 'reasonable') return 'cc-grade--reasonable';
  return 'cc-grade--speculative';
}

/** Tide color class. */
function tideClass(state: string): string {
  if (state === 'rising') return 'cc-tide--rising';
  if (state === 'ebbing') return 'cc-tide--ebbing';
  return 'cc-tide--neutral';
}

/** Severity color class for risk alerts. */
function severityClass(sev: string): string {
  if (sev === 'critical') return 'cc-alert--critical';
  if (sev === 'danger') return 'cc-alert--danger';
  return 'cc-alert--warning';
}

function formatPct(val: number | null | undefined): string {
  if (val == null) return '--';
  return `${val >= 0 ? '+' : ''}${val.toFixed(1)}%`;
}

function CountryRow({ c, expanded }: { c: CountryCyclePosition; expanded: boolean }) {
  const { t, i18n } = useTranslation();
  const isZh = i18n.language.startsWith('zh');
  const ind = c.indicators;
  const hasIncome = ind.imfGdpGrowth != null || ind.imfFiscalBalance != null
    || ind.imfCurrentAccount != null || ind.imfGovDebt != null || ind.imfGovRevenue != null;

  return (
    <div className={`cc-country-row ${expanded ? 'cc-country-row--expanded' : ''}`}>
      <div className="cc-country-row__header">
        <div className="cc-country-row__left">
          <span className="cc-country-row__code">{c.countryCode}</span>
          <span className="cc-country-row__name">{c.countryName}</span>
        </div>
        <div className="cc-country-row__right">
          <span className={`cc-phase-badge ${phaseColorClass(c.phase)}`}>
            {isZh ? c.phaseLabel : c.phase}
          </span>
          {ind.creditGdpGap != null && (
            <span className="cc-country-row__gap" title={t('creditCycle.creditGdpGap')}>
              {formatPct(ind.creditGdpGap)}
            </span>
          )}
          <span className={`cc-grade-dot ${gradeClass(c.confidenceGrade)}`} title={t(`grade.${c.confidenceGrade}`, { defaultValue: c.confidenceGrade })} />
          {c.reliability < 0.70 && (
            <span className="cc-country-row__warn" title={t('creditCycle.lowReliability')}>!</span>
          )}
        </div>
      </div>
      {expanded && hasIncome && (
        <div className="cc-country-row__income">
          {ind.imfGdpGrowth != null && (
            <span className="cc-income-chip" title={t('creditCycle.gdpGrowth')}>
              <span className="cc-income-chip__label">GDP</span>
              <span className={ind.imfGdpGrowth >= 0 ? 'cc-val--positive' : 'cc-val--negative'}>{formatPct(ind.imfGdpGrowth)}</span>
            </span>
          )}
          {ind.imfFiscalBalance != null && (
            <span className="cc-income-chip" title={t('creditCycle.fiscalBalance')}>
              <span className="cc-income-chip__label">{t('creditCycle.fiscalShort')}</span>
              <span className={ind.imfFiscalBalance >= 0 ? 'cc-val--positive' : 'cc-val--negative'}>{formatPct(ind.imfFiscalBalance)}</span>
            </span>
          )}
          {ind.imfCurrentAccount != null && (
            <span className="cc-income-chip" title={t('creditCycle.currentAccount')}>
              <span className="cc-income-chip__label">CA</span>
              <span className={ind.imfCurrentAccount >= 0 ? 'cc-val--positive' : 'cc-val--negative'}>{formatPct(ind.imfCurrentAccount)}</span>
            </span>
          )}
          {ind.imfGovDebt != null && (
            <span className="cc-income-chip" title={t('creditCycle.govDebt')}>
              <span className="cc-income-chip__label">{t('creditCycle.debtShort')}</span>
              <span className={ind.imfGovDebt > 100 ? 'cc-val--negative' : 'cc-val--neutral'}>{formatPct(ind.imfGovDebt)}</span>
            </span>
          )}
          {ind.imfGovRevenue != null && (
            <span className="cc-income-chip" title={t('creditCycle.govRevenue')}>
              <span className="cc-income-chip__label">{t('creditCycle.revShort')}</span>
              <span className="cc-val--neutral">{formatPct(ind.imfGovRevenue)}</span>
            </span>
          )}
        </div>
      )}
      {expanded && !hasIncome && (
        <div className="cc-country-row__income cc-country-row__income--empty">
          <span className="cc-income-chip__label">{t('creditCycle.noImfData')}</span>
        </div>
      )}
    </div>
  );
}

export function CreditCyclePanel() {
  const { t, i18n } = useTranslation();
  const isZh = i18n.language.startsWith('zh');
  const [data, setData] = useState<GlobalCycleOverview | null>(null);
  const [loadState, setLoadState] = useState<LoadState>('loading');
  const [showIncome, setShowIncome] = useState(false);

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const overview = await getCreditCycleOverview();
      setData(overview);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  if (loadState === 'loading') {
    return (
      <div className="cc-panel__state">
        <RefreshCw size={14} className="cc-panel__spinner" />
        <span className="cc-panel__state-text">{t('creditCycle.loading')}</span>
      </div>
    );
  }

  if (loadState === 'error' || !data) {
    return (
      <div className="cc-panel__state cc-panel__state--error">
        <p className="cc-panel__state-text">{t(loadState === 'error' ? 'creditCycle.failed' : 'creditCycle.noData')}</p>
        <button className="cc-panel__retry-btn" onClick={() => void load()}>{t('state.retry')}</button>
      </div>
    );
  }

  const core = data.countries.filter(c => c.tier === 'core');
  const important = data.countries.filter(c => c.tier === 'important');
  const monitor = data.countries.filter(c => c.tier === 'monitor');

  return (
    <div className="cc-panel">
      {/* Global phase + dollar tide header */}
      <div className="cc-header">
        <div className="cc-header__item">
          <span className="cc-header__label">{t('creditCycle.globalPhase')}</span>
          <span className={`cc-phase-badge cc-phase-badge--lg ${phaseColorClass(data.globalPhase)}`}>
            {isZh ? data.globalPhaseLabel : data.globalPhase}
          </span>
        </div>
        <div className="cc-header__item">
          <span className="cc-header__label">{t('creditCycle.dollarTide')}</span>
          <span className={`cc-tide-badge ${tideClass(data.dollarTide.tideState)}`}>
            {isZh ? data.dollarTide.tideLabel : data.dollarTide.tideState}
          </span>
        </div>
        <div className="cc-header__item">
          <span className="cc-header__label">{t('creditCycle.percentile')}</span>
          <span className="cc-header__value">{data.globalPercentile.toFixed(0)}%</span>
        </div>
      </div>

      {/* Percentile bar */}
      <div className="cc-percentile-bar">
        <div className="cc-percentile-bar__fill" style={{ width: `${data.globalPercentile}%` }} />
        <span className="cc-percentile-bar__marker" style={{ left: `${data.globalPercentile}%` }} />
      </div>

      {/* Risk alerts */}
      {data.riskAlerts.length > 0 && (
        <div className="cc-section">
          <h4 className="cc-section__title">{t('creditCycle.riskAlerts')}</h4>
          <ul className="cc-alerts">
            {data.riskAlerts.map((a, i) => (
              <li key={i} className={`cc-alert-item ${severityClass(a.severity)}`}>
                <span className="cc-alert-item__code">{a.countryCode}</span>
                <span className="cc-alert-item__text">{a.alert}</span>
                <span className={`cc-grade-dot ${gradeClass(a.confidenceGrade)}`} />
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* Income toggle */}
      <button className="cc-income-toggle" onClick={() => setShowIncome(!showIncome)}>
        {showIncome ? '▾' : '▸'} {t('creditCycle.incomeSection')}
      </button>

      {/* Core countries */}
      <div className="cc-section">
        <h4 className="cc-section__title">
          {t('creditCycle.coreTier')}
          <span className={`cc-phase-badge cc-phase-badge--sm ${phaseColorClass(data.coreSummary.dominantPhase)}`}>
            {isZh ? data.coreSummary.dominantPhaseLabel : data.coreSummary.dominantPhase}
          </span>
        </h4>
        {core.map(c => <CountryRow key={c.countryCode} c={c} expanded={showIncome} />)}
      </div>

      {/* Important countries */}
      {important.length > 0 && (
        <div className="cc-section">
          <h4 className="cc-section__title">
            {t('creditCycle.importantTier')}
            <span className={`cc-phase-badge cc-phase-badge--sm ${phaseColorClass(data.importantSummary.dominantPhase)}`}>
              {isZh ? data.importantSummary.dominantPhaseLabel : data.importantSummary.dominantPhase}
            </span>
          </h4>
          {important.map(c => <CountryRow key={c.countryCode} c={c} expanded={showIncome} />)}
        </div>
      )}

      {/* Monitor countries */}
      {monitor.length > 0 && (
        <div className="cc-section">
          <h4 className="cc-section__title">
            {t('creditCycle.monitorTier')}
            <span className={`cc-phase-badge cc-phase-badge--sm ${phaseColorClass(data.monitorSummary.dominantPhase)}`}>
              {isZh ? data.monitorSummary.dominantPhaseLabel : data.monitorSummary.dominantPhase}
            </span>
          </h4>
          {monitor.map(c => <CountryRow key={c.countryCode} c={c} expanded={showIncome} />)}
        </div>
      )}

      {/* Dollar tide detail */}
      <div className="cc-section">
        <h4 className="cc-section__title">{t('creditCycle.tideDetail')}</h4>
        <div className="cc-tide-detail">
          <div className="cc-tide-row">
            <span>{t('creditCycle.dxy3m')}</span><span>{formatPct(data.dollarTide.dxyTrend3m)}</span>
          </div>
          <div className="cc-tide-row">
            <span>{t('creditCycle.dxy6m')}</span><span>{formatPct(data.dollarTide.dxyTrend6m)}</span>
          </div>
          <div className="cc-tide-row">
            <span>{t('creditCycle.m2GrowthLabel')}</span><span>{formatPct(data.dollarTide.m2Growth)}</span>
          </div>
          <div className="cc-tide-row">
            <span>{t('creditCycle.yieldSpread')}</span><span>{data.dollarTide.yieldSpread.toFixed(2)}pp</span>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="cc-footer">
        <span>{t('creditCycle.dataAsOf')}: {data.dataPeriod}</span>
        <span className={`cc-grade-dot ${gradeClass(data.confidence >= 0.8 ? 'high' : data.confidence >= 0.5 ? 'reasonable' : 'speculative')}`} />
      </div>
    </div>
  );
}
