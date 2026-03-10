import { create } from 'zustand';
import type { ApiServiceStatus, PanelId, PanelState } from '@contracts/app-types';
import { MOCK_API_STATUS } from '@utils/mocks/api-status';

// 所有面板 ID（左栏 7 + 右栏 8）
const ALL_PANEL_IDS: PanelId[] = [
  'cycle-reasoning',
  'news-feed',
  'ai-brief',
  'fred-indicators',
  'bis-rates',
  'wto-trade',
  'supply-chain',
  'market-radar',
  'indices',
  'forex',
  'oil-energy',
  'crypto',
  'btc-etf',
  'fear-greed',
  'gulf-fdi',
];

// 初始面板状态 — 全部展开
const initialPanels: Record<PanelId, PanelState> = Object.fromEntries(
  ALL_PANEL_IDS.map((id) => [id, { expanded: true }])
) as Record<PanelId, PanelState>;

interface AppState {
  // 左右栏折叠状态
  leftPanelCollapsed: boolean;
  rightPanelCollapsed: boolean;
  panels: Record<PanelId, PanelState>;
  toggleLeftPanel: () => void;
  toggleRightPanel: () => void;
  togglePanel: (panelId: PanelId) => void;

  // API 状态灯
  apiStatus: Record<string, ApiServiceStatus>;
  updateApiStatus: (service: string, status: ApiServiceStatus) => void;

  // 计数
  intelCount: number;
  notificationCount: number;
}

export const useAppStore = create<AppState>((set) => ({
  // 左右栏折叠状态（默认展开）
  leftPanelCollapsed: false,
  rightPanelCollapsed: false,
  panels: initialPanels,

  toggleLeftPanel: () =>
    set((s) => ({ leftPanelCollapsed: !s.leftPanelCollapsed })),

  toggleRightPanel: () =>
    set((s) => ({ rightPanelCollapsed: !s.rightPanelCollapsed })),

  togglePanel: (panelId: PanelId) =>
    set((s) => ({
      panels: {
        ...s.panels,
        [panelId]: { expanded: !(s.panels[panelId]?.expanded ?? true) },
      },
    })),

  // API 状态灯 — 初始从 mock 数据加载，联调后由 Tauri event 覆盖
  apiStatus: MOCK_API_STATUS,

  updateApiStatus: (service: string, status: ApiServiceStatus) =>
    set((s) => ({
      apiStatus: { ...s.apiStatus, [service]: status },
    })),

  // 计数（Phase 4 mock 初始值）
  intelCount: 0,
  notificationCount: 0,
}));
