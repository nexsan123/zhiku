# App Layout Specification

> 智库全局布局规范
> 四段式结构：TitleBar + Sidebar + MainContent + StatusBar

---

## 1. Overall Structure

```
+--------------------------------------------------+
|                  TitleBar (52px)                   |
+--------+-----------------------------------------+
|        |                                         |
| Side   |              MainContent                |
| bar    |              (flex: 1)                   |
| 72/    |                                         |
| 240px  |                                         |
|        |                                         |
+--------+-----------------------------------------+
|                  StatusBar (32px)                  |
+--------------------------------------------------+
```

### CSS Structure

```css
.app {
  display: flex;
  flex-direction: column;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  background: /* mesh gradient layers, see glassmorphism-spec.md */;
}

.app-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.main-content {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  padding: var(--spacing-4); /* 16px */
}
```

---

## 2. TitleBar

| Property | Value | Token |
|----------|-------|-------|
| Height | 52px | `layout.titleBar.height` |
| Background | Glass Tier 1 | `glass.bg.structural` |
| Blur | 24px | `blur.titleBar` |
| Border bottom | 1px solid | `glass.border.subtle` |
| z-index | 30 | `zIndex.titleBar` |
| Padding horizontal | 16px | `spacing['4']` |

### Content Layout

```
+--[drag region]------------------------------------+
| [Logo] [AppName]           [Notification] [W][M][X]|
+----------------------------------------------------+
```

| Element | Spec |
|---------|------|
| Logo | "智" character, `fontSize.xl` (20px), `fontWeight.semibold`, `color.accent.primary` |
| App Name | "智库", `fontSize.lg` (16px), `fontWeight.medium`, `color.text.primary` |
| Gap (logo-name) | `spacing['2']` (8px) |
| Notification bell | 20x20 icon, `color.text.secondary`. Badge: 16px circle, `color.intel.p0` bg, `fontSize.xs` white text |
| Window controls | minimize / maximize / close, 12x12 circles, aligned right |
| Drag region | Entire titlebar except buttons (`-webkit-app-region: drag`) |

### Notification Badge States

| State | Visual |
|-------|--------|
| No notifications | Bell icon only, `text.secondary` |
| P1/P2 notifications | Bell + count badge, `accent.primary` bg |
| P0 critical | Bell + count badge, `intel.p0` bg + subtle pulse glow |

### Three-State Design

| State | Display |
|-------|---------|
| **Empty** | Logo + App Name + bell (no badge) + window controls |
| **Loading** | Logo + App Name + skeleton pulse (24x24) where bell is + window controls |
| **Error** | Same as empty (titlebar is resilient, never shows error) |

### i18n Consideration

- "智库" (zh-CN) = 2 chars, "ZhiKu" (en-US) = 5 chars. AppName uses `text.primary` with no fixed width, flex layout absorbs length difference.

---

## 3. Sidebar

| Property | Value | Token |
|----------|-------|-------|
| Collapsed width | 72px | `layout.sidebar.collapsedWidth` |
| Expanded width | 240px | `layout.sidebar.expandedWidth` |
| Background | Glass Tier 1 | `glass.bg.structural` |
| Blur | 20px | `blur.sidebar` |
| Border right | 1px solid | `glass.border.subtle` |
| z-index | 20 | `zIndex.sidebar` |
| Transition | width `animation.duration.normal` `animation.easing.default` |

### Navigation Items (5 pages)

| # | Icon | Label (zh-CN) | Label (en-US) | Route |
|---|------|--------------|---------------|-------|
| 1 | Globe icon | 世界地图 | World Map | `map` |
| 2 | Chart/Layers icon | 金融板块 | Finance | `finance` |
| 3 | AI/Brain icon | AI 对话 | AI Chat | `ai` |
| 4 | Bell icon | 通知中心 | Notifications | `notifications` |
| 5 | Gear icon | 设置 | Settings | `settings` |

### Nav Item States

| State | Visual |
|-------|--------|
| **Default** | Icon `text.secondary`, label `text.secondary` |
| **Hover** | Background `bg.hover`, icon/label `text.primary` |
| **Active (current page)** | Background `accent.subtle`, icon/label `accent.primary`, left border 3px `accent.primary` |

### Nav Item Dimensions

| Property | Collapsed | Expanded |
|----------|-----------|----------|
| Item height | 48px | 44px |
| Icon size | 24x24 | 20x20 |
| Icon alignment | Centered | Left, `spacing['4']` from edge |
| Label | Hidden | Visible, `spacing['3']` after icon |
| Item padding | `spacing['3']` all | `spacing['3']` vertical, `spacing['4']` horizontal |

