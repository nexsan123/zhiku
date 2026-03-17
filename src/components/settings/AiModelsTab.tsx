import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Plus, Trash2, X } from 'lucide-react';
import { listAiModels, saveAiModel, removeAiModel, testAiModel } from '../../services/tauri-bridge';
import type { AiModelConfig, AiProviderType } from '@contracts/app-types';

const PROVIDERS: { id: AiProviderType; label: string; needsKey: boolean; needsEndpoint: boolean; defaultEndpoint: string; defaultModel: string }[] = [
  { id: 'ollama', label: 'Ollama', needsKey: false, needsEndpoint: true, defaultEndpoint: 'http://localhost:11434', defaultModel: 'llama3.1:8b' },
  { id: 'groq', label: 'Groq', needsKey: true, needsEndpoint: false, defaultEndpoint: '', defaultModel: 'llama-3.1-8b-instant' },
  { id: 'claude', label: 'Claude (Anthropic)', needsKey: true, needsEndpoint: false, defaultEndpoint: '', defaultModel: 'claude-sonnet-4-20250514' },
  { id: 'openai', label: 'OpenAI', needsKey: true, needsEndpoint: false, defaultEndpoint: '', defaultModel: 'gpt-4o' },
  { id: 'gemini', label: 'Google Gemini', needsKey: true, needsEndpoint: false, defaultEndpoint: '', defaultModel: 'gemini-2.0-flash' },
  { id: 'deepseek', label: 'DeepSeek', needsKey: true, needsEndpoint: false, defaultEndpoint: '', defaultModel: 'deepseek-chat' },
  { id: 'openai_compatible', label: 'Custom (OpenAI Compatible)', needsKey: true, needsEndpoint: true, defaultEndpoint: '', defaultModel: '' },
];

type TestStatus = 'idle' | 'testing' | 'ok' | 'error';

