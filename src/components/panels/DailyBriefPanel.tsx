import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw, FileText } from 'lucide-react';
import { getDailyBrief, listenDailyBriefUpdated, formatTimeAgo } from '@services/tauri-bridge';
import type { DailyBrief, AttentionItem, SectorAdjustment } from '@services/tauri-bridge';
import './DailyBriefPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

function urgencyClass(urgency: string): string {
  if (urgency === 'high' || urgency === 'critical') return 'high';
  if (urgency === 'low') return 'low';
  return 'medium';
}

function sectorWeightClass(weight: number): string {
  if (weight > 1.05) return 'overweight';
  if (weight < 0.95) return 'underweight';
  return 'neutral';
}

function sectorWeightLabel(weight: number): string {
  const pct = Math.round((weight - 1) * 100);
  if (pct > 0) return `+${pct}%`;
  if (pct < 0) return `${pct}%`;
  return '0%';
}

function biasClass(bias: number): string {
  if (bias > 0.1) return 'bullish';
  if (bias < -0.1) return 'bearish';
  return 'neutral';
}

/** Convert positionBias (-1 to +1) to bar fill width (0-100%). */
function biasToWidth(bias: number): number {
  return Math.round(Math.abs(bias) * 100);
}

function AttentionItemRow({ item }: { item: AttentionItem }) {
  const { t } = useTranslation();
  return (
    <li className={`db-panel__attention-item db-panel__attention-item--${item.priority}`}>
      <div className="db-panel__attention-header">
        <span className={`db-panel__priority-badge db-panel__priority-badge--${item.priority}`}>
          {t(`dailyBrief.priority.${item.priority}`, { defaultValue: item.priority.toUpperCase() })}
        </span>
        <span className="db-panel__attention-category">{item.category}</span>
      </div>
      <p className="db-panel__attention-content">{item.content}</p>
      {item.reason && (
        <span className="db-panel__attention-reason">{item.reason}</span>
      )}
    </li>
  );
}

function SectorRow({ sector }: { sector: SectorAdjustment }) {
  const cls = sectorWeightClass(sector.weight);
  return (
    <div className="db-panel__sector-row">
      <span className="db-panel__sector-name">{sector.sector}</span>
      <span className={`db-panel__sector-weight db-panel__sector-weight--${cls}`}>
        {sectorWeightLabel(sector.weight)}
      </span>
      <span className="db-panel__sector-reason">{sector.reason}</span>
    </div>
  );
}

