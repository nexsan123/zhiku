import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { X, Newspaper, TrendingUp, Building2, RefreshCw } from 'lucide-react';
import { getNews, getMarketData, getMacroData } from '@services/tauri-bridge';
import type { MarketDataItem, MacroDataItem } from '@services/tauri-bridge';
import type { NewsItem } from '@contracts/api-news';

export interface MapSelection {
  name: string;
  layerType: 'exchange' | 'centralBank' | 'gulfFdi';
  country: string;
  keywords: string[];
  rate?: string;
  size?: string;
  x: number;
  y: number;
}

interface Props {
  selection: MapSelection;
  onClose: () => void;
}

// Map country codes to Yahoo Finance-style symbol prefixes for filtering
const SYMBOL_MAP: Record<string, string[]> = {
  US: ['^GSPC', '^DJI', '^IXIC', '^VIX', 'EURUSD=X', 'DX-Y.NYB'],
  UK: ['^FTSE'],
  JP: ['^N225', 'USDJPY=X'],
  CN: ['000001.SS'],
  HK: ['^HSI'],
  DE: ['^GDAXI'],
  EU: ['^STOXX50E', 'EURUSD=X'],
  AU: ['^AXJO', 'AUDUSD=X'],
  CA: ['^GSPTSE', 'USDCAD=X'],
  IN: ['^BSESN'],
  KR: ['^KS11'],
  BR: ['^BVSP'],
  FR: ['^FCHI'],
  CH: ['^SSMI'],
};

export function MapDetailCard({ selection, onClose }: Props) {
  const { t } = useTranslation();
  const [relatedNews, setRelatedNews] = useState<NewsItem[]>([]);
  const [relatedMarket, setRelatedMarket] = useState<MarketDataItem[]>([]);
  const [macroData, setMacroData] = useState<MacroDataItem[]>([]);
  const [loading, setLoading] = useState(true);

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [allNews, allMarket, allMacro] = await Promise.all([
        getNews(),
        getMarketData(),
        getMacroData(),
      ]);

      // Filter news by keywords (match title against any keyword, case-insensitive)
      const kws = selection.keywords.map((k) => k.toLowerCase());
      const filtered = allNews
        .filter((n) => kws.some((kw) => n.title.toLowerCase().includes(kw)))
        .slice(0, 5);
      setRelatedNews(filtered);

      // Filter market data by country symbol prefixes
      const symbols = SYMBOL_MAP[selection.country] ?? [];
      const marketFiltered = allMarket
        .filter((m) => symbols.some((s) => m.symbol.includes(s)))
        .slice(0, 4);
      setRelatedMarket(marketFiltered);

      // Show macro data only for US (most FRED data is US-specific)
      if (selection.country === 'US') {
        const fredIndicators = new Set(['FEDFUNDS', 'CPIAUCSL', 'UNRATE', 'GDP']);
        setMacroData(allMacro.filter((m) => fredIndicators.has(m.indicator)));
      } else {
        setMacroData([]);
      }
    } catch (e) {
      console.warn('MapDetailCard load error:', e);
    } finally {
      setLoading(false);
    }
  }, [selection]);

  useEffect(() => {
    void loadData();
  }, [loadData]);

  // Position the card — clamp to viewport edges
  const cardStyle: React.CSSProperties = {
    position: 'absolute',
    left: Math.min(selection.x + 16, window.innerWidth - 340),
    top: Math.max(Math.min(selection.y - 40, window.innerHeight - 500), 60),
    zIndex: 20,
  };

  return (
    <div className="map-detail" style={cardStyle}>
      {/* Header */}
      <div className="map-detail__header">
        <div className="map-detail__title">
          <span className={`map-detail__dot map-detail__dot--${selection.layerType}`} />
          {selection.name}
        </div>
        <button className="map-detail__close" onClick={onClose} aria-label="Close">
          <X size={12} />
        </button>
      </div>

      {/* Meta info */}
      <div className="map-detail__meta">
        <span className="map-detail__country">{selection.country}</span>
        {selection.rate && (
          <span className="map-detail__rate">
            {t('map.rate')}: {selection.rate}
          </span>
        )}
        {selection.size && (
          <span className="map-detail__size">{selection.size.toUpperCase()}</span>
        )}
      </div>

      {loading ? (
        <div className="map-detail__loading">
          <RefreshCw size={12} className="map-detail__spinner" />
          <span>{t('state.loading')}</span>
        </div>
      ) : (
        <>
          {/* Related Market Data */}
          {relatedMarket.length > 0 && (
            <div className="map-detail__section">
              <div className="map-detail__section-title">
                <TrendingUp size={11} />
                {t('map.marketData')}
              </div>
              <div className="map-detail__market-grid">
                {relatedMarket.map((m) => (
                  <div key={m.symbol} className="map-detail__market-item">
                    <span className="map-detail__symbol">
                      {m.symbol.replace('^', '').replace('=X', '')}
                    </span>
                    <span className="map-detail__price">
                      {m.price.toLocaleString(undefined, { maximumFractionDigits: 2 })}
                    </span>
                    <span
                      className={`map-detail__change ${
                        (m.changePct ?? 0) >= 0
                          ? 'map-detail__change--up'
                          : 'map-detail__change--down'
                      }`}
                    >
                      {(m.changePct ?? 0) >= 0 ? '+' : ''}
                      {(m.changePct ?? 0).toFixed(2)}%
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Macro Indicators (US only) */}
          {macroData.length > 0 && (
            <div className="map-detail__section">
              <div className="map-detail__section-title">
                <Building2 size={11} />
                {t('map.macroIndicators')}
              </div>
              <div className="map-detail__macro-grid">
                {macroData.map((m) => (
                  <div key={m.indicator} className="map-detail__macro-item">
                    <span className="map-detail__indicator">{m.indicator}</span>
                    <span className="map-detail__value">{m.value.toFixed(2)}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Related News */}
          <div className="map-detail__section">
            <div className="map-detail__section-title">
              <Newspaper size={11} />
              {t('map.relatedNews')} ({relatedNews.length})
            </div>
            {relatedNews.length > 0 ? (
              <ul className="map-detail__news-list">
                {relatedNews.map((n) => (
                  <li
                    key={n.id}
                    className="map-detail__news-item map-detail__news-item--clickable"
                    onClick={() => {
                      if (n.sourceUrl) window.open(n.sourceUrl, '_blank', 'noopener,noreferrer');
                    }}
                    role="button"
                    tabIndex={0}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        if (n.sourceUrl) window.open(n.sourceUrl, '_blank', 'noopener,noreferrer');
                      }
                    }}
                  >
                    <span className="map-detail__news-title">{n.title}</span>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="map-detail__empty">{t('map.noRelatedNews')}</p>
            )}
          </div>
        </>
      )}
    </div>
  );
}
