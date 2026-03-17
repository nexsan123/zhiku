import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw, Inbox, ChevronDown, ChevronRight } from 'lucide-react';
import { getNews, getAiBrief, listenNewsUpdated, listenAiSummaryCompleted, hostnameFromUrl, formatTimeAgo } from '@services/tauri-bridge';
import type { AiBriefCategory } from '@services/tauri-bridge';
import type { NewsItem } from '@contracts/api-news';
import { NewsDetailModal } from '@components/news-detail';
import './NewsFeedPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

function getSentimentClass(score: number): 'positive' | 'negative' | 'neutral' {
  if (score > 0.6) return 'positive';
  if (score < 0.4) return 'negative';
  return 'neutral';
}

function getSentimentLabel(score: number): string {
  if (score > 0.6) return 'Positive';
  if (score < 0.4) return 'Negative';
  return 'Neutral';
}

const CATEGORY_COLORS: Record<string, string> = {
  geopolitical: 'var(--color-intel-geopolitical)',
  macro_policy: 'var(--color-intel-macro-policy)',
  market: 'var(--color-intel-market)',
  corporate: 'var(--color-intel-corporate)',
};

export function NewsFeedPanel() {
  const { t } = useTranslation();
  const [items, setItems] = useState<NewsItem[]>([]);
  const [loadState, setLoadState] = useState<LoadState>('loading');
  const [errorMsg, setErrorMsg] = useState<string>('');
  const [selectedNews, setSelectedNews] = useState<NewsItem | null>(null);
  const [briefData, setBriefData] = useState<AiBriefCategory[]>([]);
  const [briefExpanded, setBriefExpanded] = useState(true);

  const load = useCallback(async () => {
    setLoadState('loading');
    try {
      const data = await getNews();
      setItems(data);
      setLoadState('loaded');
    } catch (err) {
      setErrorMsg(err instanceof Error ? err.message : String(err));
      setLoadState('error');
    }
  }, []);

  useEffect(() => {
    void load();

    let cleanup: (() => void) | null = null;
    const unlistenPromise = listenNewsUpdated(() => void load());
    void unlistenPromise.then((fn) => {
      cleanup = fn;
    });

    let cleanupAi: (() => void) | null = null;
    const unlistenAiPromise = listenAiSummaryCompleted(() => void load());
    void unlistenAiPromise.then((fn) => {
      cleanupAi = fn;
    });

    return () => {
      if (cleanup) {
        cleanup();
      } else {
        void unlistenPromise.then((fn) => fn());
      }
      if (cleanupAi) {
        cleanupAi();
      } else {
        void unlistenAiPromise.then((fn) => fn());
      }
    };
  }, [load]);

  // Load AI brief independently — failure is silent (overview bar just hides)
  useEffect(() => {
    let cancelled = false;
    getAiBrief()
      .then((data) => { if (!cancelled) setBriefData(data); })
      .catch(() => { /* silent — Phase 3 may not be implemented yet */ });
    return () => { cancelled = true; };
  }, []);

  // Always render the modal portal regardless of loadState — it uses
  // createPortal(…, document.body) so it lives outside this component's
  // DOM subtree. Rendering it unconditionally means a click that fires
  // setSelectedNews() right before a loadState transition (e.g. news-updated
  // triggers a reload) will still show the modal instead of losing the state.
  const modal = (
    <NewsDetailModal
      news={selectedNews}
      onClose={() => setSelectedNews(null)}
    />
  );

  // Loading state
  if (loadState === 'loading') {
    return (
      <>
        <div className="news-feed__state">
          <RefreshCw size={16} className="news-feed__spinner" />
          <span className="news-feed__state-text">{t('state.loadingNews')}</span>
        </div>
        {modal}
      </>
    );
  }

  // Error state
  if (loadState === 'error') {
    return (
      <>
        <div className="news-feed__state news-feed__state--error">
          <p className="news-feed__state-text">{t('state.failedNews')}</p>
          {errorMsg && <p className="news-feed__error-detail">{errorMsg}</p>}
          <button className="news-feed__retry-btn" onClick={() => void load()}>
            {t('state.retry')}
          </button>
        </div>
        {modal}
      </>
    );
  }

  // Empty state
  if (items.length === 0) {
    return (
      <>
        <div className="news-feed__state">
          <Inbox size={20} className="news-feed__empty-icon" />
          <p className="news-feed__state-text">{t('state.noNews')}</p>
          <p className="news-feed__state-sub">{t('state.waitingFetch')}</p>
        </div>
        {modal}
      </>
    );
  }

  // Sort: AI-analyzed items first, then by time
  const sorted = [...items].sort((a, b) => {
    const aHasAi = a.aiSummary ? 1 : 0;
    const bHasAi = b.aiSummary ? 1 : 0;
    if (aHasAi !== bHasAi) return bHasAi - aHasAi;
    return 0; // preserve backend order (already by time) within each group
  });

  const displayed = sorted.slice(0, 6);
  const aiCount = displayed.filter(i => i.aiSummary).length;

  // Loaded state — render list
  return (
    <>
      {aiCount > 0 && (
        <div className="news-feed__ai-header">
          <span className="news-feed__ai-header-badge">AI</span>
          <span className="news-feed__ai-header-text">
            {aiCount} / {displayed.length} analyzed
          </span>
        </div>
      )}

      {briefData.length > 0 && (
        <div className="news-feed-overview">
          <button
            className="news-feed-overview__toggle"
            onClick={() => setBriefExpanded((prev) => !prev)}
            aria-expanded={briefExpanded}
          >
            <span className="news-feed-overview__title">{t('newsFeed.aiOverview')}</span>
            <span className="news-feed-overview__chevron">
              {briefExpanded ? <ChevronDown size={10} /> : <ChevronRight size={10} />}
            </span>
          </button>
          {briefExpanded && (
            <div className="news-feed-overview__body">
              {briefData.map((cat) => (
                <div key={cat.category} className="news-feed-overview__row">
                  <span
                    className="news-feed-overview__dot"
                    style={{ background: CATEGORY_COLORS[cat.category] ?? 'var(--color-text-disabled)' }}
                    aria-hidden="true"
                  />
                  <span className="news-feed-overview__cat">{cat.category.replace('_', ' ')}</span>
                  <span className={`news-feed-overview__sentiment news-feed-overview__sentiment--${getSentimentClass(cat.avgSentiment)}`}>
                    {getSentimentLabel(cat.avgSentiment).charAt(0)}
                  </span>
                  <span className="news-feed-overview__count">{cat.count}{t('newsFeed.articles')}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      <ul className="news-feed" aria-label="News feed">
        {displayed.map((item, idx) => {
          const hasAi = !!item.aiSummary;
          return (
            <li
              key={item.id}
              className={`news-feed__item news-feed__item--clickable ${hasAi ? 'news-feed__item--ai' : ''} ${idx < displayed.length - 1 ? 'news-feed__item--divider' : ''}`}
              onClick={() => setSelectedNews(item)}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') setSelectedNews(item);
              }}
            >
              <div
                className="news-feed__category-dot"
                style={{
                  background:
                    CATEGORY_COLORS[item.category] ?? 'var(--color-text-disabled)',
                }}
                aria-hidden="true"
              />
              <div className="news-feed__content">
                <p className="news-feed__title">{item.title}</p>
                {hasAi && (
                  <p className="news-feed__ai-preview">{item.aiSummary}</p>
                )}
                <div className="news-feed__meta">
                  <span className="news-feed__source">
                    {hostnameFromUrl(item.sourceUrl)}
                  </span>
                  <span className="news-feed__time">
                    {formatTimeAgo(item.publishedAt)}
                  </span>
                  {item.sentimentScore != null && (
                    <span className={`news-feed__sentiment news-feed__sentiment--${getSentimentClass(item.sentimentScore)}`}>
                      {getSentimentLabel(item.sentimentScore)}
                    </span>
                  )}
                  {hasAi && <span className="news-feed__ai-badge">AI</span>}
                </div>
              </div>
            </li>
          );
        })}
      </ul>
      {modal}
    </>
  );
}
