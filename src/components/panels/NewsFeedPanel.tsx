import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw, Inbox } from 'lucide-react';
import { getNews, listenNewsUpdated, hostnameFromUrl, formatTimeAgo } from '@services/tauri-bridge';
import type { NewsItem } from '@contracts/api-news';
import { NewsDetailModal } from '@components/news-detail';
import './NewsFeedPanel.css';

type LoadState = 'loading' | 'loaded' | 'error';

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

    return () => {
      if (cleanup) {
        cleanup();
      } else {
        void unlistenPromise.then((fn) => fn());
      }
    };
  }, [load]);

  // Loading state
  if (loadState === 'loading') {
    return (
      <div className="news-feed__state">
        <RefreshCw size={16} className="news-feed__spinner" />
        <span className="news-feed__state-text">{t('state.loadingNews')}</span>
      </div>
    );
  }

  // Error state
  if (loadState === 'error') {
    return (
      <div className="news-feed__state news-feed__state--error">
        <p className="news-feed__state-text">{t('state.failedNews')}</p>
        {errorMsg && <p className="news-feed__error-detail">{errorMsg}</p>}
        <button className="news-feed__retry-btn" onClick={() => void load()}>
          {t('state.retry')}
        </button>
      </div>
    );
  }

  // Empty state
  if (items.length === 0) {
    return (
      <div className="news-feed__state">
        <Inbox size={20} className="news-feed__empty-icon" />
        <p className="news-feed__state-text">{t('state.noNews')}</p>
        <p className="news-feed__state-sub">{t('state.waitingFetch')}</p>
      </div>
    );
  }

  // Loaded state — render list
  return (
    <>
      <ul className="news-feed" aria-label="News feed">
        {items.map((item, idx) => (
          <li
            key={item.id}
            className={`news-feed__item news-feed__item--clickable ${idx < items.length - 1 ? 'news-feed__item--divider' : ''}`}
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
              <div className="news-feed__meta">
                <span className="news-feed__source">
                  {hostnameFromUrl(item.sourceUrl)}
                </span>
                <span className="news-feed__time">
                  {formatTimeAgo(item.publishedAt)}
                </span>
              </div>
            </div>
          </li>
        ))}
      </ul>
      <NewsDetailModal
        news={selectedNews}
        onClose={() => setSelectedNews(null)}
      />
    </>
  );
}
