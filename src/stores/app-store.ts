import { create } from 'zustand';
import type { PageId } from '@contracts/app-types';

interface AppState {
  currentPage: PageId;
  sidebarExpanded: boolean;
  setCurrentPage: (page: PageId) => void;
  toggleSidebar: () => void;
}

export const useAppStore = create<AppState>((set) => ({
  currentPage: 'map',
  sidebarExpanded: false,
  setCurrentPage: (page) => set({ currentPage: page }),
  toggleSidebar: () => set((s) => ({ sidebarExpanded: !s.sidebarExpanded })),
}));
