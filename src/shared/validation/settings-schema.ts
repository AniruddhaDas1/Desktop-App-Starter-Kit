import { z } from 'zod';
import {
  AI_PROVIDER_IDS,
  DEFAULT_BASE_URL,
  DEFAULT_MODEL,
  DEFAULT_PROVIDER,
} from '@shared/ai/models';

export const themeSchema = z.enum(['system', 'light', 'dark', 'slate', 'midnight', 'hc']);

// Free-text user-facing display name. Bounded so UI that renders it in
// chrome (sidebar, title bar, greeting) never has to guard against huge strings.
export const displayNameSchema = z
  .string()
  .min(1, 'Display name is required')
  .max(40, 'Display name must be 40 characters or fewer');

// UI density. Compact rescales the Tailwind 4 --spacing token via a
// [data-density="compact"] block in global.css, which propagates to every
// p-*/m-*/gap-* utility. Default 'comfortable' matches the original layout.
export const densitySchema = z.enum(['comfortable', 'compact']);

export const aiProviderSchema = z.enum(AI_PROVIDER_IDS);

// Model used when a call does not pass one. Any id string is accepted so new
// models and OpenAI-compatible endpoints work without a code change; the AI
// screen offers the current presets from src/shared/ai/models.ts.
export const aiModelSchema = z
  .string()
  .min(1)
  .max(128)
  .regex(/^[A-Za-z0-9_./:-]+$/, 'model contains invalid characters');

export const aiBaseUrlSchema = z
  .string()
  .min(1, 'Base URL is required')
  .max(300)
  .refine((value) => {
    try {
      const url = new URL(value);
      return url.protocol === 'http:' || url.protocol === 'https:';
    } catch {
      return false;
    }
  }, 'Enter a valid http(s) URL');

export const appSettingsSchema = z.object({
  theme: themeSchema,
  displayName: displayNameSchema,
  density: densitySchema,
  aiProvider: aiProviderSchema,
  aiModel: aiModelSchema,
  aiBaseUrl: aiBaseUrlSchema,
});

export type AppSettings = z.infer<typeof appSettingsSchema>;
export type AppSettingKey = keyof AppSettings;

export const APP_SETTINGS_DEFAULTS: AppSettings = {
  theme: 'system',
  displayName: 'User',
  density: 'comfortable',
  aiProvider: DEFAULT_PROVIDER,
  aiModel: DEFAULT_MODEL,
  aiBaseUrl: DEFAULT_BASE_URL,
};

const settingValueSchemas = {
  theme: themeSchema,
  displayName: displayNameSchema,
  density: densitySchema,
  aiProvider: aiProviderSchema,
  aiModel: aiModelSchema,
  aiBaseUrl: aiBaseUrlSchema,
} as const satisfies { [K in AppSettingKey]: z.ZodType<AppSettings[K]> };

export const settingsGetRequestSchema = z.object({
  key: appSettingsSchema.keyof(),
});

export const settingsSetRequestSchema = z
  .object({
    key: appSettingsSchema.keyof(),
    value: z.unknown(),
  })
  .superRefine((data, ctx) => {
    const schema = settingValueSchemas[data.key as AppSettingKey];
    const result = schema.safeParse(data.value);
    if (!result.success) {
      for (const issue of result.error.issues) {
        ctx.addIssue({ ...issue, path: ['value', ...issue.path] });
      }
    }
  });
