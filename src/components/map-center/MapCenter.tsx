import { useState, useCallback, useMemo, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import DeckGL from '@deck.gl/react';
import type { PickingInfo, Color } from '@deck.gl/core';
import { Map } from 'react-map-gl/maplibre';
import { ScatterplotLayer, TextLayer, ArcLayer } from '@deck.gl/layers';
import 'maplibre-gl/dist/maplibre-gl.css';
import './MapCenter.css';
import {
  getCreditCycleOverview,
  getBilateralDynamics,
  getNewsHeatmap,
  type GlobalCycleOverview,
  type CountryCyclePosition,
  type BilateralDynamic,
  type NewsHeatmapEntry,
} from '@services/tauri-bridge';

function isWebGLAvailable(): boolean {
  try {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
    return gl !== null;
  } catch {
    return false;
  }
}
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

// Phase-to-RGBA color mapping for credit cycle dots
const PHASE_COLORS: Record<string, [number, number, number, number]> = {
  easing:       [52,  199, 89,  200],
  leveraging:   [0,   212, 170, 200],
  overheating:  [255, 69,  58,  200],
  tightening:   [191, 90,  242, 200],
  deleveraging: [255, 159, 10,  200],
  clearing:     [90,  200, 250, 200],
  unknown:      [100, 100, 100, 160],
};

function getPhaseColor(phase: string): [number, number, number, number] {
  return PHASE_COLORS[phase] ?? PHASE_COLORS['unknown'];
}

// Coordinates for the 15 BIS credit cycle countries
const CREDIT_CYCLE_LOCATIONS: Record<string, [number, number]> = {
  US: [-77.0469, 38.8951],       // Federal Reserve (Washington DC)
  CN: [116.3912, 39.9042],       // PBoC (Beijing)
  XM: [8.6821, 50.1109],         // Frankfurt (Euro Area proxy)
  JP: [139.7671, 35.6812],       // Bank of Japan (Tokyo)
  GB: [-0.0886, 51.5142],        // Bank of England (London)
  CA: [-75.6972, 45.4215],       // Bank of Canada (Ottawa)
  AU: [149.1300, -35.2809],      // Reserve Bank of Australia (Canberra)
  KR: [126.978, 37.5518],        // Bank of Korea (Seoul)
  IN: [72.8347, 18.9322],        // Reserve Bank of India (Mumbai)
  BR: [-47.8825, -15.7942],      // Central Bank of Brazil (Brasília)
  TR: [32.8597, 39.9334],        // Ankara, Turkey
  AR: [-58.3816, -34.6037],      // Buenos Aires, Argentina
  ZA: [28.0473, -26.2041],       // Johannesburg, South Africa (JSE)
  SA: [46.6753, 24.7136],        // Saudi Central Bank (Riyadh)
  AE: [54.3773, 24.4539],        // Central Bank of UAE (Abu Dhabi)
};

// Human-readable country names
const COUNTRY_NAMES: Record<string, string> = {
  US: 'United States', CN: 'China', XM: 'Euro Area', JP: 'Japan',
  GB: 'United Kingdom', CA: 'Canada', AU: 'Australia', KR: 'South Korea',
  IN: 'India', BR: 'Brazil', TR: 'Turkey', AR: 'Argentina',
  ZA: 'South Africa', SA: 'Saudi Arabia', AE: 'UAE',
};

// Alias mapping: bilateral id tokens → CREDIT_CYCLE_LOCATIONS keys
// 'eu' from 'us-eu' maps to XM (Euro Area); 'me' maps to SA (Saudi Arabia as proxy)
const COUNTRY_ALIASES: Record<string, string> = {
  EU: 'XM',
  ME: 'SA',
};

function resolveCountryCode(raw: string): string {
  const upper = raw.toUpperCase();
  return COUNTRY_ALIASES[upper] ?? upper;
}

// Parse bilateral id (e.g. 'us-cn', 'us_eu') into source/target coordinates
function parseBilateralEndpoints(
  id: string,
): { source: [number, number]; target: [number, number] } | null {
  const parts = id.split(/[-_]/);
  if (parts.length < 2) return null;
  const srcKey = resolveCountryCode(parts[0]);
  const tgtKey = resolveCountryCode(parts[1]);
  const src = CREDIT_CYCLE_LOCATIONS[srcKey];
  const tgt = CREDIT_CYCLE_LOCATIONS[tgtKey];
  if (!src || !tgt) return null;
  return { source: src, target: tgt };
}

type LayerId = 'creditCycle' | 'newsHeatmap' | 'tensionArcs';

interface HoverInfo {
  x: number;
  y: number;
  name: string;
  detail: string;
}

// Shape of each data item fed to the credit cycle ScatterplotLayer
interface CreditCycleDot {
  countryCode: string;
  coordinates: [number, number];
  phase: string;
  phaseLabel: string;
  confidence: number;
  tier: string;
}

// ArcLayer data item (bilateral + resolved endpoints)
interface ArcDatum extends BilateralDynamic {
  source: [number, number];
  target: [number, number];
}

// ScatterplotLayer data item for news heatmap (entry + resolved coordinates)
interface HeatmapDot extends NewsHeatmapEntry {
  coordinates: [number, number];
}

export function MapCenter() {
  const { t } = useTranslation();
  const [webglOk, setWebglOk] = useState(true);
  const [viewState, setViewState] = useState(INITIAL_VIEW_STATE);

  useEffect(() => {
    setWebglOk(isWebGLAvailable());
  }, []);

  const [activeLayers, setActiveLayers] = useState<Record<LayerId, boolean>>({
    creditCycle: true,
    newsHeatmap: true,
    tensionArcs: true,
  });
  const [hoverInfo, setHoverInfo] = useState<HoverInfo | null>(null);
  const [selection, setSelection] = useState<MapSelection | null>(null);

  // ---- Credit cycle live data ----
  const [creditCycleData, setCreditCycleData] = useState<GlobalCycleOverview | null>(null);
  const pollingRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function fetchCreditCycle() {
      try {
        const data = await getCreditCycleOverview();
        if (!cancelled) setCreditCycleData(data);
      } catch {
        // Silent fail — layer simply won't render
      }
    }

    void fetchCreditCycle();
    pollingRef.current = setInterval(() => { void fetchCreditCycle(); }, 60_000);

    return () => {
      cancelled = true;
      if (pollingRef.current !== null) {
        clearInterval(pollingRef.current);
        pollingRef.current = null;
      }
    };
  }, []);

  // ---- Bilateral dynamics (tension arcs) ----
  const [bilateralData, setBilateralData] = useState<BilateralDynamic[]>([]);

  useEffect(() => {
    let cancelled = false;

    async function fetchBilaterals() {
      try {
        const data = await getBilateralDynamics();
        if (!cancelled) setBilateralData(data);
      } catch {
        // Silent fail
      }
    }

    void fetchBilaterals();
    const interval = setInterval(() => { void fetchBilaterals(); }, 120_000); // 2 min

    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  // ---- News heatmap ----
  const [newsHeatmapData, setNewsHeatmapData] = useState<NewsHeatmapEntry[]>([]);

  useEffect(() => {
    let cancelled = false;

    async function fetchHeatmap() {
      try {
        const data = await getNewsHeatmap(1);
        if (!cancelled) setNewsHeatmapData(data);
      } catch {
        // Silent fail
      }
    }

    void fetchHeatmap();
    const interval = setInterval(() => { void fetchHeatmap(); }, 60_000); // 1 min

    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  // ---- Pulse animation for news heatmap (800ms toggle) ----
  const [pulseScale, setPulseScale] = useState(1.0);

  useEffect(() => {
    const interval = setInterval(() => {
      setPulseScale((prev) => (prev === 1.0 ? 1.3 : 1.0));
    }, 800);
    return () => clearInterval(interval);
  }, []);

  const toggleLayer = useCallback((id: LayerId) => {
    setActiveLayers((prev) => ({ ...prev, [id]: !prev[id] }));
  }, []);

  // Build credit cycle dot data from live backend response
  const creditCycleDots = useMemo<CreditCycleDot[]>(() => {
    if (!creditCycleData) return [];
    return creditCycleData.countries
      .filter((c: CountryCyclePosition) => c.countryCode in CREDIT_CYCLE_LOCATIONS)
      .map((c: CountryCyclePosition) => ({
        countryCode: c.countryCode,
        coordinates: CREDIT_CYCLE_LOCATIONS[c.countryCode],
        phase: c.phase,
        phaseLabel: c.phaseLabel,
        confidence: c.confidence,
        tier: c.tier,
      }));
  }, [creditCycleData]);

  // Build arc data from bilateral dynamics
  const arcData = useMemo<ArcDatum[]>(() => {
    return bilateralData
      .map((b) => {
        const endpoints = parseBilateralEndpoints(b.id);
        if (!endpoints) return null;
        return { ...b, ...endpoints } as ArcDatum;
      })
      .filter((d): d is ArcDatum => d !== null);
  }, [bilateralData]);

  // Build heatmap dots from news heatmap data
  const heatmapDots = useMemo<HeatmapDot[]>(() => {
    return newsHeatmapData
      .filter((e) => e.countryCode in CREDIT_CYCLE_LOCATIONS)
      .map((e) => ({
        ...e,
        coordinates: CREDIT_CYCLE_LOCATIONS[e.countryCode],
      }));
  }, [newsHeatmapData]);

  const layers = useMemo(() => {
    const result = [];

    // ---- Credit cycle: ScatterplotLayer + TextLayer ----
    if (activeLayers.creditCycle && creditCycleDots.length > 0) {
      result.push(
        new ScatterplotLayer<CreditCycleDot>({
          id: 'credit-cycle',
          data: creditCycleDots,
          getPosition: (d) => d.coordinates,
          getRadius: (d) =>
            d.tier === 'core' ? 100000 : d.tier === 'important' ? 70000 : 50000,
          getFillColor: (d) => getPhaseColor(d.phase) as Color,
          getLineColor: (d) => {
            const c = getPhaseColor(d.phase);
            return [c[0], c[1], c[2], 255] as Color;
          },
          lineWidthMinPixels: 1,
          stroked: true,
          pickable: true,
          radiusMinPixels: 4,
          radiusMaxPixels: 24,
        }),
        new TextLayer<CreditCycleDot>({
          id: 'credit-cycle-labels',
          data: creditCycleDots,
          getPosition: (d) => d.coordinates,
          getText: (d) => d.countryCode,
          getSize: 10,
          getColor: [220, 230, 245, 210] as Color,
          getTextAnchor: 'start',
          getAlignmentBaseline: 'center',
          getPixelOffset: [10, 0] as [number, number],
          fontFamily: 'monospace',
        }),
      );
    }

    // ---- News heatmap: pulsing ScatterplotLayer ----
    if (activeLayers.newsHeatmap && heatmapDots.length > 0) {
      result.push(
        new ScatterplotLayer<HeatmapDot>({
          id: 'news-heatmap',
          data: heatmapDots,
          getPosition: (d) => d.coordinates,
          getRadius: (d) => 40000 + d.newsCount * 15000,
          getFillColor: (d) => {
            if (d.avgSentiment < 0.4) return [255, 69, 58, 120] as Color;
            if (d.avgSentiment > 0.6) return [52, 199, 89, 120] as Color;
            return [255, 204, 0, 120] as Color;
          },
          radiusScale: pulseScale,
          radiusMinPixels: 8,
          radiusMaxPixels: 40,
          pickable: true,
          transitions: {
            getRadius: 800,
          },
        }),
      );
    }

    // ---- Tension arcs: ArcLayer ----
    if (activeLayers.tensionArcs && arcData.length > 0) {
      result.push(
        new ArcLayer<ArcDatum>({
          id: 'tension-arcs',
          data: arcData,
          getSourcePosition: (d) => d.source,
          getTargetPosition: (d) => d.target,
          getSourceColor: (d) =>
            d.tension > 0.6
              ? ([255, 69, 58, 200] as Color)
              : d.tension > 0.3
                ? ([255, 159, 10, 200] as Color)
                : ([52, 199, 89, 200] as Color),
          getTargetColor: (d) =>
            d.tension > 0.6
              ? ([255, 69, 58, 200] as Color)
              : d.tension > 0.3
                ? ([255, 159, 10, 200] as Color)
                : ([52, 199, 89, 200] as Color),
          getWidth: (d) => 1 + d.tension * 5,
          greatCircle: true,
          pickable: true,
        }),
      );
    }

    return result;
  }, [activeLayers, creditCycleDots, heatmapDots, arcData, pulseScale]);

  const onHover = useCallback((info: PickingInfo) => {
    if (info.object) {
      const obj = info.object as Record<string, unknown>;

      // Credit cycle dot
      if ('countryCode' in obj && 'phaseLabel' in obj) {
        const code = obj['countryCode'] as string;
        const phaseLabel = obj['phaseLabel'] as string;
        const countryName = COUNTRY_NAMES[code] ?? code;
        setHoverInfo({ x: info.x, y: info.y, name: countryName, detail: phaseLabel });
        return;
      }

      // News heatmap dot
      if ('newsCount' in obj) {
        const code = obj['countryCode'] as string;
        const count = obj['newsCount'] as number;
        const latestTitle = obj['latestTitle'] as string;
        const countryName = COUNTRY_NAMES[code] ?? code;
        setHoverInfo({ x: info.x, y: info.y, name: countryName, detail: `${count} news · ${latestTitle}` });
        return;
      }

      // Tension arc
      if ('tension' in obj) {
        const name = obj['name'] as string;
        const tension = obj['tension'] as number;
        const tensionLabel = obj['tensionLabel'] as string;
        setHoverInfo({ x: info.x, y: info.y, name, detail: `${tensionLabel} (${Math.round(tension * 100)}%)` });
        return;
      }
    } else {
      setHoverInfo(null);
    }
  }, []);

  const onClick = useCallback((info: PickingInfo) => {
    if (info.object) {
      const obj = info.object as Record<string, unknown>;

      // Credit cycle dot click
      if ('countryCode' in obj && 'phaseLabel' in obj) {
        const code = obj['countryCode'] as string;
        const phaseLabel = obj['phaseLabel'] as string;
        const confidence = obj['confidence'] as number;
        const countryName = COUNTRY_NAMES[code] ?? code;
        setSelection({
          name: countryName,
          layerType: 'creditCycle',
          country: code,
          keywords: [code, countryName, phaseLabel],
          detail: phaseLabel,
          confidence,
          x: info.x,
          y: info.y,
        });
        return;
      }

      // News heatmap dot click
      if ('newsCount' in obj) {
        const code = obj['countryCode'] as string;
        const count = obj['newsCount'] as number;
        const keywords = obj['topKeywords'] as string[];
        const countryName = COUNTRY_NAMES[code] ?? code;
        setSelection({
          name: countryName,
          layerType: 'newsHeatmap',
          country: code,
          keywords: [code, countryName, ...keywords],
          detail: `${count} news in 1h`,
          x: info.x,
          y: info.y,
        });
        return;
      }

      // Tension arc click
      if ('tension' in obj) {
        const name = obj['name'] as string;
        const tensionLabel = obj['tensionLabel'] as string;
        const headlines = obj['recentHeadlines'] as string[];
        setSelection({
          name,
          layerType: 'tensionArcs',
          country: '',
          keywords: headlines ?? [],
          detail: tensionLabel,
          x: info.x,
          y: info.y,
        });
        return;
      }
    } else {
      // Click on empty map → close detail card
      setSelection(null);
    }
  }, []);

  if (!webglOk) {
    return (
      <div className="map-center" style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <div style={{ textAlign: 'center', color: 'var(--color-text-tertiary)', fontFamily: 'var(--font-mono)', fontSize: 12 }}>
          <p style={{ marginBottom: 8 }}>WebGL unavailable</p>
          <p style={{ fontSize: 10, opacity: 0.6 }}>Map requires GPU acceleration</p>
        </div>
      </div>
    );
  }

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

      {/* Global phase badge — top-left, only when credit cycle layer is active and data loaded */}
      {activeLayers.creditCycle && creditCycleData && (
        <div className="map-center__global-badge">
          <span className="map-center__global-badge-label">{t('map.globalPhase')}</span>
          <span className="map-center__global-badge-phase">{creditCycleData.globalPhaseLabel}</span>
          <span className="map-center__global-badge-confidence">
            {Math.round(creditCycleData.confidence * 100)}%
          </span>
        </div>
      )}

      {/* Layer toggle controls */}
      <div className="map-center__controls">
        <span className="map-center__controls-title">{t('map.layers')}</span>
        {([
          { id: 'creditCycle' as LayerId, label: t('map.layerCreditCycle'), color: '#ff6b35' },
          { id: 'newsHeatmap' as LayerId, label: t('map.layerNewsHeatmap'), color: '#ffcc00' },
          { id: 'tensionArcs' as LayerId, label: t('map.layerTensionArcs'), color: '#ff453a' },
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
