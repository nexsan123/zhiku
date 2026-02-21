import './StatusBar.css';

interface StatusDotProps {
  status: 'connected' | 'fetching' | 'error' | 'idle';
  label: string;
}

function StatusDot({ status, label }: StatusDotProps) {
  return (
    <span className="status-dot-item">
      <span className={`status-dot status-dot--${status}`} />
      <span className="status-dot-label">{label}</span>
    </span>
  );
}

const INDICATORS: { label: string; status: 'connected' | 'fetching' | 'error' | 'idle' }[] = [
  { label: 'News API', status: 'idle' },
  { label: 'Ollama', status: 'idle' },
  { label: 'Claude', status: 'idle' },
];

export function StatusBar() {
  return (
    <footer className="status-bar">
      <div className="status-bar__indicators">
        {INDICATORS.map((indicator) => (
          <StatusDot
            key={indicator.label}
            status={indicator.status}
            label={indicator.label}
          />
        ))}
      </div>

      <div className="status-bar__right">
        <span className="status-bar__text">Ready</span>
      </div>
    </footer>
  );
}
