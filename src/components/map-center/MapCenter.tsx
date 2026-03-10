import { useState, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import DeckGL from '@deck.gl/react';
import type { PickingInfo, Color } from '@deck.gl/core';
import { Map } from 'react-map-gl/maplibre';
import { ScatterplotLayer, TextLayer } from '@deck.gl/layers';
import 'maplibre-gl/dist/maplibre-gl.css';
import './MapCenter.css';
import { MapDetailCard } from './MapDetailCard';
import type { MapSelection } from './MapDetailCard';

// Free dark map tiles — no API key needed
const MAP_STYLE = 'https://basemaps.cartocdn.com/gl/dark-matter-gl-style/style.json';

const INITIAL_VIEW_STATE = {
  longitude: 0,
  latitude: 20,
  zoom: 2,
  pitch: 0,
  bearing: 0,
};

// Static data: 19 major financial centers
const FINANCIAL_CENTERS = [
  { name: 'New York', coordinates: [-74.006, 40.7128], type: 'exchange', size: 'major', country: 'US', keywords: ['NYSE', 'NASDAQ', 'Wall Street', 'S&P', '纽约', '美股'] },
  { name: 'London', coordinates: [-0.1276, 51.5074], type: 'exchange', size: 'major', country: 'UK', keywords: ['LSE', 'FTSE', '伦敦', '英国'] },
  { name: 'Tokyo', coordinates: [139.6917, 35.6895], type: 'exchange', size: 'major', country: 'JP', keywords: ['Nikkei', 'TSE', '日经', '东京', '日本'] },
  { name: 'Shanghai', coordinates: [121.4737, 31.2304], type: 'exchange', size: 'major', country: 'CN', keywords: ['SSE', 'CSI', '上证', '上海', 'A股'] },
  { name: 'Hong Kong', coordinates: [114.1694, 22.3193], type: 'exchange', size: 'major', country: 'HK', keywords: ['HSI', 'Hang Seng', '恒生', '港股'] },
  { name: 'Singapore', coordinates: [103.8198, 1.3521], type: 'exchange', size: 'major', country: 'SG', keywords: ['SGX', 'STI', '新加坡'] },
  { name: 'Frankfurt', coordinates: [8.6821, 50.1109], type: 'exchange', size: 'medium', country: 'DE', keywords: ['DAX', 'Frankfurt', '法兰克福', '德国'] },
  { name: 'Sydney', coordinates: [151.2093, -33.8688], type: 'exchange', size: 'medium', country: 'AU', keywords: ['ASX', 'Sydney', '悉尼', '澳大利亚'] },
  { name: 'Toronto', coordinates: [-79.3832, 43.6532], type: 'exchange', size: 'medium', country: 'CA', keywords: ['TSX', 'Toronto', '多伦多', '加拿大'] },
  { name: 'Mumbai', coordinates: [72.8777, 19.076], type: 'exchange', size: 'medium', country: 'IN', keywords: ['BSE', 'NSE', 'Sensex', 'Nifty', '孟买', '印度'] },
  { name: 'Zurich', coordinates: [8.5417, 47.3769], type: 'exchange', size: 'medium', country: 'CH', keywords: ['SIX', 'SMI', '苏黎世', '瑞士'] },
  { name: 'Dubai', coordinates: [55.2708, 25.2048], type: 'exchange', size: 'medium', country: 'AE', keywords: ['DFM', 'ADX', '迪拜', '阿联酋'] },
  { name: 'Seoul', coordinates: [126.978, 37.5665], type: 'exchange', size: 'medium', country: 'KR', keywords: ['KOSPI', '首尔', '韩国'] },
  { name: 'São Paulo', coordinates: [-46.6333, -23.5505], type: 'exchange', size: 'medium', country: 'BR', keywords: ['Bovespa', 'B3', '圣保罗', '巴西'] },
  { name: 'Paris', coordinates: [2.3522, 48.8566], type: 'exchange', size: 'medium', country: 'FR', keywords: ['CAC', 'Euronext', '巴黎', '法国'] },
  { name: 'Chicago', coordinates: [-87.6298, 41.8781], type: 'exchange', size: 'medium', country: 'US', keywords: ['CME', 'CBOE', 'VIX', '芝加哥'] },
  { name: 'Johannesburg', coordinates: [28.0473, -26.2041], type: 'exchange', size: 'small', country: 'ZA', keywords: ['JSE', '南非'] },
  { name: 'Taipei', coordinates: [121.5654, 25.033], type: 'exchange', size: 'small', country: 'TW', keywords: ['TWSE', 'TSMC', '台湾', '台北'] },
  { name: 'Jakarta', coordinates: [106.8456, -6.2088], type: 'exchange', size: 'small', country: 'ID', keywords: ['IDX', '雅加达', '印尼'] },
];

// 13 central banks
const CENTRAL_BANKS = [
  { name: 'Federal Reserve', coordinates: [-77.0469, 38.8951], rate: '5.25%', country: 'US', keywords: ['Fed', 'FOMC', '美联储', '联邦基金'] },
  { name: 'ECB', coordinates: [8.6724, 50.1109], rate: '4.50%', country: 'EU', keywords: ['ECB', '欧央行', '欧元区'] },
  { name: 'Bank of Japan', coordinates: [139.7671, 35.6812], rate: '0.10%', country: 'JP', keywords: ['BOJ', '日银', '日本央行', '日本银行'] },
  { name: 'Bank of England', coordinates: [-0.0886, 51.5142], rate: '5.25%', country: 'UK', keywords: ['BOE', '英国央行', '英格兰银行'] },
  { name: "People's Bank of China", coordinates: [116.3912, 39.9042], rate: '3.45%', country: 'CN', keywords: ['PBOC', '央行', '人民银行', '中国人民银行'] },
  { name: 'Reserve Bank of India', coordinates: [72.8347, 18.9322], rate: '6.50%', country: 'IN', keywords: ['RBI', '印度央行', '印度储备银行'] },
  { name: 'Bank of Canada', coordinates: [-75.6972, 45.4215], rate: '5.00%', country: 'CA', keywords: ['BOC', '加拿大央行', 'Bank of Canada'] },
  { name: 'Reserve Bank of Australia', coordinates: [149.1300, -35.2809], rate: '4.35%', country: 'AU', keywords: ['RBA', '澳联储', '澳大利亚央行'] },
  { name: 'Swiss National Bank', coordinates: [7.4474, 46.948], rate: '1.75%', country: 'CH', keywords: ['SNB', '瑞士央行', '瑞士国家银行'] },
  { name: 'Bank of Korea', coordinates: [126.978, 37.5518], rate: '3.50%', country: 'KR', keywords: ['BOK', '韩国央行', '韩国银行'] },
  { name: 'Central Bank of Brazil', coordinates: [-47.8825, -15.7942], rate: '11.75%', country: 'BR', keywords: ['BCB', '巴西央行', 'Banco Central do Brasil'] },
  { name: 'Saudi Central Bank', coordinates: [46.6753, 24.7136], rate: '6.00%', country: 'SA', keywords: ['SAMA', '沙特央行', '沙特阿拉伯货币局'] },
  { name: 'Central Bank of UAE', coordinates: [54.3773, 24.4539], rate: '5.40%', country: 'AE', keywords: ['CBUAE', '阿联酋央行', 'UAE央行'] },
];

// Gulf FDI zones
const GULF_FDI_ZONES = [
  { name: 'DIFC Dubai', coordinates: [55.2819, 25.2135], type: 'fdi', country: 'AE', keywords: ['DIFC', '迪拜', '阿联酋', 'Dubai'] },
  { name: 'ADGM Abu Dhabi', coordinates: [54.6515, 24.4539], type: 'fdi', country: 'AE', keywords: ['ADGM', '阿布扎比', 'Abu Dhabi'] },
  { name: 'KAFD Riyadh', coordinates: [46.6359, 24.7649], type: 'fdi', country: 'SA', keywords: ['KAFD', '利雅得', '沙特', 'Riyadh'] },
  { name: 'QFC Doha', coordinates: [51.4316, 25.2854], type: 'fdi', country: 'QA', keywords: ['QFC', '多哈', '卡塔尔', 'Qatar'] },
  { name: 'Bahrain Financial Harbour', coordinates: [50.5577, 26.2285], type: 'fdi', country: 'BH', keywords: ['BFH', '巴林', 'Bahrain'] },
  { name: 'Kuwait Financial Centre', coordinates: [47.9783, 29.3759], type: 'fdi', country: 'KW', keywords: ['科威特', 'Kuwait', 'KFC'] },
];

type LayerId = 'exchanges' | 'centralBanks' | 'gulfFdi';

interface HoverInfo {
  x: number;
  y: number;
  name: string;
  detail: string;
}

type FinancialCenter = typeof FINANCIAL_CENTERS[0];
type CentralBank = typeof CENTRAL_BANKS[0];
type GulfFdiZone = typeof GULF_FDI_ZONES[0];

export function MapCenter() {
  const { t } = useTranslation();
  const [viewState, setViewState] = useState(INITIAL_VIEW_STATE);
  const [activeLayers, setActiveLayers] = useState<Record<LayerId, boolean>>({
    exchanges: true,
    centralBanks: true,
    gulfFdi: false,
  });
  const [hoverInfo, setHoverInfo] = useState<HoverInfo | null>(null);
  const [selection, setSelection] = useState<MapSelection | null>(null);

  const toggleLayer = useCallback((id: LayerId) => {
    setActiveLayers((prev) => ({ ...prev, [id]: !prev[id] }));
  }, []);

  const layers = useMemo(() => {
    const result = [];

    if (activeLayers.exchanges) {
      result.push(
        new ScatterplotLayer<FinancialCenter>({
          id: 'exchanges',
          data: FINANCIAL_CENTERS,
          getPosition: (d) => d.coordinates as [number, number],
          getRadius: (d) =>
            d.size === 'major' ? 80000 : d.size === 'medium' ? 50000 : 30000,
          getFillColor: [0, 212, 170, 180] as Color,
          getLineColor: [0, 212, 170, 255] as Color,
          lineWidthMinPixels: 1,
          stroked: true,
          pickable: true,
          radiusMinPixels: 4,
          radiusMaxPixels: 20,
        }),
        new TextLayer<FinancialCenter>({
          id: 'exchange-labels',
          data: FINANCIAL_CENTERS.filter((d) => d.size === 'major'),
          getPosition: (d) => d.coordinates as [number, number],
          getText: (d) => d.name,
          getSize: 11,
          getColor: [200, 220, 240, 200] as Color,
          getTextAnchor: 'start',
          getAlignmentBaseline: 'center',
          getPixelOffset: [10, 0] as [number, number],
          fontFamily: 'monospace',
        })
      );
    }

    if (activeLayers.centralBanks) {
      result.push(
        new ScatterplotLayer<CentralBank>({
          id: 'central-banks',
          data: CENTRAL_BANKS,
          getPosition: (d) => d.coordinates as [number, number],
          getRadius: 60000,
          getFillColor: [191, 90, 242, 160] as Color,
          getLineColor: [191, 90, 242, 255] as Color,
          lineWidthMinPixels: 1,
          stroked: true,
          pickable: true,
          radiusMinPixels: 5,
          radiusMaxPixels: 15,
        })
      );
    }

    if (activeLayers.gulfFdi) {
      result.push(
        new ScatterplotLayer<GulfFdiZone>({
          id: 'gulf-fdi',
          data: GULF_FDI_ZONES,
          getPosition: (d) => d.coordinates as [number, number],
          getRadius: 40000,
          getFillColor: [255, 184, 0, 160] as Color,
          getLineColor: [255, 184, 0, 255] as Color,
          lineWidthMinPixels: 1,
          stroked: true,
          pickable: true,
          radiusMinPixels: 4,
          radiusMaxPixels: 12,
        })
      );
    }

    return result;
  }, [activeLayers]);

  const onHover = useCallback((info: PickingInfo) => {
    if (info.object) {
      const obj = info.object as Record<string, unknown>;
      const name = obj['name'] as string;
      const rate = obj['rate'] as string | undefined;
      const size = obj['size'] as string | undefined;
      const type = obj['type'] as string | undefined;
      const detail = rate ? `Rate: ${rate}` : size ? size.toUpperCase() : type ?? '';
      setHoverInfo({ x: info.x, y: info.y, name, detail });
    } else {
      setHoverInfo(null);
    }
  }, []);

  const onClick = useCallback((info: PickingInfo) => {
    if (info.object) {
      const obj = info.object as Record<string, unknown>;
      const name = obj['name'] as string;
      const country = obj['country'] as string;
      const keywords = (obj['keywords'] as string[]) || [];
      const rate = obj['rate'] as string | undefined;
      const size = obj['size'] as string | undefined;
      const type = obj['type'] as string | undefined;

      let layerType: MapSelection['layerType'] = 'exchange';
      if (rate) layerType = 'centralBank';
      else if (type === 'fdi') layerType = 'gulfFdi';

      setSelection({
        name,
        layerType,
        country,
        keywords,
        rate,
        size,
        x: info.x,
        y: info.y,
      });
    } else {
      // Click on empty map → close detail card
      setSelection(null);
    }
  }, []);

  return (
    <div className="map-center">
      <DeckGL
        viewState={viewState}
        onViewStateChange={(e) => setViewState(e.viewState as typeof INITIAL_VIEW_STATE)}
        controller={true}
        layers={layers}
        onHover={onHover}
        onClick={onClick}
        getCursor={() => (hoverInfo ? 'pointer' : 'grab')}
      >
        <Map
          mapStyle={MAP_STYLE}
          attributionControl={false}
        />
      </DeckGL>

      {/* Hover tooltip */}
      {hoverInfo && (
        <div
          className="map-center__tooltip"
          style={{ left: hoverInfo.x + 12, top: hoverInfo.y - 12 }}
        >
          <div className="map-center__tooltip-name">{hoverInfo.name}</div>
          {hoverInfo.detail && (
            <div className="map-center__tooltip-detail">{hoverInfo.detail}</div>
          )}
        </div>
      )}

      {/* Click detail card */}
      {selection && (
        <MapDetailCard
          selection={selection}
          onClose={() => setSelection(null)}
        />
      )}

      {/* Layer toggle controls */}
      <div className="map-center__controls">
        <span className="map-center__controls-title">{t('map.layers')}</span>
        {([
          { id: 'exchanges' as LayerId, label: t('map.layerExchanges'), color: '#00d4aa' },
          { id: 'centralBanks' as LayerId, label: t('map.layerCentralBanks'), color: '#bf5af2' },
          { id: 'gulfFdi' as LayerId, label: t('map.layerGulfFdi'), color: '#ffb800' },
        ]).map(({ id, label, color }) => (
          <button
            key={id}
            className={`map-center__layer-btn ${activeLayers[id] ? 'map-center__layer-btn--active' : ''}`}
            onClick={() => toggleLayer(id)}
          >
            <span
              className="map-center__layer-dot"
              style={{ backgroundColor: activeLayers[id] ? color : 'transparent', borderColor: color }}
            />
            {label}
          </button>
        ))}
      </div>
    </div>
  );
}
