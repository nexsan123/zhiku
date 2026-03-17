import { create } from 'zustand';
import type { ApiServiceStatus, PanelId, PanelState } from '@contracts/app-types';
import { MOCK_API_STATUS } from '@utils/mocks/api-status';

export type SituationTab = 'cycle' | 'credit' | 'intel' | 'gameMap';

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
  'credit-cycle',
  'intel-brief',
  'game-map',
  'situation-center',
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

  // CmdK / Settings / Situation tab
  cmdKOpen: boolean;
  setCmdKOpen: (open: boolean) => void;
  settingsOpen: boolean;
  settingsInitialTab: 'data-sources' | 'ai-models' | 'api-keys';
  openSettings: (tab?: 'data-sources' | 'ai-models' | 'api-keys') => void;
  closeSettings: () => void;
  situationTab: SituationTab;
  setSituationTab: (tab: SituationTab) => void;
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

  // CmdK / Settings / Situation tab
  cmdKOpen: false,
  setCmdKOpen: (open: boolean) => set({ cmdKOpen: open }),
  settingsOpen: false,
  settingsInitialTab: 'data-sources',
  openSettings: (tab = 'data-sources') => set({ settingsOpen: true, settingsInitialTab: tab }),
  closeSettings: () => set({ settingsOpen: false }),
  situationTab: 'cycle',
  setSituationTab: (tab) => set({ situationTab: tab }),
}));
