import type { CSSProperties } from 'react';

export interface SparklineProps {
  /** Array of numeric values to plot. */
  data: number[];
  width?: number;
  height?: number;
  /** Line color. Defaults to CSS var resolved from trend when not supplied. */
  color?: string;
  /** Fill opacity for the area gradient under the line. Default 0.15. */
  fillOpacity?: number;
  /** Show a dot on the last data point. Default true. */
  showDot?: boolean;
  /**
   * Trend direction. If omitted, auto-calculated by comparing the mean of
   * the last third of values against the mean of the first third.
   */
  trend?: 'up' | 'down' | 'flat';
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function autoTrend(data: number[]): 'up' | 'down' | 'flat' {
  if (data.length < 3) return 'flat';
  const third = Math.max(1, Math.floor(data.length / 3));
  const first = data.slice(0, third);
  const last = data.slice(data.length - third);
  const avg = (arr: number[]) => arr.reduce((s, v) => s + v, 0) / arr.length;
  const diff = avg(last) - avg(first);
  const range = Math.max(...data) - Math.min(...data);
  // Threshold: 5 % of total range counts as movement
  if (range === 0) return 'flat';
  if (diff > range * 0.05) return 'up';
  if (diff < -range * 0.05) return 'down';
  return 'flat';
}

function trendColor(trend: 'up' | 'down' | 'flat'): string {
  if (trend === 'up') return 'var(--color-semantic-success)';
  if (trend === 'down') return 'var(--color-semantic-error)';
  return 'var(--color-accent-primary)';
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function Sparkline({
  data,
  width = 120,
  height = 32,
  color,
  fillOpacity = 0.15,
  showDot = true,
  trend: trendProp,
}: SparklineProps) {
  // ---- Degenerate cases ----
  if (data.length === 0) {
    return (
      <svg
        width={width}
        height={height}
        aria-hidden="true"
        style={{ display: 'block', flexShrink: 0 } as CSSProperties}
      >
        <line
          x1={0}
          y1={height / 2}
          x2={width}
          y2={height / 2}
          stroke="var(--color-text-disabled)"
          strokeWidth={1}
          strokeDasharray="3 3"
        />
      </svg>
    );
  }

  if (data.length === 1) {
    return (
      <svg
        width={width}
        height={height}
        aria-hidden="true"
        style={{ display: 'block', flexShrink: 0 } as CSSProperties}
      >
        <line
          x1={0}
          y1={height / 2}
          x2={width}
          y2={height / 2}
          stroke="var(--color-text-disabled)"
          strokeWidth={1}
        />
      </svg>
    );
  }

  // ---- Normalize data ----
  const min = Math.min(...data);
  const max = Math.max(...data);
  const range = max - min;

  const padding = 3; // px padding top/bottom so the line doesn't clip
  const usableHeight = height - padding * 2;
  const usableWidth = width;

  // Map value → SVG Y coordinate (SVG Y grows downward)
  const toY = (v: number): number => {
    if (range === 0) return height / 2;
    return padding + usableHeight - ((v - min) / range) * usableHeight;
  };

  // X positions: evenly distributed
  const toX = (i: number): number =>
    data.length === 1 ? usableWidth / 2 : (i / (data.length - 1)) * usableWidth;

  const points = data.map((v, i) => `${toX(i).toFixed(1)},${toY(v).toFixed(1)}`).join(' ');

  // Area polygon: close the shape at the bottom
  const firstX = toX(0);
  const lastX = toX(data.length - 1);
  const bottom = height;
  const areaPoints =
    `${firstX.toFixed(1)},${bottom} ` + points + ` ${lastX.toFixed(1)},${bottom}`;

  // ---- Trend + color ----
  const resolvedTrend = trendProp ?? autoTrend(data);
  const strokeColor = color ?? trendColor(resolvedTrend);

  // Last dot
  const lastIndex = data.length - 1;
  const dotX = toX(lastIndex);
  const dotY = toY(data[lastIndex]);

  const gradientId = `sparkline-grad-${Math.random().toString(36).slice(2, 8)}`;

  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      aria-hidden="true"
      style={{ display: 'block', flexShrink: 0, overflow: 'visible' } as CSSProperties}
    >
      <defs>
        <linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={strokeColor} stopOpacity={fillOpacity * 2.5} />
          <stop offset="100%" stopColor={strokeColor} stopOpacity={0} />
        </linearGradient>
      </defs>

      {/* Filled area */}
      <polygon
        points={areaPoints}
        fill={`url(#${gradientId})`}
      />

      {/* Line */}
      <polyline
        points={points}
        fill="none"
        stroke={strokeColor}
        strokeWidth={1.5}
        strokeLinejoin="round"
        strokeLinecap="round"
      />

      {/* Last-point dot */}
      {showDot && (
        <circle
          cx={dotX}
          cy={dotY}
          r={2.5}
          fill={strokeColor}
        />
      )}
    </svg>
  );
}
