import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Search, X } from 'lucide-react';
import './CmdKModal.css';

interface SearchResult {
  id: string;
  type: 'panel' | 'country' | 'source';
  label: string;
  detail?: string;
}

export function CmdKModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  // Static search items
  const allItems: SearchResult[] = useMemo(() => [
    // Panels
    { id: 'cycle-reasoning', type: 'panel', label: t('panel.cycleReasoning'), detail: t('cycle.title') },
    { id: 'news-feed', type: 'panel', label: t('panel.newsFeed') },
    { id: 'ai-brief', type: 'panel', label: t('panel.aiBrief') },
    { id: 'fred-indicators', type: 'panel', label: t('panel.fredIndicators') },
    { id: 'bis-rates', type: 'panel', label: t('panel.bisRates') },
    { id: 'market-radar', type: 'panel', label: t('panel.marketRadar') },
    { id: 'indices', type: 'panel', label: t('panel.indices') },
    { id: 'forex', type: 'panel', label: t('panel.forex') },
    { id: 'oil-energy', type: 'panel', label: t('panel.oilEnergy') },
    { id: 'crypto', type: 'panel', label: t('panel.crypto') },
    { id: 'fear-greed', type: 'panel', label: t('panel.fearGreed') },
    { id: 'wto-trade', type: 'panel', label: t('panel.wtoTrade') },
    { id: 'supply-chain', type: 'panel', label: t('panel.supplyChain') },
    { id: 'gulf-fdi', type: 'panel', label: t('panel.gulfFdi') },
    // Countries
    { id: 'us', type: 'country', label: 'United States', detail: 'NYSE, NASDAQ, Fed' },
    { id: 'uk', type: 'country', label: 'United Kingdom', detail: 'LSE, BoE' },
    { id: 'jp', type: 'country', label: 'Japan', detail: 'TSE, BoJ' },
    { id: 'cn', type: 'country', label: 'China', detail: 'SSE, PBoC' },
    { id: 'de', type: 'country', label: 'Germany', detail: 'FRA, ECB' },
    { id: 'sg', type: 'country', label: 'Singapore', detail: 'SGX' },
    { id: 'ae', type: 'country', label: 'UAE', detail: 'DFM, ADX, DIFC' },
    { id: 'sa', type: 'country', label: 'Saudi Arabia', detail: 'Tadawul, KAFD' },
    // Data sources
    { id: 'fred', type: 'source', label: 'FRED', detail: t('fred.staticNote') },
    { id: 'yahoo', type: 'source', label: 'Yahoo Finance' },
    { id: 'rss', type: 'source', label: 'RSS Feeds' },
    { id: 'eia', type: 'source', label: 'EIA', detail: 'Oil & Energy' },
    { id: 'coingecko', type: 'source', label: 'CoinGecko', detail: 'Crypto' },
    { id: 'ollama', type: 'source', label: 'Ollama', detail: 'Local AI' },
    { id: 'claude', type: 'source', label: 'Claude', detail: 'Deep Analysis' },
  ], [t]);

  const filtered = useMemo(() => {
    if (!query.trim()) return allItems.slice(0, 10);
    const q = query.toLowerCase();
    return allItems.filter(
      (item) => item.label.toLowerCase().includes(q) || item.detail?.toLowerCase().includes(q)
    ).slice(0, 10);
  }, [query, allItems]);

  useEffect(() => {
    if (open) {
      setQuery('');
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter' && filtered[selectedIndex]) {
      onClose();
      // TODO: Navigate to selected item (scroll panel into view, zoom map to country)
    } else if (e.key === 'Escape') {
      onClose();
    }
  }, [filtered, selectedIndex, onClose]);

  if (!open) return null;

  return (
    <div className="cmdk-overlay" onClick={onClose}>
      <div className="cmdk" onClick={(e) => e.stopPropagation()} onKeyDown={handleKeyDown}>
        <div className="cmdk__input-wrap">
          <Search size={14} className="cmdk__search-icon" />
          <input
            ref={inputRef}
            className="cmdk__input"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t('cmdK.placeholder')}
            spellCheck={false}
          />
          <button className="cmdk__close" onClick={onClose}><X size={12} /></button>
        </div>
        <ul className="cmdk__results">
          {filtered.map((item, i) => (
            <li
              key={item.id}
              className={`cmdk__result ${i === selectedIndex ? 'cmdk__result--selected' : ''}`}
              onMouseEnter={() => setSelectedIndex(i)}
              onClick={onClose}
            >
              <span className={`cmdk__type cmdk__type--${item.type}`}>
                {item.type === 'panel' ? '◻' : item.type === 'country' ? '◉' : '◈'}
              </span>
              <span className="cmdk__label">{item.label}</span>
              {item.detail && <span className="cmdk__detail">{item.detail}</span>}
            </li>
          ))}
          {filtered.length === 0 && (
            <li className="cmdk__empty">{t('cmdK.noResults')}</li>
          )}
        </ul>
      </div>
    </div>
  );
}
