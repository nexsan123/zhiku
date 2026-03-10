import { useEffect, useCallback } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { X, ExternalLink } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { isTauri, hostnameFromUrl, formatTimeAgo } from '@services/tauri-bridge';
import type { NewsItem } from '@contracts/api-news';
import './NewsDetailModal.css';

const CATEGORY_COLORS: Record<string, string> = {
  geopolitical: 'var(--color-intel-geopolitical)',
  macro_policy: 'var(--color-intel-macro-policy)',
  market: 'var(--color-intel-market)',
  corporate: 'var(--color-intel-corporate)',
};

const CATEGORY_LABELS: Record<string, string> = {
  geopolitical: 'GEOPOLITICAL',
  macro_policy: 'MACRO POLICY',
  market: 'MARKET',
  corporate: 'CORPORATE',
};

interface Props {
  news: NewsItem | null;
  onClose: () => void;
}

export function NewsDetailModal({ news, onClose }: Props) {
  const { t } = useTranslation();

  const handleOpenUrl = useCallback(() => {
    if (!news?.sourceUrl) return;
    const url = news.sourceUrl;
    if (isTauri()) {
      invoke('open_url', { url }).catch(() => {
        window.open(url, '_blank', 'noopener,noreferrer');
      });
    } else {
      window.open(url, '_blank', 'noopener,noreferrer');
    }
  }, [news?.sourceUrl]);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    if (news) {
      document.addEventListener('keydown', handler);
    }
    return () => {
      document.removeEventListener('keydown', handler);
    };
  }, [news, onClose]);

  if (!news) return null;

  const categoryColor = CATEGORY_COLORS[news.category] ?? 'var(--color-text-disabled)';
  const categoryLabel = CATEGORY_LABELS[news.category] ?? news.category.toUpperCase();

  return createPortal(
    <div
      className="news-detail-modal__overlay"
      onClick={onClose}
      role="presentation"
    >
      <div
        className="news-detail-modal"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label={news.title}
      >
        {/* Header: category badge + close */}
        <div className="news-detail-modal__header">
          <span
            className="news-detail-modal__category"
            style={{ background: categoryColor }}
          >
            {categoryLabel}
          </span>
          <button
            className="news-detail-modal__close"
            onClick={onClose}
            aria-label="Close"
          >
            <X size={14} />
          </button>
        </div>

        {/* Title */}
        <h2 className="news-detail-modal__title">{news.title}</h2>

        {/* Meta: source + time */}
        <div className="news-detail-modal__meta">
          <span className="news-detail-modal__source">
            {hostnameFromUrl(news.sourceUrl)}
          </span>
          <span className="news-detail-modal__separator">·</span>
          <span className="news-detail-modal__time">
            {formatTimeAgo(news.publishedAt)}
          </span>
        </div>

        {/* Summary */}
        <p className="news-detail-modal__summary">
          {news.summary || t('newsDetail.noSummary')}
        </p>

        {/* Footer action */}
        <div className="news-detail-modal__footer">
          <button
            className="news-detail-modal__open-btn"
            onClick={handleOpenUrl}
          >
            <ExternalLink size={12} />
            {t('newsDetail.openInBrowser')}
          </button>
        </div>
      </div>
    </div>,
    document.body,
  );
}
