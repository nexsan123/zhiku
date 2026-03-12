import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getPolicyVectors, getBilateralDynamics, getDecisionCalendar, getActiveScenarios } from '@services/tauri-bridge';
import type { PolicyVector, BilateralDynamic, CalendarEvent, ScenarioMatrix, Scenario } from '@services/tauri-bridge';
import './GameMapPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

function activityClass(label: string): string {
  if (label === 'critical') return 'gm-activity--critical';
  if (label === 'high') return 'gm-activity--high';
  if (label === 'moderate') return 'gm-activity--moderate';
  return 'gm-activity--low';
}

function dirClass(dir: string): string {
  if (dir === 'bullish') return 'gm-dir--bullish';
  if (dir === 'bearish') return 'gm-dir--bearish';
  return 'gm-dir--neutral';
}

function probChange(s: Scenario): { delta: number; arrow: string; cls: string } {
  const delta = Math.round((s.probability - s.previousProbability) * 100);
  if (delta > 0) return { delta, arrow: '▲', cls: 'gm-prob--up' };
  if (delta < 0) return { delta, arrow: '▼', cls: 'gm-prob--down' };
  return { delta: 0, arrow: '—', cls: 'gm-prob--flat' };
}

function gradeClass(grade: string): string {
  if (grade === 'high') return 'gm-grade--high';
  if (grade === 'reasonable') return 'gm-grade--reasonable';
  return 'gm-grade--speculative';
}

