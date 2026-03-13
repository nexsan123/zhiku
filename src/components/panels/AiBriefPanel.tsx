import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import { getAiBrief } from '@services/tauri-bridge';
import type { AiBriefCategory } from '@services/tauri-bridge';
import './AiBriefPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

/** Map category string to CSS modifier and CSS variable for color. */
function categoryModifier(category: string): string {
  switch (category) {
    case 'geopolitical':  return 'geopolitical';
    case 'macro_policy':  return 'macro-policy';
    case 'market':        return 'market';
    case 'corporate':     return 'corporate';
    default:              return 'market';
  }
}

/** Derive sentiment label from avgSentiment (0-1 scale). */
function sentimentLabel(avg: number): 'positive' | 'neutral' | 'negative' {
  if (avg >= 0.6) return 'positive';
  if (avg <= 0.4) return 'negative';
  return 'neutral';
}

/** Map sentiment to i18n key. */
function sentimentKey(sentiment: 'positive' | 'neutral' | 'negative'): string {
  if (sentiment === 'positive') return 'aiBrief.positive';
  if (sentiment === 'negative') return 'aiBrief.negative';
  return 'aiBrief.neutral';
}

export function AiBriefPanel() {
  const { t } = useTranslation();
  const [items, setItems] = useState<AiBriefCategory[]>([]);
  const [loadState, setLoadState] = useState<LoadState>('loading');

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const data = await getAiBrief();
      setItems(data);
      setLoadState('loaded');
    } catch {
      setLoadState('error');
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (loadState === 'loading') {
    return (
      <div className="ai-brief__state">
        <RefreshCw size={14} className="ai-brief__spinner" />
        <span className="ai-brief__state-text">{t('aiBrief.loading')}</span>
      </div>
    );
  }

  if (loadState === 'error') {
    return (
      <div className="ai-brief__state ai-brief__state--error">
        <p className="ai-brief__state-text">{t('aiBrief.failed')}</p>
        <button className="ai-brief__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  if (items.length === 0) {
    return (
      <div className="ai-brief__state">
        <p className="ai-brief__state-text">{t('aiBrief.noData')}</p>
      </div>
    );
  }

  return (
    <div className="ai-brief">
      <ul className="ai-brief__list" aria-label="AI brief by category">
        {items.map((item) => {
          const catMod = categoryModifier(item.category);
          const sent = sentimentLabel(item.avgSentiment);
          return (
            <li key={item.category} className="ai-brief__card">
              {/* Card header: category tag + sentiment + count */}
              <div className="ai-brief__card-header">
                <span className={`ai-brief__category ai-brief__category--${catMod}`}>
                  {t(`category.${item.category}`, { defaultValue: item.category })}
                </span>
                <span className={`ai-brief__sentiment ai-brief__sentiment--${sent}`}>
                  {t(sentimentKey(sent))}
                </span>
                <span className="ai-brief__time">{item.count}{t('cycle.articlesUnit')}</span>
              </div>

              {/* Summary text */}
              <p className="ai-brief__summary">{item.latestSummary}</p>

              {/* Keyword pills */}
              {item.topKeywords.length > 0 && (
                <div className="ai-brief__keywords" aria-label="Keywords">
                  {item.topKeywords.map((kw) => (
                    <span key={kw} className="ai-brief__keyword">{kw}</span>
                  ))}
                </div>
              )}
            </li>
          );
        })}
      </ul>

      {/* Footer: AI attribution */}
      <div className="ai-brief__footer">
        <span className="ai-brief__footer-label">{t('aiBrief.generated')}</span>
      </div>
    </div>
  );
}
