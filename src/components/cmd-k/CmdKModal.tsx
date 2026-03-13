import { useState, useEffect, useRef, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Search, X } from 'lucide-react';
import { useAppStore } from '@stores/app-store';
import type { SituationTab } from '@stores/app-store';
import './CmdKModal.css';

interface SearchResult {
  id: string;
  type: 'panel' | 'country' | 'source';
  label: string;
  detail?: string;
}

// Map panel IDs to situation center tabs (panels inside SituationCenter)
const SITUATION_TAB_MAP: Record<string, SituationTab> = {
  'cycle-reasoning': 'cycle',
  'credit-cycle': 'credit',
  'intel-brief': 'intel',
  'game-map': 'gameMap',
};

export function CmdKModal({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const togglePanel = useAppStore((s) => s.togglePanel);
  const panels = useAppStore((s) => s.panels);
  const setSituationTab = useAppStore((s) => s.setSituationTab);
  const leftPanelCollapsed = useAppStore((s) => s.leftPanelCollapsed);
  const rightPanelCollapsed = useAppStore((s) => s.rightPanelCollapsed);
  const toggleLeftPanel = useAppStore((s) => s.toggleLeftPanel);
  const toggleRightPanel = useAppStore((s) => s.toggleRightPanel);
  const openSettings = useAppStore((s) => s.openSettings);

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
    { id: 'credit-cycle', type: 'panel', label: t('panel.creditCycle'), detail: t('creditCycle.globalPhase') },
    { id: 'intel-brief', type: 'panel', label: t('panel.intelBrief'), detail: t('intel.aiGenerated') },
    { id: 'game-map', type: 'panel', label: t('panel.gameMap'), detail: t('gameMap.policyVectors') },
    // Countries
    { id: 'US', type: 'country', label: t('country.US'), detail: 'NYSE, NASDAQ, Fed' },
    { id: 'GB', type: 'country', label: t('country.GB'), detail: 'LSE, BoE' },
    { id: 'JP', type: 'country', label: t('country.JP'), detail: 'TSE, BoJ' },
    { id: 'CN', type: 'country', label: t('country.CN'), detail: 'SSE, PBoC' },
    { id: 'XM', type: 'country', label: t('country.XM'), detail: 'ECB, STOXX' },
    { id: 'AE', type: 'country', label: t('country.AE'), detail: 'DFM, ADX, DIFC' },
    { id: 'SA', type: 'country', label: t('country.SA'), detail: 'Tadawul, KAFD' },
    { id: 'IN', type: 'country', label: t('country.IN'), detail: 'BSE, RBI' },
    // Data sources
    { id: 'fred', type: 'source', label: 'FRED', detail: t('fred.staticNote') },
    { id: 'imf', type: 'source', label: 'IMF WEO', detail: t('creditCycle.incomeSection') },
    { id: 'yahoo', type: 'source', label: 'Yahoo Finance', detail: t('panel.marketRadar') },
    { id: 'rss', type: 'source', label: 'RSS Feeds', detail: t('panel.newsFeed') },
    { id: 'eia', type: 'source', label: 'EIA', detail: t('panel.oilEnergy') },
    { id: 'coingecko', type: 'source', label: 'CoinGecko', detail: t('panel.crypto') },
    { id: 'ollama', type: 'source', label: 'Ollama', detail: t('settings.local') + ' AI' },
    { id: 'claude', type: 'source', label: 'Claude', detail: t('intel.aiGenerated') },
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

  const navigateTo = useCallback((item: SearchResult) => {
    onClose();
    if (item.type === 'panel') {
      const sitTab = SITUATION_TAB_MAP[item.id];
      if (sitTab) {
        // Panel lives inside SituationCenter — switch tab + ensure left panel open
        setSituationTab(sitTab);
        if (leftPanelCollapsed) toggleLeftPanel();
      } else {
        // Standalone panel — ensure its side is open + expand it
        const leftPanels = ['news-feed', 'ai-brief', 'fred-indicators', 'bis-rates', 'situation-center'];
        const isLeft = leftPanels.includes(item.id);
        if (isLeft && leftPanelCollapsed) toggleLeftPanel();
        if (!isLeft && rightPanelCollapsed) toggleRightPanel();
        // Expand if collapsed
        if (panels[item.id as keyof typeof panels] && !panels[item.id as keyof typeof panels].expanded) {
          togglePanel(item.id as Parameters<typeof togglePanel>[0]);
        }
        // Scroll panel into view
        setTimeout(() => {
          document.getElementById(`panel-${item.id}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }, 100);
      }
    } else if (item.type === 'source') {
      openSettings('data-sources');
    }
    // country type: no specific action yet (could zoom map in Phase 5)
  }, [onClose, setSituationTab, leftPanelCollapsed, rightPanelCollapsed, toggleLeftPanel, toggleRightPanel, panels, togglePanel, openSettings]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter' && filtered[selectedIndex]) {
      navigateTo(filtered[selectedIndex]);
    } else if (e.key === 'Escape') {
      onClose();
    }
  }, [filtered, selectedIndex, onClose, navigateTo]);

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
              onClick={() => navigateTo(item)}
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
