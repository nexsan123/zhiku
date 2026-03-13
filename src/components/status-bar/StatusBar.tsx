import { useState, useEffect, useRef, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useAppStore } from '@stores/app-store';
import { listAiModels } from '@services/tauri-bridge';
import type { AiModelConfig } from '@contracts/app-types';
import './StatusBar.css';

// Loosened type to accommodate backend services not yet in ApiServiceName contract
// (e.g. 'rss', 'fear_greed') which the backend tracks at runtime.
interface ServiceStatusItem {
  service: string;
  status: 'online' | 'offline' | 'checking' | 'idle';
  lastCheck?: string;
  lastError?: string;
  responseMs?: number;
}

interface StatusDotProps {
  item: ServiceStatusItem;
  displayLabel: string;
}

function StatusDot({ item, displayLabel }: StatusDotProps) {
  const { t } = useTranslation();
  const [showTooltip, setShowTooltip] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleMouseEnter = () => {
    timerRef.current = setTimeout(() => setShowTooltip(true), 300);
  };

  const handleMouseLeave = () => {
    if (timerRef.current) clearTimeout(timerRef.current);
    setShowTooltip(false);
  };

  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  return (
    <span
      className="status-dot-item"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      aria-label={`${item.service}: ${item.status}`}
    >
      <span className={`status-dot status-dot--${item.status}`} role="status" />
      <span className="status-dot-label">{displayLabel}</span>

      {showTooltip && (
        <div className="status-dot-tooltip" role="tooltip">
          <div className="status-dot-tooltip__service">{item.service}</div>
          <div className="status-dot-tooltip__status">
            {t('statusBar.status')}
            <span className={`tooltip-status--${item.status}`}>{item.status}</span>
          </div>
          {item.lastCheck && (
            <div className="status-dot-tooltip__row">
              {t('statusBar.lastCheck')}{new Date(item.lastCheck).toLocaleTimeString()}
            </div>
          )}
          {item.responseMs !== undefined && (
            <div className="status-dot-tooltip__row">
              {t('statusBar.response')}{item.responseMs}{t('statusBar.ms')}
            </div>
          )}
          {item.lastError && (
            <div className="status-dot-tooltip__error">{item.lastError}</div>
          )}
        </div>
      )}
    </span>
  );
}

// Ordered list of services to display.
// 'rss' and 'fear_greed' are not yet in ApiServiceName contract but are valid
// runtime service names tracked by the backend.
const SERVICE_ORDER: string[] = [
  'ollama', 'groq', 'claude', 'fred', 'yahoo', 'eia', 'bis', 'imf', 'wto', 'coingecko', 'rss', 'fear_greed', 'qt_rest', 'qt_ws',
];

// Custom display labels for services with underscores or special names
const SERVICE_LABEL_MAP: Record<string, string> = {
  fear_greed: 'F&G',
};

function getServiceLabel(service: string): string {
  return SERVICE_LABEL_MAP[service] ?? service.toUpperCase();
}

export function StatusBar() {
  const { t } = useTranslation();
  const apiStatus = useAppStore((s) => s.apiStatus);
  const [currentTime, setCurrentTime] = useState(
    () => new Date().toLocaleTimeString()
  );
  const [aiModels, setAiModels] = useState<AiModelConfig[]>([]);

  const loadAiModels = useCallback(async () => {
    const models = await listAiModels();
    setAiModels(models.filter(m => m.enabled));
  }, []);

  // Load AI models on mount and refresh every 30s
  useEffect(() => {
    void loadAiModels();
    const id = setInterval(() => void loadAiModels(), 30_000);
    return () => clearInterval(id);
  }, [loadAiModels]);

  // Clock: update every second, cleanup on unmount
  useEffect(() => {
    const id = setInterval(() => {
      setCurrentTime(new Date().toLocaleTimeString());
    }, 1000);
    return () => clearInterval(id);
  }, []);

  const statusValues = Object.values(apiStatus);
  const onlineCount = statusValues.filter((s) => s.status === 'online').length;
  const hasOffline = statusValues.some((s) => s.status === 'offline');
  const readyLabel = hasOffline
    ? t('statusBar.degraded')
    : onlineCount > 0
    ? t('statusBar.ready')
    : t('statusBar.initializing');
  const readyClass = hasOffline ? 'status-bar__ready--error' : 'status-bar__ready--ok';

  return (
    <footer className="status-bar">
      <div className="status-bar__indicators">
        {SERVICE_ORDER.map((key) => {
          const item = apiStatus[key] as ServiceStatusItem | undefined;
          if (!item) return null;
          return (
            <StatusDot
              key={key}
              item={item}
              displayLabel={getServiceLabel(key)}
            />
          );
        })}
      </div>

      {aiModels.length > 0 && (
        <div className="status-bar__ai-models">
          <span className="status-bar__ai-label">AI</span>
          {aiModels.map((m) => {
            const providerStatus = apiStatus[m.provider] as ServiceStatusItem | undefined;
            const dotClass = providerStatus?.status ?? 'idle';
            return (
              <span key={m.id} className="status-dot-item" title={`${m.displayName} · ${m.modelName}`}>
                <span className={`status-dot status-dot--${dotClass}`} role="status" />
                <span className="status-dot-label">{m.displayName}</span>
              </span>
            );
          })}
        </div>
      )}

      <div className="status-bar__right">
        <span className={`status-bar__ready ${readyClass}`}>{readyLabel}</span>
        <span className="status-bar__separator">·</span>
        <time className="status-bar__time" dateTime={currentTime}>
          {currentTime}
        </time>
      </div>
    </footer>
  );
}
