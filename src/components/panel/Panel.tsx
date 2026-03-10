import { ChevronDown, ChevronRight } from 'lucide-react';
import { useAppStore } from '@stores/app-store';
import type { PanelId } from '@contracts/app-types';
import './Panel.css';

interface PanelProps {
  title: string;
  icon: React.ReactNode;
  panelId: PanelId;
  children: React.ReactNode;
}

export function Panel({ title, icon, panelId, children }: PanelProps) {
  const panels = useAppStore((s) => s.panels);
  const togglePanel = useAppStore((s) => s.togglePanel);
  const expanded = panels[panelId]?.expanded ?? true;

  return (
    <section className={`panel ${expanded ? 'panel--expanded' : 'panel--collapsed'}`}>
      <button
        className="panel__header"
        onClick={() => togglePanel(panelId)}
        aria-expanded={expanded}
        aria-controls={`panel-body-${panelId}`}
      >
        <span className="panel__icon">{icon}</span>
        <span className="panel__title">{title}</span>
        <span className="panel__chevron">
          {expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
        </span>
      </button>

      <div
        className={`panel__body ${expanded ? 'panel__body--expanded' : ''}`}
        id={`panel-body-${panelId}`}
        role="region"
        aria-hidden={!expanded}
      >
        <div className="panel__body-inner">{children}</div>
      </div>
    </section>
  );
}