export function AiModelsTab() {
  const { t } = useTranslation();
  const [models, setModels] = useState<AiModelConfig[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [testState, setTestState] = useState<Record<string, { status: TestStatus; message: string; ms: number }>>({});

  // Form state
  const [formProvider, setFormProvider] = useState<AiProviderType>('groq');
  const [formName, setFormName] = useState('');
  const [formKey, setFormKey] = useState('');
  const [formModel, setFormModel] = useState('');
  const [formEndpoint, setFormEndpoint] = useState('');
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    const data = await listAiModels();
    setModels(data);
    setLoading(false);
  }, []);

  useEffect(() => { void load(); }, [load]);

  const providerDef = PROVIDERS.find(p => p.id === formProvider)!;

  const resetForm = () => {
    setFormProvider('groq');
    setFormName('');
    setFormKey('');
    setFormModel('');
    setFormEndpoint('');
    setShowForm(false);
  };

  // Auto-fill defaults when provider changes
  const handleProviderChange = (provider: AiProviderType) => {
    setFormProvider(provider);
    const def = PROVIDERS.find(p => p.id === provider)!;
    setFormName(def.label);
    setFormModel(def.defaultModel);
    setFormEndpoint(def.defaultEndpoint);
    setFormKey('');
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      const model: AiModelConfig = {
        id: `${formProvider}-${Date.now()}`,
        provider: formProvider,
        displayName: formName || providerDef.label,
        apiKey: formKey,
        modelName: formModel || providerDef.defaultModel,
        endpointUrl: formEndpoint || providerDef.defaultEndpoint,
        enabled: true,
      };
      await saveAiModel(model);
      await load();
      resetForm();
    } finally {
      setSaving(false);
    }
  };

  const handleRemove = async (id: string) => {
    await removeAiModel(id);
    await load();
  };

  const handleTest = async (modelId: string) => {
    setTestState(prev => ({ ...prev, [modelId]: { status: 'testing', message: '', ms: 0 } }));
    const result = await testAiModel(modelId);
    setTestState(prev => ({
      ...prev,
      [modelId]: {
        status: result.success ? 'ok' : 'error',
        message: result.message,
        ms: result.responseMs,
      },
    }));
  };

  const dotStatus = (id: string) => {
    const s = testState[id];
    if (!s) return 'idle';
    if (s.status === 'ok') return 'online';
    if (s.status === 'error') return 'error';
    if (s.status === 'testing') return 'checking';
    return 'idle';
  };

  if (loading) return <div className="settings-coming-soon">{t('state.loading')}</div>;

  return (
    <div>
      <div className="settings-page__content-title">{t('settings.aiModels')}</div>

      {/* Configured models */}
      {models.length === 0 && (
        <div className="settings-coming-soon">{t('settings.noModels')}</div>
      )}

      {models.map((m) => {
        const state = testState[m.id];
        return (
          <div key={m.id} className="settings-model-card">
            <span className={`status-dot status-dot--${dotStatus(m.id)}`} />
            <div className="settings-model-card__info">
              <div className="settings-model-card__name">{m.displayName}</div>
              <div className="settings-model-card__desc">
                {m.provider} · {m.modelName} {m.apiKey && `· Key: ${m.apiKey}`}
              </div>
            </div>
            <div className="settings-model-card__actions">
              {state?.status === 'ok' && (
                <span className="settings-model-card__result settings-model-card__result--ok">
                  {state.message} ({state.ms}ms)
                </span>
              )}
              {state?.status === 'error' && (
                <span className="settings-model-card__result settings-model-card__result--error">
                  {state.message}
                </span>
              )}
              <button
                className="settings-model-card__test-btn"
                disabled={state?.status === 'testing'}
                onClick={() => void handleTest(m.id)}
              >
                {state?.status === 'testing' ? t('settings.testing') : t('settings.testConnection')}
              </button>
              <button
                className="settings-model-card__test-btn"
                onClick={() => void handleRemove(m.id)}
                aria-label="Remove"
              >
                <Trash2 size={13} />
              </button>
            </div>
          </div>
        );
      })}

      {/* Add model form */}
      {!showForm ? (
        <button
          className="settings-key-item__btn"
          style={{ marginTop: 12, display: 'flex', alignItems: 'center', gap: 6 }}
          onClick={() => { handleProviderChange('groq'); setShowForm(true); }}
        >
          <Plus size={14} />
          {t('settings.addModel')}
        </button>
      ) : (
        <div className="settings-model-card" style={{ flexDirection: 'column', alignItems: 'stretch', gap: 10, marginTop: 12 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div className="settings-model-card__name">{t('settings.addModel')}</div>
            <button className="settings-model-card__test-btn" onClick={resetForm}><X size={14} /></button>
          </div>

          {/* Provider selector */}
          <div className="settings-key-item">
            <span className="settings-key-item__label">{t('settings.provider')}</span>
            <select
              className="settings-key-item__input"
              value={formProvider}
              onChange={(e) => handleProviderChange(e.target.value as AiProviderType)}
            >
              {PROVIDERS.map(p => (
                <option key={p.id} value={p.id}>{p.label}</option>
              ))}
            </select>
          </div>

          {/* Display name */}
          <div className="settings-key-item">
            <span className="settings-key-item__label">{t('settings.modelDisplayName')}</span>
            <input
              className="settings-key-item__input"
              value={formName}
              onChange={(e) => setFormName(e.target.value)}
              placeholder={providerDef.label}
            />
          </div>

          {/* API Key */}
          {providerDef.needsKey && (
            <div className="settings-key-item">
              <span className="settings-key-item__label">API Key</span>
              <input
                className="settings-key-item__input"
                type="password"
                value={formKey}
                onChange={(e) => setFormKey(e.target.value)}
                placeholder="sk-..."
              />
            </div>
          )}

          {/* Model name */}
          <div className="settings-key-item">
            <span className="settings-key-item__label">{t('settings.modelName')}</span>
            <input
              className="settings-key-item__input"
              value={formModel}
              onChange={(e) => setFormModel(e.target.value)}
              placeholder={providerDef.defaultModel}
            />
          </div>

          {/* Endpoint URL */}
          {providerDef.needsEndpoint && (
            <div className="settings-key-item">
              <span className="settings-key-item__label">{t('settings.endpoint')}</span>
              <input
                className="settings-key-item__input"
                value={formEndpoint}
                onChange={(e) => setFormEndpoint(e.target.value)}
                placeholder={providerDef.defaultEndpoint || 'https://...'}
              />
            </div>
          )}

          {/* Save button */}
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8 }}>
            <button className="settings-key-item__btn" onClick={resetForm}>{t('settings.cancel')}</button>
            <button
              className="settings-key-item__btn settings-key-item__btn--save"
              disabled={saving || (providerDef.needsKey && !formKey)}
              onClick={() => void handleSave()}
            >
              {saving ? t('settings.saving') : t('settings.saveKey')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
