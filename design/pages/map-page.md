# Map Page Design Specification

> 智库核心页面 — Civilization IV-style Political World Map
> Route: `map` (PageId)
> Library: react-simple-maps (Natural Earth projection)

---

## 1. Page Structure

```
+------------------------------------------+
|                                          |
|          Interactive World Map            |
|          (flex: 1, fills space)          |
|                                          |
|                                          |
+------------------------------------------+
|         Bottom Summary Bar (48px)         |
+------------------------------------------+
```

The map page fills the entire MainContent area. No additional chrome — the map IS the page.

```css
.map-page {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding: 0; /* override MainContent padding for full-bleed map */
  margin: calc(-1 * var(--spacing-4)); /* negate parent padding */
}

.map-container {
  flex: 1;
  position: relative;
  overflow: hidden;
}

.map-summary-bar {
  height: 48px; /* map.summaryBarHeight */
  flex-shrink: 0;
}
```

---

## 2. Map Projection & Base

### Projection

**Natural Earth** projection — balanced distortion, familiar shape, good for political maps.

```jsx
<ComposableMap projection="geoNaturalEarth1">
  <ZoomableGroup>
    <Geographies geography={worldTopojson}>
      {/* country paths */}
    </Geographies>
  </ZoomableGroup>
</ComposableMap>
```

### Ocean (Background)

The ocean is rendered as the map container background, not as a geographic feature.

```css
.map-container {
  background: var(--map-ocean); /* #080B10 */
}
```

The ocean is darker than the app base (`#0D0D12`), creating a subtle "sunken" effect that makes land masses stand out.

### SVG Defs (filters and gradients)

Place these in the SVG `<defs>` block:

```html
<defs>
  <!-- Fog filter for no-data countries -->
  <filter id="fog-filter">
    <feGaussianBlur in="SourceGraphic" stdDeviation="0.5" />
  </filter>

  <!-- Glow filter for P0 event countries -->
  <filter id="pulse-glow" x="-50%" y="-50%" width="200%" height="200%">
    <feGaussianBlur in="SourceAlpha" stdDeviation="4" result="blur" />
    <feFlood flood-color="var(--color-intel-p0)" flood-opacity="0.6" result="color" />
    <feComposite in="color" in2="blur" operator="in" result="glow" />
    <feMerge>
      <feMergeNode in="glow" />
      <feMergeNode in="SourceGraphic" />
    </feMerge>
  </filter>

  <!-- Ocean texture gradient (optional enhancement) -->
  <radialGradient id="ocean-vignette" cx="50%" cy="50%" r="60%">
    <stop offset="0%" stop-color="rgba(0, 194, 168, 0.02)" />
    <stop offset="100%" stop-color="transparent" />
  </radialGradient>
</defs>
```

---

## 3. Country Color System

### Design Intent

Each country has a unique fill color inspired by Civilization IV's political map aesthetic: distinct territories, clear borders, and a sense of geopolitical identity.

### Color Assignment Algorithm

Countries are assigned colors from a palette of 24 base hues, distributed evenly on the HSL color wheel. The specific assignment uses a hash of the country's ISO 3166-1 alpha-3 code to ensure consistency across sessions.

**Base Palette Generation:**

```typescript
// Generate 24 evenly-spaced hues
function generateCountryPalette(): string[] {
  const palette: string[] = [];
  for (let i = 0; i < 24; i++) {
    const hue = (i * 15) % 360; // 15-degree steps
    palette.push(`hsl(${hue}, 35%, 28%)`); // muted, dark default
  }
  return palette;
}

// Assign country to palette index
function getCountryColorIndex(isoCode: string): number {
  let hash = 0;
  for (const char of isoCode) {
    hash = ((hash << 5) - hash) + char.charCodeAt(0);
    hash |= 0;
  }
  return Math.abs(hash) % 24;
}
```

### Brightness/Saturation Modulation by Data Activity

