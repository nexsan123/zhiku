import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getSettings, setSetting } from '../../services/tauri-bridge';

interface KeyDef {
  id: string;
  label: string;
}

const KEYS: KeyDef[] = [
  { id: 'fred_api_key', label: 'FRED' },
  { id: 'eia_api_key', label: 'EIA' },
  { id: 'wto_api_key', label: 'WTO' },
];

export function ApiKeysTab() {
  const { t } = useTranslation();
  const [saved, setSaved] = useState<Record<string, string>>({});
  const [values, setValues] = useState<Record<string, string>>({});
  const [showKey, setShowKey] = useState<Record<string, boolean>>({});
  const [saving, setSaving] = useState<Record<string, boolean>>({});

  const load = useCallback(async () => {
    const settings = await getSettings();
    setSaved(settings);
  }, []);

  useEffect(() => { void load(); }, [load]);

  const handleSave = async (keyId: string) => {
    const val = values[keyId];
    if (!val) return;
    setSaving(prev => ({ ...prev, [keyId]: true }));
    try {
      await setSetting(keyId, val);
      setSaved(prev => ({ ...prev, [keyId]: val.slice(0, 3) + '***' + val.slice(-4) }));
      setValues(prev => ({ ...prev, [keyId]: '' }));
    } finally {
      setSaving(prev => ({ ...prev, [keyId]: false }));
    }
  };

  return (
    <div>
      <div className="settings-page__content-title">{t('settings.apiKeys')}</div>
      <div className="settings-key-group">
        <div className="settings-key-group__title">{t('settings.dataSourceKeys')}</div>
        {KEYS.map(k => (
          <div key={k.id} className="settings-key-item">
            <span className="settings-key-item__label">{k.label}</span>
            <input
              className="settings-key-item__input"
              type={showKey[k.id] ? 'text' : 'password'}
              placeholder={saved[k.id] || t('settings.notConfigured')}
              value={values[k.id] ?? ''}
              onChange={(e) => setValues(prev => ({ ...prev, [k.id]: e.target.value }))}
            />
            <button
              className="settings-key-item__btn"
              onClick={() => setShowKey(prev => ({ ...prev, [k.id]: !prev[k.id] }))}
            >
              {showKey[k.id] ? t('settings.hideKey') : t('settings.showKey')}
            </button>
            <button
              className="settings-key-item__btn settings-key-item__btn--save"
              disabled={!values[k.id] || saving[k.id]}
              onClick={() => void handleSave(k.id)}
            >
              {saving[k.id] ? t('settings.saving') : t('settings.saveKey')}
            </button>
            <span className={`settings-key-item__status ${saved[k.id] ? 'settings-key-item__status--configured' : 'settings-key-item__status--missing'}`}>
              {saved[k.id] ? t('settings.configured') : t('settings.notConfigured')}
            </span>
          </div>
        ))}
      </div>
      <div className="settings-coming-soon">
        {t('settings.aiKeysMovedNote')}
      </div>
    </div>
  );
}
