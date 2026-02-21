// ============================================================
// 智库 Design Token System
// Dark theme — Intelligence/Analysis aesthetic
// Based on QuantTerminal token structure, accent: Teal (#00C2A8)
// ============================================================

export const theme = {
  // ----------------------------------------------------------
  // Color System
  // ----------------------------------------------------------
  colors: {
    // --- Background layers (darkest -> lightest) ---
    bg: {
      /** App base background — near-black with slight blue tint */
      base: '#0D0D12',
      /** Primary surfaces: sidebar, panels */
      primary: '#141419',
      /** Elevated surfaces: cards, modals */
      elevated: '#1C1C24',
      /** Overlays: dropdowns, tooltips */
      overlay: '#252530',
      /** Hover state for interactive surfaces */
      hover: '#2A2A36',
      /** Active/pressed state */
      active: '#32323F',
    },

    // --- Text layers ---
    text: {
      /** Primary text — high contrast, not pure white */
      primary: 'rgba(255, 255, 255, 0.92)',
      /** Secondary text — labels, descriptions */
      secondary: 'rgba(255, 255, 255, 0.56)',
      /** Tertiary text — placeholders, captions */
      tertiary: 'rgba(255, 255, 255, 0.36)',
      /** Disabled text */
      disabled: 'rgba(255, 255, 255, 0.20)',
    },

    // --- Accent color (Teal — intelligence/analysis) ---
    accent: {
      /** Primary accent — teal for intel aesthetic */
      primary: '#00C2A8',
      /** Hover state — lightened */
      hover: '#33D4BE',
      /** Active/pressed state — darkened */
      active: '#009E88',
      /** Subtle background tint for accent areas */
      subtle: 'rgba(0, 194, 168, 0.12)',
    },

    // --- Semantic colors ---
    semantic: {
      /** Success — connected status, positive outcomes */
      success: '#30D158',
      successSubtle: 'rgba(48, 209, 88, 0.12)',
      /** Warning — checking, pending states */
      warning: '#FFD60A',
      warningSubtle: 'rgba(255, 214, 10, 0.12)',
      /** Error — failures, critical alerts */
      error: '#FF453A',
      errorSubtle: 'rgba(255, 69, 58, 0.12)',
      /** Info — neutral informational */
      info: '#64D2FF',
      infoSubtle: 'rgba(100, 210, 255, 0.12)',
    },

    // --- Intel-specific colors ---
    // Priority levels (notification severity)
    // Category colors (news classification)
    intel: {
      /** P0 Critical — shares error red (urgent, demands attention) */
      p0: '#FF453A',
      p0Subtle: 'rgba(255, 69, 58, 0.15)',
      /** P1 Important — amber/orange (notable, non-urgent) */
      p1: '#FF9F0A',
      p1Subtle: 'rgba(255, 159, 10, 0.15)',
      /** P2 Routine — accent teal (standard, calm) */
      p2: '#00C2A8',
      p2Subtle: 'rgba(0, 194, 168, 0.12)',

      /** Geopolitical — warm brick-red (distinct from semantic.error #FF453A) */
      geopolitical: '#D4553A',
      geopoliticalSubtle: 'rgba(212, 85, 58, 0.15)',
      /** Macro Policy — blue-violet */
      macroPolicy: '#7B68EE',
      macroPolicySubtle: 'rgba(123, 104, 238, 0.15)',
      /** Market — accent teal (same as accent.primary) */
      market: '#00C2A8',
      marketSubtle: 'rgba(0, 194, 168, 0.12)',
      /** Corporate — amber/gold */
      corporate: '#D4A03A',
      corporateSubtle: 'rgba(212, 160, 58, 0.15)',
    },

    // --- Border system ---
    border: {
      /** Default border — subtle separator */
      primary: 'rgba(255, 255, 255, 0.08)',
      /** Stronger border — card outlines, input borders */
      secondary: 'rgba(255, 255, 255, 0.12)',
      /** Focus ring / active border — teal accent */
      focus: 'rgba(0, 194, 168, 0.48)',
      /** Error border */
      error: 'rgba(255, 69, 58, 0.48)',
    },
  },

  // ----------------------------------------------------------
  // Shadow System
  // ----------------------------------------------------------
  shadows: {
    /** Subtle shadow — buttons, small elements */
    sm: '0 1px 2px rgba(0, 0, 0, 0.24), 0 0 1px rgba(0, 0, 0, 0.12)',
    /** Medium shadow — cards, dropdowns */
    md: '0 4px 12px rgba(0, 0, 0, 0.32), 0 0 1px rgba(0, 0, 0, 0.12)',
    /** Large shadow — modals, popovers */
    lg: '0 12px 40px rgba(0, 0, 0, 0.48), 0 0 1px rgba(0, 0, 0, 0.12)',
  },

  // ----------------------------------------------------------
  // Border Radius
  // ----------------------------------------------------------
  radius: {
    /** Small — tags, badges, small buttons */
    sm: '6px',
    /** Medium — cards, inputs, standard buttons */
    md: '10px',
    /** Large — modals, panels */
    lg: '14px',
    /** Extra large — floating containers */
    xl: '20px',
    /** Full circle — avatars, status dots */
    full: '9999px',
  },

  // ----------------------------------------------------------
  // Spacing (4px base grid)
  // ----------------------------------------------------------
  spacing: {
    /** 2px — micro adjustments */
    '0.5': '2px',
    /** 4px — tight spacing */
    '1': '4px',
    /** 8px — default small gap */
    '2': '8px',
    /** 12px — compact spacing */
    '3': '12px',
    /** 16px — standard spacing */
    '4': '16px',
    /** 20px — comfortable spacing */
    '5': '20px',
    /** 24px — section spacing */
    '6': '24px',
    /** 32px — large section spacing */
    '8': '32px',
    /** 40px — extra spacing */
    '10': '40px',
    /** 48px — layout spacing */
    '12': '48px',
  },

  // ----------------------------------------------------------
  // Animation
  // ----------------------------------------------------------
  animation: {
    duration: {
      /** Fast — hover effects, small state changes */
      fast: '150ms',
      /** Normal — standard transitions */
      normal: '250ms',
      /** Slow — layout shifts, modal open/close */
      slow: '350ms',
    },
    easing: {
      /** Default — Apple-style deceleration curve */
      default: 'cubic-bezier(0.25, 0.1, 0.25, 1.0)',
      /** Ease in — elements entering */
      easeIn: 'cubic-bezier(0.42, 0, 1, 1)',
      /** Ease out — elements exiting */
      easeOut: 'cubic-bezier(0, 0, 0.58, 1)',
      /** Spring-like — interactive feedback */
      spring: 'cubic-bezier(0.34, 1.56, 0.64, 1)',
    },
  },

  // ----------------------------------------------------------
  // Typography
  // ----------------------------------------------------------
  typography: {
    fontFamily: {
      /** Primary sans-serif — UI text */
      sans: '-apple-system, "SF Pro Display", "Inter", "Segoe UI", system-ui, sans-serif',
      /** Monospace — code, numbers, data tables */
      mono: '"SF Mono", "JetBrains Mono", "Cascadia Code", "Consolas", monospace',
    },
    fontWeight: {
      /** Regular — body text */
      regular: 400,
      /** Medium — labels, emphasis */
      medium: 500,
      /** Semibold — headings, buttons */
      semibold: 600,
    },
    fontSize: {
      /** 11px — captions, status bar text */
      xs: '11px',
      /** 12px — small labels, secondary info */
      sm: '12px',
      /** 13px — default body text */
      base: '13px',
      /** 14px — emphasized body, input text */
      md: '14px',
      /** 16px — section headings */
      lg: '16px',
      /** 20px — page titles */
      xl: '20px',
      /** 24px — hero headings */
      '2xl': '24px',
    },
    lineHeight: {
      /** Tight — headings */
      tight: '1.2',
      /** Normal — body text */
      normal: '1.5',
      /** Relaxed — paragraphs */
      relaxed: '1.7',
    },
  },

  // ----------------------------------------------------------
  // Blur (backdrop-filter)
  // ----------------------------------------------------------
  blur: {
    /** Sidebar panels — moderate blur */
    sidebar: '20px',
    /** Modal backgrounds — heavy blur */
    modal: '40px',
    /** Tooltips — light blur */
    tooltip: '10px',
    /** Title bar — moderate blur */
    titleBar: '24px',
    /** Status bar — light blur */
    statusBar: '16px',
  },

  // ----------------------------------------------------------
  // Z-index Scale
  // ----------------------------------------------------------
  zIndex: {
    /** Below default — hidden panels */
    behind: -1,
    /** Default layer */
    base: 0,
    /** Sticky elements — headers within scroll areas */
    sticky: 10,
    /** Sidebar — always above content */
    sidebar: 20,
    /** Title bar — above sidebar */
    titleBar: 30,
    /** Status bar */
    statusBar: 25,
    /** Dropdown menus */
    dropdown: 40,
    /** Tooltip / hover card */
    tooltip: 50,
    /** Modals and overlays */
    modal: 60,
    /** Toast notifications — always on top */
    toast: 70,
  },

  // ----------------------------------------------------------
  // Glassmorphism
  // ----------------------------------------------------------
  glass: {
    bg: {
      /** Structural surfaces — titlebar, sidebar, statusbar */
      structural: 'rgba(20, 20, 28, 0.72)',
      /** Content cards — panels, data containers */
      card: 'rgba(28, 28, 40, 0.45)',
      /** Elevated surfaces — modals, dropdowns, tooltips */
      elevated: 'rgba(36, 36, 48, 0.82)',
    },
    border: {
      /** Structural element edges — faintest glow */
      subtle: 'rgba(255, 255, 255, 0.06)',
      /** Card / panel edges — default glass border */
      default: 'rgba(255, 255, 255, 0.08)',
      /** Elevated surface edges — most visible */
      strong: 'rgba(255, 255, 255, 0.12)',
    },
    glow: {
      /** Primary accent glow — interactive hover, focus (teal) */
      accent: 'rgba(0, 194, 168, 0.10)',
      /** Success glow — connected status indicators */
      success: 'rgba(48, 209, 88, 0.10)',
      /** Error glow — error states, disconnected indicators */
      error: 'rgba(255, 69, 58, 0.10)',
      /** AI purple glow — AI-related components */
      ai: 'rgba(191, 90, 242, 0.10)',
    },
    mesh: {
      /** Teal accent radial — top-left region of app backdrop */
      accent: 'rgba(0, 194, 168, 0.04)',
      /** AI purple radial — bottom-right region of app backdrop */
      purple: 'rgba(191, 90, 242, 0.035)',
      /** Warm highlight radial — center of app backdrop */
      warm: 'rgba(255, 140, 50, 0.02)',
    },
  },

  // ----------------------------------------------------------
  // Layout Dimensions
  // ----------------------------------------------------------
  layout: {
    titleBar: {
      height: '52px',
    },
    sidebar: {
      collapsedWidth: '72px',
      expandedWidth: '240px',
    },
    statusBar: {
      height: '32px',
    },
  },

  // ----------------------------------------------------------
  // Map-specific tokens
  // ----------------------------------------------------------
  map: {
    /** Ocean background — darker than app base, slight blue-green tint */
    ocean: '#080B10',
    /** Country border stroke width */
    borderWidth: '2px',
    /** Fog opacity for countries with no data */
    fogOpacity: 0.25,
    /** Fog hover — slight brightening on hover */
    fogHoverOpacity: 0.45,
    /** Pulse animation duration for P0 events */
    pulseDuration: '2s',
    /** Glow radius for P0 event countries */
    pulseGlowRadius: '12px',
    /** Hover info card width */
    hoverCardWidth: '280px',
    /** Bottom summary bar height */
    summaryBarHeight: '48px',
  },
} as const;

export type Theme = typeof theme;