| Activity Level | Saturation | Lightness | Description |
|---------------|------------|-----------|-------------|
| **No data** (fog) | 5% | 12% | Near-grayscale, very dark |
| **Low activity** | 25% | 22% | Muted, visible but subdued |
| **Medium activity** | 40% | 30% | Clear color identity |
| **High activity** | 55% | 38% | Vibrant, prominent |
| **MVP highlight (USA)** | — | — | Uses `accent.primary` (#00C2A8) override |

```typescript
function getCountryFill(
  isoCode: string,
  activityLevel: 'none' | 'low' | 'medium' | 'high',
  isMvpCountry: boolean
): string {
  if (isMvpCountry) return theme.colors.accent.primary; // #00C2A8

  const baseHue = getCountryColorIndex(isoCode) * 15;

  const config = {
    none: { saturation: 5, lightness: 12 },
    low:  { saturation: 25, lightness: 22 },
    medium: { saturation: 40, lightness: 30 },
    high: { saturation: 55, lightness: 38 },
  };

  const { saturation, lightness } = config[activityLevel];
  return `hsl(${baseHue}, ${saturation}%, ${lightness}%)`;
}
```

### USA (MVP Country) — Special Treatment

| Property | Value |
|----------|-------|
| Fill | `accent.primary` (#00C2A8) |
| Border | `accent.hover` (#33D4BE), 3px |
| Hover | `accent.hover` (#33D4BE) fill |
| Active data | Subtle inner glow via box-shadow or SVG filter |

---

## 4. Country Borders

Political borders are a defining feature of the Civilization IV aesthetic. They must be clearly visible.

### Border Styles

```css
/* Default border — all countries */
.country-path {
  stroke-width: 2px;
  stroke-linejoin: round;
  stroke-linecap: round;
  transition: fill var(--duration-fast) var(--easing-default),
              stroke var(--duration-fast) var(--easing-default),
              opacity var(--duration-fast) var(--easing-default);
}
```

### Border Color Rule

The border color is always **one lightness step brighter** than the fill color:

```typescript
function getCountryStroke(fillColor: string): string {
  // For HSL fills: increase lightness by 12%
  // Example: hsl(120, 35%, 28%) -> hsl(120, 35%, 40%)
  // For MVP country (accent): use accent.hover
  return lighten(fillColor, 12);
}
```

| Country State | Fill Example | Stroke Example |
|--------------|-------------|----------------|
| No data (fog) | `hsl(H, 5%, 12%)` | `hsl(H, 5%, 24%)` |
| Active data | `hsl(H, 40%, 30%)` | `hsl(H, 40%, 42%)` |
| USA (MVP) | `#00C2A8` | `#33D4BE` (`accent.hover`) |

### Disputed/Special Territories

- Disputed areas: dashed border (`stroke-dasharray: 4 2`)
- Antarctica: `fill: none`, `stroke: rgba(255,255,255,0.06)`, non-interactive

---

## 5. Fog of War (No-Data Countries)

Countries with no data are obscured by a "fog of war" effect, inspired by strategy games.

### Visual Treatment

```css
/* Fog state — no data for this country */
.country-path--fog {
  fill: hsl(var(--country-hue), 5%, 12%);
  stroke: hsl(var(--country-hue), 5%, 20%);
  opacity: 0.25; /* map.fogOpacity */
  filter: url(#fog-filter);
  cursor: default;
}

/* Fog hover — slight reveal on mouseover */
.country-path--fog:hover {
  opacity: 0.45; /* map.fogHoverOpacity */
  filter: none;
  cursor: pointer;
}
```

### Fog Hover Tooltip

When hovering over a fog country, show a minimal tooltip:

```
+--------------------+
| [Flag] Country Name |
| No data available   |
+--------------------+
```

- Background: `glass.bg.elevated`
- Border: `glass.border.default`
- Text: `text.tertiary`

---

## 6. Event Pulse Animation

Countries with active events display animated effects based on event priority.

### P0 Critical Event — Glow + Scale Pulse

```css
@keyframes pulse-p0 {
  0% {
    filter: url(#pulse-glow);
    transform-origin: center;
    transform: scale(1);
  }
  50% {
    filter: url(#pulse-glow) brightness(1.3);
    transform: scale(1.015);
  }
  100% {
    filter: url(#pulse-glow);
    transform: scale(1);
  }
}

.country-path--event-p0 {
  animation: pulse-p0 2s ease-in-out infinite; /* map.pulseDuration */
  /* Override fill with slightly brighter version */
  filter: url(#pulse-glow);
}
```

**CSS-only fallback** (if SVG filter causes performance issues):

```css
@keyframes pulse-p0-fallback {
  0%, 100% {
    box-shadow: 0 0 8px rgba(255, 69, 58, 0.3);
    filter: brightness(1);
  }
  50% {
    box-shadow: 0 0 16px rgba(255, 69, 58, 0.5);
    filter: brightness(1.2);
  }
}

.country-path--event-p0 {
  animation: pulse-p0-fallback 2s ease-in-out infinite;
}
```

### P1 Important Event — Border Glow

```css
@keyframes glow-p1 {
  0%, 100% {
    stroke: var(--country-stroke-color);
    stroke-width: 2px;
  }
  50% {
    stroke: var(--color-intel-p1); /* #FF9F0A */
    stroke-width: 3px;
  }
}

.country-path--event-p1 {
  animation: glow-p1 3s ease-in-out infinite;
}
```

### P2 Routine Event — No Animation

P2 events do not trigger visual animations on the map. They are reflected only in the bottom summary bar and the hover info card's event count.

### Animation Priority Rules

| Condition | Behavior |
|-----------|----------|
| P0 + hover | Hover takes visual priority (info card appears), pulse pauses |
| P0 + P1 on same country | P0 animation wins |
| > 5 countries pulsing P0 simultaneously | Oldest P0 animations degrade to P1 (border glow only) |

### Performance Budget

- Max 5 simultaneous CSS animations with SVG filter
- Animations use `transform` and `opacity` only (GPU-composited)
- If frame drops detected, @coder-fe should disable SVG filters and use CSS-only fallback

---

## 7. Hover Info Card

When the user hovers over a country, a floating info card appears near the cursor — styled like a game-world information panel.

### Visual Design

```
+----------------------------------+
|  [Flag] UNITED STATES        [P0]|
|  --------------------------------|
|  Active Events: 12               |
|  Last Update: 2h ago             |
|  --------------------------------|
|  Geopolitical ●  5               |
|  Macro Policy ●  3               |
|  Market       ●  2               |
|  Corporate    ●  2               |
+----------------------------------+
```

### Styling

```css
.hover-info-card {
  position: absolute;
  width: 280px; /* map.hoverCardWidth */
  background: var(--glass-bg-elevated);       /* rgba(36, 36, 48, 0.82) */
  backdrop-filter: blur(var(--blur-modal));     /* 40px */
  -webkit-backdrop-filter: blur(var(--blur-modal));
  border: 1px solid var(--glass-border-strong); /* rgba(255,255,255,0.12) */
  border-radius: var(--radius-lg);              /* 14px */
  box-shadow: var(--shadow-lg);
  padding: var(--spacing-4);                    /* 16px */
  pointer-events: none;
  z-index: var(--z-tooltip);                    /* 50 */

  /* Entrance animation */
  opacity: 0;
  transform: translateY(8px);
  transition: opacity var(--duration-fast) var(--easing-default),
              transform var(--duration-fast) var(--easing-default);
}

.hover-info-card--visible {
  opacity: 1;
  transform: translateY(0);
}
```

### Content Structure

| Element | Font | Color | Token |
|---------|------|-------|-------|
| Country name | `fontSize.md` (14px), `fontWeight.semibold` | `text.primary` | — |
| Flag emoji | 20px | — | Emoji from ISO code |
| Priority badge | `fontSize.xs` (11px), `fontWeight.medium` | White on `intel.p0/p1/p2` bg | `radius.sm` |
| Divider | 1px | `border.primary` | — |
| Stats labels | `fontSize.sm` (12px), `fontWeight.regular` | `text.secondary` | — |
| Stats values | `fontSize.sm` (12px), `fontWeight.medium`, `fontFamily.mono` | `text.primary` | — |
| Category dots | 8px circle | `intel.geopolitical / macroPolicy / market / corporate` | `radius.full` |
| Category counts | `fontSize.sm`, `fontFamily.mono` | `text.primary` | — |

### Positioning Logic

- Default: 16px right and 16px below cursor
- If card would overflow right edge: flip to left of cursor
- If card would overflow bottom: flip to above cursor
- Minimum 8px from any map edge

### Hover Timing

| Action | Delay |
|--------|-------|
| Show card | 200ms hover dwell time (prevent flicker on fast mouse movement) |
| Hide card | Immediate on mouse leave |
| Transition | `duration.fast` (150ms) fade + translate |

### Three-State Design

| State | Display |
|-------|---------|
| **No data country** | Flag + Name + "No data available" in `text.tertiary` |
| **Has data** | Full card with all stats |
| **Data loading** | Flag + Name + skeleton pulse lines (3 rows) |

### Small Country Handling

Countries with very small SVG area (< 100 sq px on screen):
- Increase hover hit area by adding an invisible expanded `<circle>` or padding the path
- Tooltip offset increased to 24px to avoid occluding the country

---

## 8. Bottom Summary Bar

A persistent bar at the bottom of the map showing a global intelligence summary.

### Visual Design

```
+-------------------------------------------------------------------+
| Geopolitical ● 12 | Macro ● 8 | Market ● 23 | Corporate ● 5  | Total: 48 |
+-------------------------------------------------------------------+
```

### Styling

```css
.map-summary-bar {
  height: 48px; /* map.summaryBarHeight */
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--spacing-6);                        /* 24px */
  padding: 0 var(--spacing-4);                  /* 16px horizontal */
  background: var(--glass-bg-structural);       /* rgba(20, 20, 28, 0.72) */
  backdrop-filter: blur(var(--blur-status-bar)); /* 16px */
  -webkit-backdrop-filter: blur(var(--blur-status-bar));
  border-top: 1px solid var(--glass-border-subtle);
  font-size: var(--font-size-sm);               /* 12px */
}
```

### Summary Items

Each category item:

```css
.summary-item {
  display: flex;
  align-items: center;
  gap: var(--spacing-2); /* 8px */
}

.summary-dot {
  width: 8px;
  height: 8px;
  border-radius: var(--radius-full);
}

.summary-dot--geopolitical { background: var(--color-intel-geopolitical); } /* #D4553A */
.summary-dot--macro        { background: var(--color-intel-macro-policy); } /* #7B68EE */
.summary-dot--market       { background: var(--color-intel-market); }       /* #00C2A8 */
.summary-dot--corporate    { background: var(--color-intel-corporate); }    /* #D4A03A */

.summary-label {
  color: var(--color-text-secondary);
  font-weight: var(--font-weight-regular);
}

.summary-count {
  color: var(--color-text-primary);
  font-family: var(--font-mono);
  font-weight: var(--font-weight-medium);
}

.summary-total {
  color: var(--color-text-primary);
  font-weight: var(--font-weight-semibold);
  margin-left: var(--spacing-4);
  padding-left: var(--spacing-4);
  border-left: 1px solid var(--color-border-primary);
}
```

### Three-State Design

| State | Display |
|-------|---------|
| **Empty** (no data) | "No intelligence data — configure data sources in Settings" + link button |
| **Loading** | 4 skeleton pulse bars + total skeleton |
| **Error** | "Summary unavailable" in `text.tertiary` + retry icon button |

### i18n Consideration

| zh-CN | en-US | Width impact |
|-------|-------|-------------|
| "地缘" | "Geopolitical" | en-US 2.5x longer |
| "宏观" | "Macro" | Similar |
| "市场" | "Market" | en-US 1.5x longer |
| "企业" | "Corporate" | en-US 2x longer |

Solution: Use `flex` layout with `gap`, no fixed widths. Labels overflow with ellipsis at extreme widths. On narrow windows, abbreviate: "Geo" / "Macro" / "Mkt" / "Corp".

---

## 9. Map Empty State

When no data sources are configured or no data has been fetched yet.

### Visual Design

```
+------------------------------------------+
|                                          |
|     [Globe illustration / SVG icon]      |
|                                          |
|     No Intelligence Data Available        |
|                                          |
|     Configure data sources to begin       |
|     monitoring global events              |
|                                          |
|     [Go to Settings]                      |
|                                          |
+------------------------------------------+
|  (empty summary bar with guide text)      |
+------------------------------------------+
```

### Styling

```css
.map-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  text-align: center;
  gap: var(--spacing-4); /* 16px */
}

.map-empty-icon {
  width: 64px;
  height: 64px;
  color: var(--color-text-tertiary);
  opacity: 0.6;
}

.map-empty-title {
  font-size: var(--font-size-lg); /* 16px */
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.map-empty-description {
  font-size: var(--font-size-base); /* 13px */
  color: var(--color-text-secondary);
  max-width: 320px;
}

.map-empty-action {
  margin-top: var(--spacing-2); /* 8px */
  padding: var(--spacing-2) var(--spacing-4); /* 8px 16px */
  background: var(--color-accent-primary);
  color: #FFFFFF;
  border: none;
  border-radius: var(--radius-md);
  font-size: var(--font-size-base);
  font-weight: var(--font-weight-medium);
  cursor: pointer;
  transition: background var(--duration-fast) var(--easing-default);
}

.map-empty-action:hover {
  background: var(--color-accent-hover);
}
```

NOTE: Even in empty state, the world map SVG is rendered behind the empty overlay — all countries in full fog state. This creates a "dark globe" background that reinforces the intel aesthetic.

---

## 10. Map Loading State

When the map topology JSON is loading or data is being fetched.

### Visual Design

The world map outline renders immediately (topology is bundled). Country data overlay shows skeleton states:

```css
.map-loading-overlay {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--spacing-3); /* 12px */
}

.map-loading-spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--color-border-primary);
  border-top-color: var(--color-accent-primary);
  border-radius: var(--radius-full);
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.map-loading-text {
  font-size: var(--font-size-sm);
  color: var(--color-text-secondary);
}
```

---

## 11. Map Error State

When the topology file fails to load.

```css
.map-error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: var(--spacing-4);
}

.map-error-icon {
  width: 48px;
  height: 48px;
  color: var(--color-semantic-error);
}

.map-error-title {
  font-size: var(--font-size-lg);
  font-weight: var(--font-weight-semibold);
  color: var(--color-text-primary);
}

.map-error-description {
  font-size: var(--font-size-base);
  color: var(--color-text-secondary);
}

.map-error-retry {
  padding: var(--spacing-2) var(--spacing-4);
  background: transparent;
  color: var(--color-accent-primary);
  border: 1px solid var(--color-accent-primary);
  border-radius: var(--radius-md);
  font-size: var(--font-size-base);
  cursor: pointer;
}

.map-error-retry:hover {
  background: var(--color-accent-subtle);
}
```

---

## 12. Country Click Behavior

| Country State | Click Action |
|--------------|-------------|
| Has data | Navigate to Finance page (`finance`) filtered by this country |
| Fog (no data) | No-op (cursor: default). Tooltip explains "No data available" |
| MVP country (USA) | Navigate to Finance page filtered to USA |

### Click Feedback

```css
.country-path--active:active {
  opacity: 0.8;
  transition: opacity 50ms;
}
```

---

## 13. Zoom and Pan

react-simple-maps `<ZoomableGroup>` provides built-in zoom/pan.

### Configuration

| Property | Value |
|----------|-------|
| Min zoom | 1 (show full world) |
| Max zoom | 8 |
| Default zoom | 1 |
| Default center | [0, 20] (slightly north to show more land) |
| Zoom controls | Mouse wheel + pinch. No visible zoom buttons (desktop app, not touch-first) |

### Zoom Behavior

- Zoom preserves cursor position as center
- Pan with click-drag
- Double-click zooms in 2x
- Borders scale inversely with zoom (borders stay visually consistent thickness)

```typescript
// Border width scaling
const strokeWidth = 2 / currentZoom; // 2px at zoom=1, 1px at zoom=2, etc.
```

---

## 14. Decision Points for User (X.1a Proposal)

The following design decisions require user confirmation before finalizing:

### Decision A: Accent Color

| Option | Hex | Rationale |
|--------|-----|-----------|
| **A1 (recommended)** | `#00C2A8` (Teal) | Intel/analysis feel, distinct from QT blue, good dark bg contrast |
| A2 | `#00BFA5` (Darker Teal) | More subdued, may lack punch on dark backgrounds |
| A3 | `#26C6DA` (Cyan) | Brighter, more "tech" feel, but may clash with `semantic.info` (#64D2FF) |

### Decision B: Intel Category Colors

| Category | Option B1 (recommended) | Option B2 (warm/cool split) |
|----------|------------------------|---------------------------|
| Geopolitical | `#D4553A` (brick red) | `#E8634A` (coral) |
| Macro Policy | `#7B68EE` (blue-violet) | `#5C7AEA` (royal blue) |
| Market | `#00C2A8` (accent teal) | `#00C2A8` (accent teal) |
| Corporate | `#D4A03A` (amber/gold) | `#F0A030` (bright amber) |

B1 is muted/professional. B2 is brighter/more vibrant.

### Decision C: Map Country Color Strategy

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **C1 (recommended)** | 24-hue HSL palette, per-country hash | Unique identity per country, Civ IV feel | Complex, adjacent countries may look similar |
| C2 | 6-continent palette | Simple, clear regions | Countries within same continent indistinguishable |
| C3 | Monochrome (gray scale only) | Clean, modern | Loses Civ IV personality, less engaging |

---

## 15. Token Reference Summary

All tokens used in this spec, mapped to `theme.ts`:

| Usage | Token Path | Value |
|-------|-----------|-------|
| Ocean bg | `map.ocean` | `#080B10` |
| Country border width | `map.borderWidth` | `2px` |
| Fog opacity | `map.fogOpacity` | `0.25` |
| Fog hover opacity | `map.fogHoverOpacity` | `0.45` |
| Pulse duration | `map.pulseDuration` | `2s` |
| Pulse glow radius | `map.pulseGlowRadius` | `12px` |
| Hover card width | `map.hoverCardWidth` | `280px` |
| Summary bar height | `map.summaryBarHeight` | `48px` |
| MVP country fill | `colors.accent.primary` | `#00C2A8` |
| MVP country border | `colors.accent.hover` | `#33D4BE` |
| P0 color | `colors.intel.p0` | `#FF453A` |
| P1 color | `colors.intel.p1` | `#FF9F0A` |
| P2 color | `colors.intel.p2` | `#00C2A8` |
| Geopolitical | `colors.intel.geopolitical` | `#D4553A` |
| Macro Policy | `colors.intel.macroPolicy` | `#7B68EE` |
| Market | `colors.intel.market` | `#00C2A8` |
| Corporate | `colors.intel.corporate` | `#D4A03A` |
| Info card bg | `glass.bg.elevated` | `rgba(36, 36, 48, 0.82)` |
| Info card border | `glass.border.strong` | `rgba(255,255,255,0.12)` |
| Summary bar bg | `glass.bg.structural` | `rgba(20, 20, 28, 0.72)` |
| Transition fast | `animation.duration.fast` | `150ms` |
| Transition slow | `animation.duration.slow` | `350ms` |
| Easing default | `animation.easing.default` | `cubic-bezier(0.25, 0.1, 0.25, 1.0)` |
