# Glassmorphism Visual Specification

> 智库 Dark Glassmorphism
> Based on QuantTerminal glass system, accent adjusted to Teal (#00C2A8)
> Layout structure, typography, and spacing remain unchanged.

---

## 1. Design Philosophy

The 智库 design uses a solid dark color palette with backdrop-filter blur on structural elements (TitleBar, Sidebar, StatusBar). This enhancement deepens that foundation into a full glassmorphism aesthetic:

- **Translucent surfaces**: Cards and panels gain semi-transparent backgrounds that reveal a subtle mesh gradient beneath them, creating depth without sacrificing readability.
- **Luminous borders**: Surfaces are outlined with faint white/colored borders that simulate light refraction on glass edges.
- **Ambient glow**: Interactive elements and status indicators emit soft colored glows via box-shadow, reinforcing semantic meaning (teal accent for primary actions, green for success, red for errors, purple for AI).
- **Mesh gradient backdrop**: The app's base layer includes 2-3 large radial gradients at very low opacity, providing organic color variation that glass surfaces partially reveal.

**What does NOT change**: Layout geometry, font system, spacing system, z-index scale, animation timings.

---

## 2. Mesh Gradient Backdrop

The mesh gradient sits on the `.app` element as a `background` layer beneath all content. It provides the "scene behind the glass" that makes translucent surfaces meaningful.

### Specification

```css
.app {
  background:
    radial-gradient(
      ellipse 600px 400px at 15% 20%,
      var(--glass-mesh-accent) 0%,       /* rgba(0, 194, 168, 0.04) */
      transparent 70%
    ),
    radial-gradient(
      ellipse 500px 500px at 80% 75%,
      var(--glass-mesh-purple) 0%,       /* rgba(191, 90, 242, 0.035) */
      transparent 70%
    ),
    radial-gradient(
      ellipse 400px 300px at 50% 50%,
      var(--glass-mesh-warm) 0%,         /* rgba(255, 140, 50, 0.02) */
      transparent 70%
    ),
    var(--color-bg-base);                /* #0D0D12 */
}
```

### Token Values

| Token | Value | Purpose |
|-------|-------|---------|
| `--glass-mesh-accent` | `rgba(0, 194, 168, 0.04)` | Teal accent glow, top-left region |
| `--glass-mesh-purple` | `rgba(191, 90, 242, 0.035)` | AI purple glow, bottom-right region |
| `--glass-mesh-warm` | `rgba(255, 140, 50, 0.02)` | Warm accent, center (very subtle) |

These gradients are intentionally very faint. They should be barely perceptible on their own, but become visible through translucent glass panels as a gentle color shift.

### Behavior

- The mesh gradient is **static** (no animation) to avoid performance overhead.
- It is applied to the `.app` root container, so it sits behind everything.
- The gradient positions are fixed (percentage-based), not dependent on content.

---

## 3. Glass Surface Levels

Three tiers of glass translucency, used by different surface types. All combine a semi-transparent background with backdrop-filter blur and a luminous border.

### Tier 1: Structural Glass (TitleBar, Sidebar, StatusBar)

```css
/* TitleBar */
.title-bar {
  background: var(--glass-bg-structural);      /* rgba(20, 20, 28, 0.72) */
  backdrop-filter: blur(var(--blur-title-bar)); /* 24px */
  border-bottom: 1px solid var(--glass-border-subtle); /* rgba(255,255,255,0.06) */
}

/* Sidebar */
.sidebar {
  background: var(--glass-bg-structural);
  backdrop-filter: blur(var(--blur-sidebar));    /* 20px */
  border-right: 1px solid var(--glass-border-subtle);
}

/* StatusBar */
.status-bar {
  background: var(--glass-bg-structural);
  backdrop-filter: blur(var(--blur-status-bar)); /* 16px */
  border-top: 1px solid var(--glass-border-subtle);
}
```

| Property | Value |
|----------|-------|
| Background | `rgba(20, 20, 28, 0.72)` |
| Blur | Unchanged (reuse existing blur tokens) |
| Border | `rgba(255, 255, 255, 0.06)` |

### Tier 2: Card Glass (content cards, panels, data containers)

The primary glassmorphism surface. Used for cards inside the main content area.

```css
.glass-card {
  background: var(--glass-bg-card);            /* rgba(28, 28, 40, 0.45) */
  backdrop-filter: blur(var(--blur-sidebar));   /* 20px */
  -webkit-backdrop-filter: blur(var(--blur-sidebar));
  border: 1px solid var(--glass-border-default); /* rgba(255,255,255,0.08) */
  border-radius: var(--radius-lg);              /* 14px */
  box-shadow: var(--shadow-md);
}
```

| Property | Value |
|----------|-------|
| Background | `rgba(28, 28, 40, 0.45)` |
| Blur | 20px (reuse `--blur-sidebar`) |
| Border | `rgba(255, 255, 255, 0.08)` |
| Radius | 14px (`--radius-lg`) |

### Tier 3: Elevated Glass (modals, dropdowns, tooltips)

Higher opacity for better readability on floating surfaces.

```css
.glass-elevated {
  background: var(--glass-bg-elevated);        /* rgba(36, 36, 48, 0.82) */
  backdrop-filter: blur(var(--blur-modal));      /* 40px */
  -webkit-backdrop-filter: blur(var(--blur-modal));
  border: 1px solid var(--glass-border-strong);  /* rgba(255,255,255,0.12) */
  border-radius: var(--radius-xl);               /* 20px */
  box-shadow: var(--shadow-lg);
}
```

| Property | Value |
|----------|-------|
| Background | `rgba(36, 36, 48, 0.82)` |
| Blur | 40px (reuse `--blur-modal`) |
| Border | `rgba(255, 255, 255, 0.12)` |
| Radius | 20px (`--radius-xl`) |

---

## 4. Glass Border System

Luminous borders simulate light catching the edge of a glass surface. They use white at varying opacities.

| Token | Value | Usage |
|-------|-------|-------|
| `--glass-border-subtle` | `rgba(255, 255, 255, 0.06)` | Structural elements (titlebar, sidebar, statusbar) |
| `--glass-border-default` | `rgba(255, 255, 255, 0.08)` | Cards, panels |
| `--glass-border-strong` | `rgba(255, 255, 255, 0.12)` | Elevated surfaces (modals, dropdowns) |

These replace `--color-border-primary` and `--color-border-secondary` on glass surfaces only. The existing border tokens remain available for non-glass elements (inputs, dividers, etc.).

---

## 5. Glow Effects

Glow effects use `box-shadow` (not border) for performance. They are applied to interactive elements on hover/focus, and to status indicators.

### Hover Glow

Applied to cards and interactive containers on hover.

```css
.glass-card:hover {
  border-color: var(--glass-border-strong);    /* brighten border */
  box-shadow:
    var(--shadow-md),                           /* keep existing shadow */
    0 0 20px var(--glass-glow-accent);          /* add teal glow */
}
```

### Glow Color Tokens

| Token | Value | Purpose |
|-------|-------|---------|
| `--glass-glow-accent` | `rgba(0, 194, 168, 0.10)` | Primary interactive elements, default hover |
| `--glass-glow-success` | `rgba(48, 209, 88, 0.10)` | Success states, connected indicators |
| `--glass-glow-error` | `rgba(255, 69, 58, 0.10)` | Error states, disconnected indicators |
| `--glass-glow-ai` | `rgba(191, 90, 242, 0.10)` | AI-related components |

### Usage Examples

**Button hover glow:**
```css
.button-primary:hover {
  box-shadow: 0 0 16px var(--glass-glow-accent);
}
```

**Status indicator glow (connected):**
```css
.status-dot--connected {
  box-shadow: 0 0 8px var(--glass-glow-success);
}
```

**AI panel card:**
```css
.ai-card:hover {
  box-shadow:
    var(--shadow-md),
    0 0 20px var(--glass-glow-ai);
}
```

**Error card:**
```css
.error-card {
  border-color: var(--color-border-error);
  box-shadow: 0 0 12px var(--glass-glow-error);
}
```

### Glow Sizing Guidelines

| Context | Blur radius | Spread |
|---------|------------|--------|
| Small elements (buttons, badges) | 8-12px | 0 |
| Medium elements (cards) | 16-20px | 0 |
| Large elements (modals, panels) | 24-32px | 0 |

Always use `spread: 0` to keep glow soft.

---

## 6. Component Application Guide

### Cards (all pages)

Every content card in the main content area should use Tier 2 glass.

```css
.card {
  background: var(--glass-bg-card);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  border: 1px solid var(--glass-border-default);
  border-radius: var(--radius-lg);
  transition: border-color var(--duration-fast) var(--easing-default),
              box-shadow var(--duration-fast) var(--easing-default);
}

.card:hover {
  border-color: var(--glass-border-strong);
  box-shadow: var(--shadow-md), 0 0 20px var(--glass-glow-accent);
}
```

### Sidebar Navigation Items

Navigation items remain as they are (no glass effect on individual items). The sidebar container itself is Tier 1 glass. Active item retains `--color-accent-subtle` background.

### Modals / Dialogs

Use Tier 3 glass. The modal overlay (backdrop) should also gain a subtle blur:

```css
.modal-overlay {
  background: rgba(0, 0, 0, 0.50);
  backdrop-filter: blur(4px);
}

.modal {
  background: var(--glass-bg-elevated);
  backdrop-filter: blur(var(--blur-modal));
  border: 1px solid var(--glass-border-strong);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-lg);
}
```

### Input Fields

Inputs inside glass cards should use a slightly darker, more opaque background for contrast:

```css
.input {
  background: rgba(0, 0, 0, 0.20);
  border: 1px solid var(--glass-border-default);
  border-radius: var(--radius-md);
}

.input:focus {
  border-color: var(--color-border-focus);
  box-shadow: 0 0 12px var(--glass-glow-accent);
}
```

### Tooltips

Use existing tooltip blur with glass treatment:

```css
.tooltip {
  background: var(--glass-bg-elevated);
  backdrop-filter: blur(var(--blur-tooltip));
  border: 1px solid var(--glass-border-default);
  border-radius: var(--radius-md);
}
```

---

## 7. Performance Notes

- **backdrop-filter** is GPU-accelerated on modern browsers and Tauri's WebView2. However, nested blur (a blurred element inside another blurred element) can cause performance issues.
- **Rule**: Never nest more than one level of backdrop-filter. The structural glass (sidebar, titlebar) provides the first level; cards inside the main content area are NOT inside a blurred container (main-content has no blur), so they are safe.
- **Mesh gradient** uses CSS only (no canvas/JS), so it has zero runtime cost.
- **Glow box-shadows** are cheap compared to backdrop-filter. Multiple glows on screen simultaneously are fine.

---

## 8. Things That Do NOT Change

| Category | Detail |
|----------|--------|
| Layout structure | TitleBar / Sidebar / MainContent / StatusBar geometry unchanged |
| Typography | Font families, sizes, weights, line heights unchanged |
| Spacing system | 4px grid system unchanged |
| Border radius | Radius scale unchanged |
| Z-index scale | Layer ordering unchanged |
| Animation timings | Duration and easing curves unchanged |
| Blur values | Existing blur tokens unchanged (sidebar 20px, modal 40px, etc.) |
| Semantic colors | Success, warning, error, info base colors unchanged |

---

## 9. Token Summary

| Group | Count | Tokens |
|-------|-------|--------|
| Glass backgrounds | 3 | `glass-bg-structural`, `glass-bg-card`, `glass-bg-elevated` |
| Glass borders | 3 | `glass-border-subtle`, `glass-border-default`, `glass-border-strong` |
| Glass glow | 4 | `glass-glow-accent`, `glass-glow-success`, `glass-glow-error`, `glass-glow-ai` |
| Mesh gradient | 3 | `glass-mesh-accent`, `glass-mesh-purple`, `glass-mesh-warm` |
| **Total** | **13** | |

All tokens are semantic, scoped under the `glass` namespace, and append to (not replace) the existing token system.
