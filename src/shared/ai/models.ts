// AI provider catalogue offered in the AI screen. Model ids are the exact API
// strings; prices are USD per million tokens and only used for the cost readout.
// OpenAI-compatible endpoints accept any model id, so their list is a set of
// known presets — the AI screen lets the user type a different one.

export const AI_PROVIDER_IDS = ['anthropic', 'openai', 'google'] as const;

export type AiProvider = (typeof AI_PROVIDER_IDS)[number];

export interface AiModel {
  id: string;
  label: string;
  inputPerMTok?: number;
  outputPerMTok?: number;
  /** Anthropic only: accepts output_config.effort. */
  supportsEffort?: boolean;
  /** Anthropic only: accepts server-side refusal fallbacks. */
  supportsFallback?: boolean;
}

export interface AiProviderInfo {
  id: AiProvider;
  label: string;
  /** Short name used on buttons, e.g. "Ask Claude". */
  shortLabel: string;
  /** `useApiKey()` name; the secret is stored as `apikey_${keyName}`. */
  keyName: string;
  keyLabel: string;
  keyUrl: string;
  keyUrlLabel: string;
  keyPlaceholder: string;
  keyHint?: string;
  defaultModel: string;
  models: readonly AiModel[];
  /** Show the base URL field (OpenAI-compatible endpoints). */
  usesBaseUrl?: boolean;
}

export const ANTHROPIC_MODELS: readonly AiModel[] = [
  {
    id: 'claude-opus-5-5',
    label: 'Claude Opus 5.5',
    inputPerMTok: 4,
    outputPerMTok: 20,
    supportsEffort: true,
    supportsFallback: true,
  },
  {
    id: 'claude-sonnet-5-5',
    label: 'Claude Sonnet 5.5',
    inputPerMTok: 2,
    outputPerMTok: 10,
    supportsEffort: true,
    supportsFallback: true,
  },
  {
    id: 'claude-haiku-4-5',
    label: 'Claude Haiku 4.5',
    inputPerMTok: 1,
    outputPerMTok: 5,
    supportsEffort: false,
    supportsFallback: false,
  },
];

export const OPENAI_MODELS: readonly AiModel[] = [
  { id: 'gpt-4.1', label: 'GPT-4.1', inputPerMTok: 2, outputPerMTok: 8 },
  { id: 'gpt-4.1-mini', label: 'GPT-4.1 mini', inputPerMTok: 0.4, outputPerMTok: 1.6 },
  { id: 'o4-mini', label: 'o4-mini', inputPerMTok: 1.1, outputPerMTok: 4.4 },
];

export const GEMINI_MODELS: readonly AiModel[] = [
  { id: 'gemini-2.5-pro', label: 'Gemini 2.5 Pro', inputPerMTok: 1.25, outputPerMTok: 10 },
  { id: 'gemini-2.5-flash', label: 'Gemini 2.5 Flash', inputPerMTok: 0.3, outputPerMTok: 2.5 },
  {
    id: 'gemini-2.5-flash-lite',
    label: 'Gemini 2.5 Flash-Lite',
    inputPerMTok: 0.1,
    outputPerMTok: 0.4,
  },
];

const ANTHROPIC_PROVIDER: AiProviderInfo = {
  id: 'anthropic',
  label: 'Anthropic (Claude)',
  shortLabel: 'Claude',
  keyName: 'anthropic',
  keyLabel: 'Anthropic API key',
  keyUrl: 'https://console.anthropic.com/settings/keys',
  keyUrlLabel: 'Anthropic keys',
  keyPlaceholder: 'sk-ant-...',
  defaultModel: 'claude-opus-5-5',
  models: ANTHROPIC_MODELS,
};

const OPENAI_PROVIDER: AiProviderInfo = {
  id: 'openai',
  label: 'OpenAI-compatible',
  shortLabel: 'OpenAI',
  keyName: 'openai',
  keyLabel: 'API key',
  keyUrl: 'https://platform.openai.com/api-keys',
  keyUrlLabel: 'OpenAI keys',
  keyPlaceholder: 'sk-...',
  keyHint:
    'Works with OpenAI, OpenRouter, Groq, Ollama and LM Studio. A key is optional when the base URL points at localhost.',
  defaultModel: 'gpt-4.1',
  models: OPENAI_MODELS,
  usesBaseUrl: true,
};

const GOOGLE_PROVIDER: AiProviderInfo = {
  id: 'google',
  label: 'Google Gemini',
  shortLabel: 'Gemini',
  keyName: 'google',
  keyLabel: 'Gemini API key',
  keyUrl: 'https://aistudio.google.com/apikey',
  keyUrlLabel: 'Google AI Studio',
  keyPlaceholder: 'AIza...',
  defaultModel: 'gemini-2.5-flash',
  models: GEMINI_MODELS,
};

export const AI_PROVIDERS: readonly AiProviderInfo[] = [
  ANTHROPIC_PROVIDER,
  OPENAI_PROVIDER,
  GOOGLE_PROVIDER,
];

export const DEFAULT_PROVIDER: AiProvider = 'anthropic';
export const DEFAULT_MODEL = 'claude-opus-5-5';
export const DEFAULT_BASE_URL = 'https://api.openai.com/v1';

/** How hard the model thinks. Lower is faster and cheaper (Anthropic only). */
export type Effort = 'low' | 'medium' | 'high' | 'xhigh' | 'max';

export function getProvider(id: string): AiProviderInfo {
  return AI_PROVIDERS.find((provider) => provider.id === id) ?? ANTHROPIC_PROVIDER;
}

export function findModel(provider: string, modelId: string): AiModel | undefined {
  const models = getProvider(provider).models;
  const exact = models.find((model) => model.id === modelId);
  if (exact) return exact;

  // APIs may echo a dated id ("gpt-4.1-2025-04-14"); use the longest prefix match.
  return models
    .filter((model) => modelId.startsWith(model.id))
    .sort((left, right) => right.id.length - left.id.length)[0];
}

/** Rough USD cost of one call, or null when the model or price is unknown. */
export function estimateCostUsd(
  provider: string,
  modelId: string,
  inputTokens: number,
  outputTokens: number,
): number | null {
  const model = findModel(provider, modelId);
  if (!model || model.inputPerMTok === undefined || model.outputPerMTok === undefined) {
    return null;
  }
  return (inputTokens * model.inputPerMTok + outputTokens * model.outputPerMTok) / 1_000_000;
}
