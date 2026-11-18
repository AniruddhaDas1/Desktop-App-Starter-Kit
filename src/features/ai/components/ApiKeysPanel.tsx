import { useEffect, useState } from 'react';
import { AlertTriangle } from 'lucide-react';
import { Panel, Button, Badge, Select, TextInput } from '@shared/components';
import { Icon } from '@renderer/components/Icon';
import { useApiKey } from '@renderer/hooks/useApiKey';
import { useSecretStatus } from '@renderer/hooks/useSecretStatus';
import { useSetting } from '@renderer/hooks/useSetting';
import { useToast } from '@renderer/contexts/ToastContext';
import { AI_PROVIDERS, getProvider } from '@shared/ai/models';

export function ApiKeysPanel() {
  const [provider, setProvider] = useSetting('aiProvider');
  const [model, setModel] = useSetting('aiModel');
  const [baseUrl, setBaseUrl] = useSetting('aiBaseUrl');
  const info = getProvider(provider);

  const secretStatus = useSecretStatus();
  const { state, save, remove } = useApiKey(info.keyName);
  const { showToast } = useToast();

  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [modelDraft, setModelDraft] = useState(model);
  const [baseDraft, setBaseDraft] = useState(baseUrl);

  useEffect(() => {
    setEditing(false);
    setValue('');
  }, [provider]);

  useEffect(() => setModelDraft(model), [model]);
  useEffect(() => setBaseDraft(baseUrl), [baseUrl]);

  // The OS keyring has no backend on some Linux desktops (no gnome-keyring/kwallet).
  const showNoEncryptionBanner = secretStatus !== null && !secretStatus.encryptionAvailable;

  const reset = () => {
    setEditing(false);
    setValue('');
  };

  const handleProviderChange = async (next: string) => {
    const nextInfo = getProvider(next);
    try {
      await setProvider(nextInfo.id);
      if (nextInfo.defaultModel !== model) await setModel(nextInfo.defaultModel);
    } catch (err) {
      showToast(`Could not switch provider: ${String(err)}`, 'error');
    }
  };

  const handleSave = async () => {
    if (submitting) return;
    setSubmitting(true);
    try {
      const res = await save(value);
      if (!res.success) {
        showToast(`Could not save the key: ${res.error ?? 'unknown error'}`, 'error');
        return;
      }
      showToast(`${info.keyLabel} saved`, 'success');
      reset();
    } finally {
      setSubmitting(false);
    }
  };

  const handleRemove = async () => {
    const res = await remove();
    showToast(
      res.success ? `${info.keyLabel} removed` : `Could not remove the key: ${res.error ?? 'unknown'}`,
      res.success ? 'success' : 'error',
    );
  };

  const commitModel = async () => {
    const next = modelDraft.trim() || info.defaultModel;
    setModelDraft(next);
    if (next === model) return;
    try {
      await setModel(next);
    } catch (err) {
      showToast(`Could not save the model: ${String(err)}`, 'error');
    }
  };

  const commitBaseUrl = async () => {
    const next = baseDraft.trim();
    if (!next) {
      setBaseDraft(baseUrl);
      return;
    }

    try {
      const url = new URL(next);
      if (url.protocol !== 'http:' && url.protocol !== 'https:') throw new Error('protocol');
    } catch {
      showToast('Base URL must be a valid http(s) URL', 'error');
      setBaseDraft(baseUrl);
      return;
    }

    if (next === baseUrl) return;
    try {
      await setBaseUrl(next);
    } catch (err) {
      showToast(`Could not save the base URL: ${String(err)}`, 'error');
    }
  };

  const showInput = state.kind === 'absent' || editing;
  const showStored = state.kind === 'present' && !editing;

  return (
    <Panel header="AI provider">
      <div className="p-3 space-y-4">
        {showNoEncryptionBanner && (
          <div
            role="alert"
            className="flex items-start gap-2 rounded p-3"
            style={{
              border: '1px solid var(--color-warning, #b45309)',
              background: 'color-mix(in srgb, var(--color-warning, #b45309) 12%, transparent)',
            }}
          >
            <Icon icon={AlertTriangle} size={16} className="text-warning shrink-0 mt-[2px]" />
            <p className="text-[11px] text-text-secondary">
              This system has no keyring, so keys are stored without OS encryption.
            </p>
          </div>
        )}

        <div className="flex items-center justify-between gap-3">
          <span className="text-[12px] text-text-secondary font-medium">Provider</span>
          <Select
            value={provider}
            onValueChange={(next) => void handleProviderChange(next)}
            options={AI_PROVIDERS.map((option) => ({ value: option.id, label: option.label }))}
            aria-label="AI provider"
            className="w-[300px]"
          />
        </div>

        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-[12px] text-text-secondary font-medium">{info.keyLabel}</span>
              {state.kind === 'present' && <Badge variant="free">Saved</Badge>}
              {state.kind === 'absent' && <Badge variant="pro">Not set</Badge>}
            </div>
            <button
              onClick={() => void window.api.appOpenExternal({ url: info.keyUrl })}
              className="text-[10px] text-accent-light hover:underline cursor-pointer"
            >
              {info.keyUrlLabel}
            </button>
          </div>

          {state.kind === 'loading' && <p className="text-[11px] text-text-dim">Loading…</p>}

          {showStored && (
            <div className="flex items-center gap-2">
              <code className="flex-1 text-[11px] font-mono text-text-muted px-2 py-1 bg-app-deep rounded">
                {state.preview}
              </code>
              <Button variant="secondary" onClick={() => setEditing(true)}>
                Replace
              </Button>
              <Button variant="secondary" onClick={() => void handleRemove()}>
                Remove
              </Button>
            </div>
          )}

          {showInput && (
            <div className="flex items-center gap-2">
              <TextInput
                type="password"
                value={value}
                onChange={(e) => setValue(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !submitting && value.trim()) void handleSave();
                  if (e.key === 'Escape' && editing) reset();
                }}
                placeholder={info.keyPlaceholder}
                disabled={submitting}
                className="flex-1"
                autoComplete="off"
                spellCheck={false}
                aria-label={info.keyLabel}
              />
              <Button
                variant="primary"
                onClick={() => void handleSave()}
                disabled={submitting || !value.trim()}
              >
                {submitting ? 'Saving…' : 'Save'}
              </Button>
              {editing && (
                <Button variant="secondary" onClick={reset} disabled={submitting}>
                  Cancel
                </Button>
              )}
            </div>
          )}

          {info.keyHint && <p className="text-[10px] text-text-dim">{info.keyHint}</p>}
        </div>

        <div className="flex items-center justify-between gap-3">
          <span className="text-[12px] text-text-secondary font-medium">Model</span>
          {info.usesBaseUrl ? (
            <TextInput
              value={modelDraft}
              onChange={(e) => setModelDraft(e.currentTarget.value)}
              onBlur={() => void commitModel()}
              onKeyDown={(e) => {
                if (e.key === 'Enter') e.currentTarget.blur();
                if (e.key === 'Escape') setModelDraft(model);
              }}
              placeholder={info.defaultModel}
              className="w-[300px]"
              autoComplete="off"
              spellCheck={false}
              aria-label="Model id"
            />
          ) : (
            <Select
              value={model}
              onValueChange={(next) => void setModel(next)}
              options={info.models.map((option) => ({
                value: option.id,
                label:
                  option.inputPerMTok !== undefined && option.outputPerMTok !== undefined
                    ? `${option.label} · $${option.inputPerMTok} / $${option.outputPerMTok} per M`
                    : option.label,
              }))}
              aria-label="Model"
              className="w-[300px]"
            />
          )}
        </div>

        {info.usesBaseUrl && (
          <div className="space-y-1">
            <div className="flex items-center justify-between gap-3">
              <span className="text-[12px] text-text-secondary font-medium">Base URL</span>
              <TextInput
                value={baseDraft}
                onChange={(e) => setBaseDraft(e.currentTarget.value)}
                onBlur={() => void commitBaseUrl()}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') e.currentTarget.blur();
                  if (e.key === 'Escape') setBaseDraft(baseUrl);
                }}
                placeholder="https://api.openai.com/v1"
                className="w-[300px]"
                autoComplete="off"
                spellCheck={false}
                aria-label="Base URL"
              />
            </div>
            <p className="text-[10px] text-text-dim">
              API root, e.g. https://openrouter.ai/api/v1 or http://localhost:11434/v1 for Ollama.
              Plain http is allowed only for localhost.
            </p>
          </div>
        )}

        <p className="text-[10px] text-text-dim">
          Keys are encrypted by your OS and stay in the Rust backend. This screen only ever sees a
          masked version, and a saved key is sent only to the endpoint selected above.
        </p>
      </div>
    </Panel>
  );
}
