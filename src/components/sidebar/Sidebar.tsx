import { useEffect, type ComponentType } from 'react';
import {
  Globe,
  BarChart3,
  MessageSquare,
  Bell,
  Settings,
  PanelLeftOpen,
  PanelLeftClose,
} from 'lucide-react';
import type { LucideProps } from 'lucide-react';
import { useAppStore } from '@stores/app-store';
import type { PageId } from '@contracts/app-types';
import './Sidebar.css';

interface NavItem {
  page: PageId;
  label: string;
  icon: ComponentType<LucideProps>;
}

/** Top navigation items (excluding Settings) */
const NAV_ITEMS: NavItem[] = [
  { page: 'map', label: 'Map', icon: Globe },
  { page: 'finance', label: 'Finance', icon: BarChart3 },
  { page: 'ai', label: 'AI Chat', icon: MessageSquare },
  { page: 'notifications', label: 'Notifications', icon: Bell },
];

const SETTINGS_ITEM: NavItem = {
  page: 'settings',
  label: 'Settings',
  icon: Settings,
};

export function Sidebar() {
  const { currentPage, sidebarExpanded, setCurrentPage, toggleSidebar } = useAppStore();

  // Ctrl+B keyboard shortcut for toggling sidebar
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent): void => {
      if (e.ctrlKey && e.key === 'b') {
        e.preventDefault();
        toggleSidebar();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [toggleSidebar]);

  const renderItem = (item: NavItem) => {
    const Icon = item.icon;
    const isActive = currentPage === item.page;
    return (
      <button
        key={item.page}
        className={`sidebar__item ${isActive ? 'sidebar__item--active' : ''}`}
        onClick={() => setCurrentPage(item.page)}
        title={item.label}
      >
        <span className="sidebar__icon">
          <Icon />
        </span>
        <span className="sidebar__label">{item.label}</span>
      </button>
    );
  };

  const ToggleIcon = sidebarExpanded ? PanelLeftClose : PanelLeftOpen;

  return (
    <nav className={`sidebar ${sidebarExpanded ? 'sidebar--expanded' : ''}`}>
      <div className="sidebar__nav">
        {NAV_ITEMS.map(renderItem)}
      </div>

      <div className="sidebar__bottom">
        <button
          className="sidebar__item"
          onClick={toggleSidebar}
          title={sidebarExpanded ? 'Collapse Sidebar' : 'Expand Sidebar'}
        >
          <span className="sidebar__icon">
            <ToggleIcon />
          </span>
          <span className="sidebar__label">
            {sidebarExpanded ? 'Collapse' : 'Expand'}
          </span>
        </button>

        <div className="sidebar__separator" />

        {renderItem(SETTINGS_ITEM)}
      </div>
    </nav>
  );
}
