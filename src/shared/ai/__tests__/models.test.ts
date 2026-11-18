import { describe, expect, it } from 'vitest';
import { AI_PROVIDERS, estimateCostUsd, findModel, getProvider } from '../models';

describe('AI provider catalogue', () => {
  it('offers a default model inside every provider catalogue', () => {
    for (const provider of AI_PROVIDERS) {
      expect(provider.models.some((model) => model.id === provider.defaultModel)).toBe(true);
    }
  });

  it('falls back to Anthropic for an unknown provider id', () => {
    expect(getProvider('copilot').id).toBe('anthropic');
  });

  it('matches dated API model ids by the longest prefix', () => {
    expect(findModel('openai', 'gpt-4.1-2025-04-14')?.id).toBe('gpt-4.1');
    expect(findModel('openai', 'gpt-4.1-mini-2025-04-14')?.id).toBe('gpt-4.1-mini');
    expect(findModel('google', 'unknown-model')).toBeUndefined();
  });

  it('estimates cost with the selected provider prices', () => {
    expect(estimateCostUsd('anthropic', 'claude-sonnet-5-5', 1_000_000, 0)).toBe(2);
    expect(estimateCostUsd('google', 'gemini-2.5-flash', 0, 1_000_000)).toBe(2.5);
    expect(estimateCostUsd('openai', 'custom-local-model', 10, 10)).toBeNull();
  });
});
