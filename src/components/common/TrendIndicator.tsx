import { useState, useEffect } from 'react';
import { getIndicatorTrend } from '@services/tauri-bridge';
import type { TrendPoint } from '@services/tauri-bridge';
import { Sparkline } from './Sparkline';
import type { SparklineProps } from './Sparkline';

export interface TrendIndicatorProps {
  /** Backend indicator name, e.g. "cpi_yoy", "vix". */
  indicator: string;
  /** Number of days of history to fetch. Default 30. */
  days?: number;
  width?: SparklineProps['width'];
  height?: SparklineProps['height'];
}

// Pulse animation inline style — avoids needing a CSS file
const pulseKeyframes = `
@keyframes trend-pulse {
  0%, 100% { opacity: 0.3; }
  50% { opacity: 0.7; }
}
`;

let pulseCssInjected = false;
function ensurePulseCss() {
  if (pulseCssInjected) return;
  const style = document.createElement('style');
  style.textContent = pulseKeyframes;
  document.head.appendChild(style);
  pulseCssInjected = true;
}

export function TrendIndicator({
  indicator,
  days = 30,
  width = 120,
  height = 32,
}: TrendIndicatorProps) {
  const [points, setPoints] = useState<TrendPoint[] | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    ensurePulseCss();
    let cancelled = false;
    setLoading(true);
    void getIndicatorTrend(indicator, days).then((data) => {
      if (!cancelled) {
        setPoints(data);
        setLoading(false);
      }
    }).catch(() => {
      if (!cancelled) {
        setPoints([]);
        setLoading(false);
      }
    });
    return () => { cancelled = true; };
  }, [indicator, days]);

  if (loading) {
    return (
      <div
        style={{
          width,
          height,
          borderRadius: 4,
          background: 'var(--color-bg-hover)',
          animation: 'trend-pulse 1.4s ease-in-out infinite',
          flexShrink: 0,
        }}
        aria-hidden="true"
      />
    );
  }

  // Silent failure — return empty placeholder of same size
  if (!points || points.length === 0) {
    return (
      <div
        style={{ width, height, flexShrink: 0 }}
        aria-hidden="true"
      />
    );
  }

  const values = points.map((p) => p.value);
  return <Sparkline data={values} width={width} height={height} />;
}
