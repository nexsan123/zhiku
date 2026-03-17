import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getRssSources, getSettings, setSetting } from '../../services/tauri-bridge';
import type { RssSource } from '@contracts/app-types';

export function DataSourcesTab() {
  const { t } = useTranslation();
  const [sources, setSources] = useState<RssSource[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [search, setSearch] = useState('');

  // RSSHub Base URL config
  const [rsshubUrl, setRsshubUrl] = useState('');
  const [rsshubSaving, setRsshubSaving] = useState(false);
  const [rsshubLoaded, setRsshubLoaded] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const [data, settings] = await Promise.all([getRssSources(), getSettings()]);
      setSources(data);
      setRsshubLoaded(settings['rsshub_base_url'] ?? '');
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const handleSaveRsshub = async () => {
    const val = rsshubUrl.trim() || 'https://rsshub.app';
    setRsshubSaving(true);
    try {
      await setSetting('rsshub_base_url', val);
      setRsshubLoaded(val);
      setRsshubUrl('');
    } finally {
      setRsshubSaving(false);
    }
  };

  useEffect(() => { void load(); }, [load]);

  const filtered = useMemo(() => {
    if (!search) return sources;
    const q = search.toLowerCase();
    return sources.filter(s => s.name.toLowerCase().includes(q) || s.url.toLowerCase().includes(q));
  }, [sources, search]);

  const grouped = useMemo(() => {
    const groups: Record<number, RssSource[]> = {};
    for (const s of filtered) {
      (groups[s.tier] ??= []).push(s);
    }
    return groups;
  }, [filtered]);

  if (loading) return <div className="settings-coming-soon">{t('state.loading')}</div>;
  if (error) return (
    <div className="settings-coming-soon">
      {error}{' '}
      <button onClick={() => void load()}>{t('state.retry')}</button>
    </div>
  );

  return (
    <div>
      <div className="settings-page__content-title">{t('settings.dataSources')}</div>

      {/* RSSHub Base URL configuration */}
      <div className="settings-key-group">
        <div className="settings-key-group__title">{t('settings.rsshubBaseUrl')}</div>
        <div className="settings-key-item">
          <span className="settings-key-item__label">Base URL</span>
          <input
            className="settings-key-item__input"
            type="text"
            placeholder={rsshubLoaded || 'https://rsshub.app'}
            value={rsshubUrl}
            onChange={(e) => setRsshubUrl(e.target.value)}
          />
          <button
            className="settings-key-item__btn settings-key-item__btn--save"
            disabled={rsshubSaving}
            onClick={() => void handleSaveRsshub()}
          >
            {rsshubSaving ? t('settings.saving') : t('settings.saveKey')}
          </button>
          <span className="settings-key-item__status settings-key-item__status--configured">
            {rsshubLoaded || 'https://rsshub.app'}
          </span>
        </div>
        <p className="settings-coming-soon" style={{ fontSize: '11px', marginTop: '4px', marginBottom: '0' }}>
          {t('settings.rsshubBaseUrlDesc')}
        </p>
      </div>

      <input
        className="settings-search"
        placeholder={t('settings.search')}
        value={search}
        onChange={(e) => setSearch(e.target.value)}
      />
      <div className="settings-source-list">
        {Object.entries(grouped)
          .sort(([a], [b]) => Number(a) - Number(b))
          .map(([tier, items]) => (
            <div key={tier} className="settings-tier-group">
              <div className="settings-tier-group__title">{t(`settings.tierLabel${tier}`, { defaultValue: t('settings.tier', { n: tier }) })}</div>
              {items.map((s) => (
                <div key={s.url} className="settings-source-item">
                  <span className={`status-dot status-dot--${s.enabled ? 'online' : 'idle'}`} />
                  <span className="settings-source-item__name">{s.name}</span>
                  <span className="settings-source-item__url">{s.url}</span>
                  <span className="settings-source-item__badge">{s.language}</span>
                  <button
                    className={`settings-source-item__toggle ${s.enabled ? 'settings-source-item__toggle--on' : ''}`}
                    aria-label={s.enabled ? t('settings.disable') : t('settings.enable')}
                  />
                </div>
              ))}
            </div>
          ))}
      </div>
    </div>
  );
}
