import './PanelStack.css';

interface PanelStackProps {
  side: 'left' | 'right';
  collapsed: boolean;
  children: React.ReactNode;
}

export function PanelStack({ side, collapsed, children }: PanelStackProps) {
  return (
    <aside
      className={[
        'panel-stack',
        `panel-stack--${side}`,
        collapsed ? 'panel-stack--collapsed' : '',
      ]
        .filter(Boolean)
        .join(' ')}
      aria-label={`${side} panel stack`}
    >
      <div className="panel-stack__inner">{children}</div>
    </aside>
  );
}