### Collapse/Expand Toggle

- Toggle button at sidebar bottom, 36x36, chevron icon
- Collapsed: chevron-right, Expanded: chevron-left
- Hover: `bg.hover` background

### Sidebar Collapse Behavior

- Default: expanded (>= 1400px window width), collapsed (< 1400px)
- User can toggle manually (persisted to localStorage)
- MainContent width adjusts via `flex: 1` (no jitter: sidebar uses `width` transition, not `display`)

### Three-State Design

| State | Display |
|-------|---------|
| **Empty** | All 5 nav items rendered, no special state |
| **Loading** | All 5 nav items rendered (sidebar is static content, never loading) |
| **Error** | All 5 nav items rendered (sidebar is resilient) |

### i18n Consideration

- Expanded sidebar: 240px accommodates longest label "Notifications" (en-US, ~100px at 13px font)
- Collapsed sidebar: labels hidden, icon-only, no i18n issue

---

## 4. MainContent

| Property | Value | Token |
|----------|-------|-------|
| Layout | `flex: 1` | — |
| Overflow | `auto` (vertical), `hidden` (horizontal) | — |
| Padding | 16px | `spacing['4']` |
| Background | Transparent (mesh gradient shows through) | — |

MainContent renders the active page component based on current route (PageId). Each page manages its own three-state design internally (see per-page specs).

### Scroll Behavior

- Vertical scroll with native scrollbar (styled dark)
- Scrollbar: 8px width, `bg.hover` thumb, `bg.primary` track, `radius.full` thumb

```css
.main-content::-webkit-scrollbar {
  width: 8px;
}
.main-content::-webkit-scrollbar-track {
  background: var(--color-bg-primary);
}
.main-content::-webkit-scrollbar-thumb {
  background: var(--color-bg-hover);
  border-radius: var(--radius-full);
}
.main-content::-webkit-scrollbar-thumb:hover {
  background: var(--color-bg-active);
}
```

---

## 5. StatusBar

| Property | Value | Token |
|----------|-------|-------|
| Height | 32px | `layout.statusBar.height` |
| Background | Glass Tier 1 | `glass.bg.structural` |
| Blur | 16px | `blur.statusBar` |
| Border top | 1px solid | `glass.border.subtle` |
| z-index | 25 | `zIndex.statusBar` |
| Padding horizontal | 16px | `spacing['4']` |
| Font size | 11px | `typography.fontSize.xs` |

### Content Layout

```
+----------------------------------------------------+
| [StatusDot] Fed RSS: OK  |  Last: 12:34 PT  |  AI: Ollama ● |
+----------------------------------------------------+
```

| Section | Position | Content |
|---------|----------|---------|
| Data Sources | Left | Status dot + source name + status text, per data source |
| Last Update | Center | "Last: HH:MM PT" timestamp |
| AI Engine | Right | "AI: {engine}" + status dot |

### Status Dot Colors

Maps to `DataSourceStatus` from `contracts/app-types.ts`:

| Status | Color | Animation |
|--------|-------|-----------|
| `connected` | `semantic.success` (#30D158) | Static |
| `fetching` | `semantic.warning` (#FFD60A) | Subtle pulse |
| `error` | `semantic.error` (#FF453A) | Static |
| `idle` | `text.disabled` (rgba 0.20) | Static |

### Three-State Design

| State | Display |
|-------|---------|
| **Empty** (no sources configured) | "No data sources configured" + "Go to Settings" link in `accent.primary` |
| **Loading** (checking status) | Skeleton pulse bars (3 sections) |
| **Error** (status check failed) | "Status unavailable" in `text.tertiary` |

### i18n Consideration

- "Fed RSS: Connected" (en-US) vs "Fed RSS: 已连接" (zh-CN). StatusBar items use flexible spacing (`gap` + `flex`), not fixed widths. Overflow: ellipsis on source names.

---

## 6. Responsive Behavior Summary

| Window Width | Sidebar | MainContent | Notes |
|-------------|---------|-------------|-------|
| >= 1400px | Expanded (240px) | flex: 1 | Default state |
| 1024-1399px | Collapsed (72px) | flex: 1 | Sidebar auto-collapses |
| < 1024px | N/A | N/A | Below minimum window width |

TitleBar and StatusBar are full-width at all sizes.
