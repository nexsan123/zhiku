import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw, ChevronDown, ChevronRight } from 'lucide-react';
import { getDeepAnalyses } from '@services/tauri-bridge';
import type { DeepAnalysis } from '@services/tauri-bridge';
import './IntelBriefPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

function gradeClass(grade: string): string {
  if (grade === 'high') return 'ib-grade--high';
  if (grade === 'reasonable') return 'ib-grade--reasonable';
  return 'ib-grade--speculative';
}

function gradeLabel(grade: string): string {
  if (grade === 'high') return 'HIGH';
  if (grade === 'reasonable') return 'REASONABLE';
  return 'SPECULATIVE';
}

function AnalysisCard({ analysis }: { analysis: DeepAnalysis }) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);

  const timeAgo = (() => {
    const ms = Date.now() - new Date(analysis.analyzedAt).getTime();
    const mins = Math.floor(ms / 60000);
    if (mins < 60) return `${mins}m`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h`;
    return `${Math.floor(hrs / 24)}d`;
  })();

  return (
    <div className="ib-card">
      <div className="ib-card__header" onClick={() => setExpanded(!expanded)}>
        <div className="ib-card__header-left">
          {expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
          <span className="ib-card__topic">{analysis.clusterTopic}</span>
        </div>
        <div className="ib-card__header-right">
          <span className={`ib-grade-badge ${gradeClass(analysis.deepAnalysis.confidenceGrade)}`}>
            {gradeLabel(analysis.deepAnalysis.confidenceGrade)}
          </span>
          <span className="ib-card__time">{timeAgo}</span>
        </div>
      </div>

      {/* Key observation always visible */}
      <div className="ib-card__observation">
        <span className="ib-card__observation-text">{analysis.keyObservation}</span>
      </div>

      {/* Expanded details */}
      {expanded && (
        <div className="ib-card__details">
          {/* Surface */}
          <div className="ib-detail-section">
            <span className="ib-detail-section__label">{t('intel.surface')}</span>
            <p className="ib-detail-section__text">{analysis.surface}</p>
          </div>

          {/* Connection */}
          <div className="ib-detail-section">
            <span className="ib-detail-section__label">{t('intel.connection')}</span>
            <p className="ib-detail-section__text">{analysis.connection}</p>
          </div>

          {/* Deep analysis */}
          <div className="ib-detail-section">
            <span className="ib-detail-section__label">{t('intel.deepMotive')}</span>
            <p className="ib-detail-section__text">{analysis.deepAnalysis.primaryMotive}</p>
            {analysis.deepAnalysis.secondaryMotive && (
              <p className="ib-detail-section__text ib-detail-section__text--secondary">
                {t('intel.hiddenMotive')}: {analysis.deepAnalysis.secondaryMotive}
              </p>
            )}
          </div>

          {/* Five-layer impact */}
          <div className="ib-detail-section">
            <span className="ib-detail-section__label">{t('intel.layerImpact')}</span>
            <div className="ib-layer-grid">
              <div className="ib-layer-item"><span className="ib-layer-item__name">{t('intel.layerPhysical')}</span><span className="ib-layer-item__val">{analysis.layerImpact.physical}</span></div>
              <div className="ib-layer-item"><span className="ib-layer-item__name">{t('intel.layerCredit')}</span><span className="ib-layer-item__val">{analysis.layerImpact.credit}</span></div>
              <div className="ib-layer-item"><span className="ib-layer-item__name">{t('intel.layerDollar')}</span><span className="ib-layer-item__val">{analysis.layerImpact.dollar}</span></div>
              <div className="ib-layer-item"><span className="ib-layer-item__name">{t('intel.layerGeopolitical')}</span><span className="ib-layer-item__val">{analysis.layerImpact.geopolitical}</span></div>
              <div className="ib-layer-item"><span className="ib-layer-item__name">{t('intel.layerSentiment')}</span><span className="ib-layer-item__val">{analysis.layerImpact.sentiment}</span></div>
            </div>
          </div>

          {/* Sources */}
          {analysis.sourceUrls.length > 0 && (
            <div className="ib-detail-section">
              <span className="ib-detail-section__label">{t('intel.sources')} ({analysis.newsCount})</span>
              <div className="ib-sources">
                {analysis.sourceUrls.slice(0, 3).map((url, i) => (
                  <span key={i} className="ib-source-link" title={url}>
                    {(() => { try { return new URL(url).hostname.replace(/^www\./, ''); } catch { return url; } })()}
                  </span>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function IntelBriefPanel() {
  const { t } = useTranslation();
  const [analyses, setAnalyses] = useState<DeepAnalysis[]>([]);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const data = await getDeepAnalyses(10);
      setAnalyses(data);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  if (loadState === 'loading') {
    return (
      <div className="ib-panel__state">
        <RefreshCw size={14} className="ib-panel__spinner" />
        <span className="ib-panel__state-text">{t('intel.loading')}</span>
      </div>
    );
  }

  if (loadState === 'error') {
    return (
      <div className="ib-panel__state ib-panel__state--error">
        <p className="ib-panel__state-text">{t('intel.failed')}</p>
        <button className="ib-panel__retry-btn" onClick={() => void load()}>{t('state.retry')}</button>
      </div>
    );
  }

  if (analyses.length === 0) {
    return (
      <div className="ib-panel__state">
        <p className="ib-panel__state-text">{t('intel.noData')}</p>
        <button className="ib-panel__retry-btn" onClick={() => void load()}>{t('state.retry')}</button>
      </div>
    );
  }

  return (
    <div className="ib-panel">
      <div className="ib-panel__header">
        <span className="ib-panel__count">{analyses.length} {t('intel.clusters')}</span>
        <span className="ib-panel__ai-tag">{t('intel.aiGenerated')}</span>
      </div>
      <div className="ib-panel__list">
        {analyses.map((a) => <AnalysisCard key={a.clusterId} analysis={a} />)}
      </div>
    </div>
  );
}
