# @coder-fe Memory — 智库项目

## Project Structure (confirmed)

- Path aliases: `@contracts/*` → `./contracts/*`, `@components/*` → `./src/components/*`, `@stores/*` → `./src/stores/*`, `@utils/*` → `./src/utils/*`
- Mock data location: `src/utils/mocks/` (uses existing `@utils` alias — no separate `@mocks` alias exists)
- Component pattern: `src/components/[name]/[Name].tsx + [Name].css + index.ts`
- CSS BEM: `component-name__element--modifier`

## Key Variables in variables.css

- Layout: `--title-bar-height: 52px`, `--status-bar-height: 32px`
- No `--panel-width` token — write 320px directly in component CSS (approved by Lead, Task 4.1)
- `--blur-sidebar: 20px` is reused for Panel Tier-2 glass blur (glassmorphism-spec.md)

## Glassmorphism Rules

- Tier 2 Card Glass: `background: var(--glass-bg-card); backdrop-filter: blur(20px); border: 1px solid var(--glass-border-default); border-radius: var(--radius-lg)`
- NEVER nest backdrop-filter more than 1 level — PanelStack must NOT have backdrop-filter if child Panel does
- Glow via box-shadow: `0 0 20px var(--glass-glow-accent)` on hover

## Write Tool Rule

Always Read a file before Write — even for new content that overwrites. The tool requires prior read.

## Phase 4 Architecture

- Three-column layout: PanelStack(left, 320px) + MapCenter(flex:1) + PanelStack(right, 320px)
- Store: Zustand with `leftPanelCollapsed`, `rightPanelCollapsed`, `panels: Record<PanelId, PanelState>`, `apiStatus`
- StatusBar: reads apiStatus from store, has clock via `useEffect+setInterval` with cleanup
- Panel collapse: `panels[panelId]?.expanded ?? true` (safe fallback)
- All 14 PanelIds initialized in store with `{expanded: true}`

## Service Layer (Wave 2)

- `src/services/tauri-bridge.ts` — single bridge for all Tauri invoke/listen calls
- `@services/*` alias is already configured in tsconfig.json and vite.config.ts
- `isTauri()` detection: `typeof window !== 'undefined' && '__TAURI__' in window`
- Tauri listen() returns `Promise<UnlistenFn>` — cleanup pattern in useEffect:
  ```
  let cleanup: (() => void) | null = null;
  const p = listenXxx(cb);
  void p.then(fn => { cleanup = fn; });
  return () => { if (cleanup) cleanup(); else void p.then(fn => fn()); };
  ```
- Rust `serde(rename_all = "camelCase")` — `source_url` → `sourceUrl`, `published_at` → `publishedAt`, etc.

## Panel CSS Animation Pattern (Wave 2)

- Fold animation: CSS Grid rows `0fr → 1fr`, NOT max-height
- Panel.tsx: always render `panel__body`, control with `panel__body--expanded` class + `aria-hidden`
- Panel.css: `panel__body { display:grid; grid-template-rows:0fr; transition:grid-template-rows }`, inner div has `overflow:hidden; padding`
- Panel.tsx Panel.css changed: body is always in DOM (not conditional `&&`), aria-hidden={!expanded}

## Three-State Pattern (confirmed for all panels)

```
type LoadState = 'loading' | 'loaded' | 'error';
// loading: spinner icon + text
// error: red message + retry button with void load()
// empty: icon + text + sub-text
// loaded: actual list/content
```

## FRED indicator 字段名（重要！2026-03-09 Lead 确认）

- 后端 fred_client.rs 存储大写 FRED series ID: `FEDFUNDS`, `CPIAUCSL`, `UNRATE`, `GDP`, `M2SL`
- mock 数据（tauri-bridge.ts getMacroData）已对齐为大写
- 非FRED来源使用小写: `fear_greed_index`, `wti_crude`, `brent_crude`

## Phase 4 面板完成状态（2026-03-09）

左栏 (4/4): NewsFeed ✅ AiBrief ✅ FredPanel ✅ BisPanel ✅
右栏 (9总): MarketRadar ✅ Indices ✅ Forex ✅ OilEnergy ✅ Crypto ✅ FearGreed ✅ WtoPanel ✅ SupplyChainPanel ✅ GulfFdiPanel ✅

注: BIS/WTO/SupplyChain/GulfFDI 为静态数据面板（无 fetch，直接渲染）
注: AiBriefPanel 在 Tauri 环境会进 error 态（get_ai_brief 尚未在 Phase 3 实现）
注: FredPanel GDP/M2SL mock 无数据 → 显示 `--`（正常，后端写入后自动显示）