export function GameMapPanel() {
  const { t, i18n } = useTranslation();
  const isZh = i18n.language.startsWith('zh');

  const [vectors, setVectors] = useState<PolicyVector[]>([]);
  const [bilaterals, setBilaterals] = useState<BilateralDynamic[]>([]);
  const [calendar, setCalendar] = useState<CalendarEvent[]>([]);
  const [scenarios, setScenarios] = useState<ScenarioMatrix | null>(null);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const [v, b, c, s] = await Promise.all([
        getPolicyVectors(),
        getBilateralDynamics(),
        getDecisionCalendar(30),
        getActiveScenarios(),
      ]);
      setVectors(v);
      setBilaterals(b);
      setCalendar(c);
      setScenarios(s);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  if (loadState === 'loading') {
    return (
      <div className="gm-panel__state">
        <RefreshCw size={14} className="gm-panel__spinner" />
        <span className="gm-panel__state-text">{t('gameMap.loading')}</span>
      </div>
    );
  }

  if (loadState === 'error') {
    return (
      <div className="gm-panel__state gm-panel__state--error">
        <p className="gm-panel__state-text">{t('gameMap.failed')}</p>
        <button className="gm-panel__retry-btn" onClick={() => void load()}>{t('state.retry')}</button>
      </div>
    );
  }

  const hasData = vectors.length > 0 || bilaterals.length > 0 || calendar.length > 0 || (scenarios && scenarios.scenarios.length > 0);
  if (!hasData) {
    return (
      <div className="gm-panel__state">
        <span className="gm-panel__state-text">{t('gameMap.noData')}</span>
      </div>
    );
  }

  // Sort vectors by activity desc
  const sorted = [...vectors].sort((a, b) => b.activity - a.activity);

  return (
    <div className="gm-panel">
      {/* Policy vectors */}
      <div className="gm-section">
        <h4 className="gm-section__title">{t('gameMap.policyVectors')}</h4>
        <div className="gm-vectors">
          {sorted.map((v) => (
            <div key={v.id} className="gm-vector-row">
              <div className="gm-vector-row__left">
                <span className="gm-vector-row__name">{isZh ? v.nameZh : v.name}</span>
                <span className={`gm-activity-badge ${activityClass(v.activityLabel)}`}>
                  {v.activityLabel.toUpperCase()}
                </span>
              </div>
              <div className="gm-vector-row__bar-wrap">
                <div className={`gm-vector-row__bar ${activityClass(v.activityLabel)}`} style={{ width: `${Math.round(v.activity * 100)}%` }} />
              </div>
              <span className="gm-vector-row__count">{v.newsCount7d}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Bilateral dynamics */}
      {bilaterals.length > 0 && (
        <div className="gm-section">
          <h4 className="gm-section__title">{t('gameMap.bilaterals')}</h4>
          <div className="gm-bilaterals">
            {bilaterals.map((b) => (
              <div key={b.id} className="gm-bilateral-row">
                <span className="gm-bilateral-row__name">{isZh ? b.nameZh : b.name}</span>
                <div className="gm-bilateral-row__bar-wrap">
                  <div className={`gm-bilateral-row__bar gm-tension--${b.tensionLabel}`} style={{ width: `${Math.round(b.tension * 100)}%` }} />
                </div>
                <span className={`gm-tension-badge gm-tension--${b.tensionLabel}`}>
                  {b.tensionLabel.toUpperCase()}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Decision calendar */}
      {calendar.length > 0 && (
        <div className="gm-section">
          <h4 className="gm-section__title">{t('gameMap.calendar')} ({calendar.length})</h4>
          <div className="gm-calendar">
            {calendar.slice(0, 5).map((evt, i) => {
              const d = new Date(evt.date);
              const dayStr = d.toLocaleDateString(isZh ? 'zh-CN' : 'en-US', { month: 'short', day: 'numeric' });
              const daysUntil = Math.ceil((d.getTime() - Date.now()) / 86400000);
              return (
                <div key={i} className="gm-cal-item">
                  <div className="gm-cal-item__date">
                    <span className="gm-cal-item__day">{dayStr}</span>
                    {daysUntil >= 0 && <span className="gm-cal-item__until">{daysUntil}d</span>}
                  </div>
                  <div className="gm-cal-item__info">
                    <span className="gm-cal-item__title">{evt.title}</span>
                    <div className="gm-cal-item__assets">
                      {evt.affectedAssets.slice(0, 3).map((a) => (
                        <span key={a} className={`gm-asset-tag ${dirClass(evt.impactDirection)}`}>{a}</span>
                      ))}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Scenarios */}
      {scenarios && scenarios.scenarios.length > 0 && (
        <div className="gm-section">
          <h4 className="gm-section__title">
            {t('gameMap.scenarios')}
            <span className="gm-ai-tag">{t('gameMap.aiTag')}</span>
          </h4>
          <div className="gm-scenarios">
            {scenarios.scenarios.map((s) => {
              const { delta, arrow, cls } = probChange(s);
              return (
                <div key={s.id} className="gm-scenario-card">
                  <div className="gm-scenario-card__header">
                    <span className="gm-scenario-card__vector">{s.policyVector}</span>
                    <span className="gm-scenario-card__title">{s.title}</span>
                    <div className="gm-scenario-card__prob">
                      <span className="gm-scenario-card__pct">{Math.round(s.probability * 100)}%</span>
                      {delta !== 0 && (
                        <span className={`gm-scenario-card__delta ${cls}`}>
                          {arrow}{Math.abs(delta)}
                        </span>
                      )}
                    </div>
                  </div>
                  <p className="gm-scenario-card__desc">{s.description}</p>
                  {s.changeReason && (
                    <p className="gm-scenario-card__reason">{s.changeReason}</p>
                  )}
                  <div className="gm-scenario-card__impacts">
                    {s.assetImpacts.slice(0, 4).map((ai) => (
                      <span key={ai.symbol} className={`gm-asset-impact ${dirClass(ai.direction)}`}>
                        {ai.symbol} {ai.direction === 'bullish' ? '▲' : ai.direction === 'bearish' ? '▼' : '—'}
                      </span>
                    ))}
                    <span className={`gm-scenario-card__grade ${gradeClass(s.confidenceGrade)}`}>
                      {s.confidenceGrade.toUpperCase()}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