function BriefContent({ brief }: { brief: DailyBrief }) {
  const { t } = useTranslation();
  const { qtSuggestion: qt, dataSnapshot: snap } = brief;
  const biasCls = biasClass(qt.positionBias);
  const biasPct = biasToWidth(qt.positionBias);

  const sortedAttention = [...brief.attentionItems].sort((a, b) => {
    const order = { high: 0, medium: 1, low: 2 };
    return order[a.priority] - order[b.priority];
  });

  return (
    <div className="db-panel">
      {/* Headline */}
      <p className="db-panel__headline">{brief.headline}</p>

      {/* Contradictions */}
      {brief.keyContradictions.length > 0 && (
        <div className="db-panel__contradictions">
          {brief.keyContradictions.map((c, i) => (
            <div key={i} className="db-panel__contradiction">
              <span className="db-panel__contradiction-icon">!</span>
              <span className="db-panel__contradiction-text">{c}</span>
            </div>
          ))}
        </div>
      )}

      {/* Attention items */}
      {sortedAttention.length > 0 && (
        <div>
          <div className="db-panel__section-label">
            {t('dailyBrief.attentionItems')}
          </div>
          <ul className="db-panel__attention-list" aria-label="Attention items">
            {sortedAttention.map((item, i) => (
              <AttentionItemRow key={i} item={item} />
            ))}
          </ul>
        </div>
      )}

      {/* QT Suggestion */}
      <div>
        <div className="db-panel__section-label">
          {t('dailyBrief.qtSuggestion')}
        </div>
        <div className="db-panel__qt-card">
          <div className="db-panel__qt-header">
            <span className={`db-panel__qt-urgency db-panel__qt-urgency--${urgencyClass(qt.urgency)}`}>
              {qt.urgency}
            </span>
            <span className="db-panel__qt-risk">
              {t('dailyBrief.riskMultiplier')}: {qt.riskMultiplier.toFixed(2)}x
            </span>
          </div>

          {/* Position bias bar */}
          <div className="db-panel__bias-row">
            <div className="db-panel__bias-label">
              <span>{t('dailyBrief.positionBias')}</span>
              <span style={{ color: biasCls === 'bullish' ? 'var(--color-semantic-success)' : biasCls === 'bearish' ? 'var(--color-semantic-error)' : 'var(--color-text-disabled)' }}>
                {biasCls === 'bullish' ? `+${biasPct}%` : biasCls === 'bearish' ? `-${biasPct}%` : '0%'}
              </span>
            </div>
            <div className="db-panel__bias-bar-track" aria-label={`Position bias: ${biasCls}`}>
              <div
                className={`db-panel__bias-bar-fill db-panel__bias-bar-fill--${biasCls}`}
                style={{ width: `${biasPct}%` }}
              />
            </div>
          </div>

          {/* Sector adjustments */}
          {qt.sectorAdjustments.length > 0 && (
            <div className="db-panel__sectors">
              {qt.sectorAdjustments.map((s, i) => (
                <SectorRow key={i} sector={s} />
              ))}
            </div>
          )}

          {/* Reasoning */}
          {qt.reasoning && (
            <div className="db-panel__qt-reasoning">{qt.reasoning}</div>
          )}
        </div>
      </div>

      {/* Data snapshot */}
      <div>
        <div className="db-panel__section-label">
          {t('dailyBrief.dataSnapshot')}
        </div>
        <div className="db-panel__snapshot">
          <div className="db-panel__snapshot-row">
            <span className="db-panel__snapshot-key">{t('dailyBrief.snap.cycle')}</span>
            <span className="db-panel__snapshot-val">{snap.cyclePhase}</span>
          </div>
          <div className="db-panel__snapshot-row">
            <span className="db-panel__snapshot-key">{t('dailyBrief.snap.fearGreed')}</span>
            <span className="db-panel__snapshot-val">{snap.fearGreed}</span>
          </div>
          <div className="db-panel__snapshot-row">
            <span className="db-panel__snapshot-key">{t('fred.fedfunds')}</span>
            <span className="db-panel__snapshot-val">{snap.fedRate.toFixed(2)}%</span>
          </div>
          <div className="db-panel__snapshot-row">
            <span className="db-panel__snapshot-key">{t('fred.cpi')}</span>
            <span className="db-panel__snapshot-val">{snap.cpiYoy.toFixed(1)}%</span>
          </div>
          <div className="db-panel__snapshot-row">
            <span className="db-panel__snapshot-key">{t('dailyBrief.snap.sp500')}</span>
            <span className="db-panel__snapshot-val">{snap.sp500Trend > 0 ? '+' : ''}{snap.sp500Trend.toFixed(1)}%</span>
          </div>
          <div className="db-panel__snapshot-row">
            <span className="db-panel__snapshot-key">{t('dailyBrief.snap.geoRisk')}</span>
            <span className="db-panel__snapshot-val">{snap.geopoliticalRisk}</span>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="db-panel__footer">
        <span className="db-panel__footer-model">{brief.model}</span>
        <span className="db-panel__footer-time">{formatTimeAgo(brief.generatedAt)}</span>
      </div>
    </div>
  );
}

export function DailyBriefPanel() {
  const { t } = useTranslation();
  const [brief, setBrief] = useState<DailyBrief | null>(null);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const data = await getDailyBrief();
      setBrief(data);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }
  }, []);

  useEffect(() => {
    void load();
    let cleanup: (() => void) | null = null;
    const unlistenPromise = listenDailyBriefUpdated(() => void load());
    void unlistenPromise.then((fn) => { cleanup = fn; });
    return () => {
      if (cleanup) { cleanup(); }
      else { void unlistenPromise.then((fn) => fn()); }
    };
  }, [load]);

  if (loadState === 'loading') {
    return (
      <div className="db-panel__state">
        <RefreshCw size={14} className="db-panel__spinner" />
        <span className="db-panel__state-text">{t('dailyBrief.loading')}</span>
      </div>
    );
  }

  if (loadState === 'error') {
    return (
      <div className="db-panel__state db-panel__state--error">
        <p className="db-panel__state-text">{t('dailyBrief.failed')}</p>
        <button className="db-panel__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  if (!brief) {
    return (
      <div className="db-panel__state">
        <FileText size={24} style={{ opacity: 0.3 }} />
        <p className="db-panel__state-text">{t('dailyBrief.noData')}</p>
        <p className="db-panel__state-sub">{t('dailyBrief.noDataSub')}</p>
      </div>
    );
  }

  return <BriefContent brief={brief} />;
}
