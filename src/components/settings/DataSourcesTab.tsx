import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getRssSources } from '../../services/tauri-bridge';
import type { RssSource } from '@contracts/app-types';

export function DataSourcesTab() {
  const { t } = useTranslation();
  const [sources, setSources] = useState<RssSource[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [search, setSearch] = useState('');

  const load = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const data = await getRssSources();
      setSources(data);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

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
              <div className="settings-tier-group__title">Tier {tier}</div>
              {items.map((s) => (
                <div key={s.url} className="settings-source-item">
                  <span className={`status-dot status-dot--${s.enabled ? 'online' : 'idle'}`} />
                  <span className="settings-source-item__name">{s.name}</span>
                  <span className="settings-source-item__url">{s.url}</span>
                  <span className="settings-source-item__badge">{s.language}</span>
                  <button
                    className={`settings-source-item__toggle ${s.enabled ? 'settings-source-item__toggle--on' : ''}`}
                    aria-label={s.enabled ? 'Disable' : 'Enable'}
                  />
                </div>
              ))}
            </div>
          ))}
      </div>
    </div>
  );
}
