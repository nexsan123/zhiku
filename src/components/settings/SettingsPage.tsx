import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Database, Bot, Key } from 'lucide-react';
import { DataSourcesTab } from './DataSourcesTab';
import { AiModelsTab } from './AiModelsTab';
import { ApiKeysTab } from './ApiKeysTab';
import './SettingsPage.css';

type Tab = 'data-sources' | 'ai-models' | 'api-keys';

interface Props {
  open: boolean;
  onClose: () => void;
  initialTab?: Tab;
}

export function SettingsPage({ open, onClose, initialTab = 'data-sources' }: Props) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<Tab>(initialTab);

  // Sync activeTab when opened with a specific tab
  useEffect(() => {
    if (open) setActiveTab(initialTab);
  }, [open, initialTab]);

  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [open, onClose]);

  if (!open) return null;

  const tabs: { id: Tab; icon: typeof Database; label: string }[] = [
    { id: 'data-sources', icon: Database, label: t('settings.dataSources') },
    { id: 'ai-models', icon: Bot, label: t('settings.aiModels') },
    { id: 'api-keys', icon: Key, label: t('settings.apiKeys') },
  ];

  return (
    <div className="settings-overlay" onClick={onClose}>
      <div className="settings-page" onClick={(e) => e.stopPropagation()}>
        <div className="settings-page__sidebar">
          <div className="settings-page__sidebar-title">{t('settings.title')}</div>
          {tabs.map(({ id, icon: Icon, label }) => (
            <button
              key={id}
              className={`settings-page__tab ${activeTab === id ? 'settings-page__tab--active' : ''}`}
              onClick={() => setActiveTab(id)}
            >
              <Icon size={14} />
              {label}
            </button>
          ))}
        </div>
        <div className="settings-page__content">
          {activeTab === 'data-sources' && <DataSourcesTab />}
          {activeTab === 'ai-models' && <AiModelsTab />}
          {activeTab === 'api-keys' && <ApiKeysTab />}
        </div>
      </div>
    </div>
  );
}
