import { existsSync } from 'node:fs';
import { resolve } from 'node:path';

const sanitize = (s) => String(s ?? '').trim();

const splitWords = (s) =>
  sanitize(s)
    .split(/[-_:\s]+|(?=[A-Z])/)
    .filter(Boolean)
    .map((w) => w.toLowerCase());

const pcase = (s) =>
  splitWords(s)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join('');

const ccase = (s) => {
  const p = pcase(s);
  return p.charAt(0).toLowerCase() + p.slice(1);
};

const kcase = (s) => splitWords(s).join('-');

const Ccase = (s) => splitWords(s).join('_').toUpperCase();

// Rust command + module names are snake_case.
const scase = (s) => splitWords(s).join('_');

// "word-counter" -> "Word Counter" (the Sidebar label)
const tcase = (s) =>
  splitWords(s)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');

export default function plopConfig(plop) {
  plop.setHelper('pcase', pcase);
  plop.setHelper('ccase', ccase);
  plop.setHelper('kcase', kcase);
  plop.setHelper('Ccase', Ccase);
  plop.setHelper('scase', scase);
  plop.setHelper('tcase', tcase);

  plop.setGenerator('feature', {
    description: 'Scaffold a new feature module + register it in screens-registry.ts',
    prompts: [
      {
        type: 'input',
        name: 'name',
        message: 'Feature name (PascalCase, e.g. MyFeature):',
        validate: (value) =>
          /^[A-Za-z][A-Za-z0-9 _-]*$/.test(sanitize(value)) ||
          'Use letters/digits, start with a letter (e.g., MyFeature, my-feature, my_feature).',
      },
    ],
    actions: [
      {
        type: 'add',
        path: 'src/features/{{kcase name}}/components/{{pcase name}}Screen.tsx',
        templateFile: 'scripts/templates/feature/Screen.tsx.hbs',
      },
      {
        type: 'add',
        path: 'src/features/{{kcase name}}/types.ts',
        templateFile: 'scripts/templates/feature/types.ts.hbs',
      },
      // The Sidebar and App routing both read screens-registry.ts.
      {
        type: 'modify',
        path: 'src/renderer/screens-registry.ts',
        pattern: /(\r?\n\r?\nexport type ScreenSection)/,
        template:
          "\nimport { {{pcase name}}Screen } from '@features/{{kcase name}}/components/{{pcase name}}Screen';$1",
      },
      // New screens go in the main section, just above Settings.
      {
        type: 'modify',
        path: 'src/renderer/screens-registry.ts',
        pattern: /(\r?\n {2}\{\s*id: 'settings',)/,
        template:
          "\n  {\n    id: '{{kcase name}}',\n    label: '{{tcase name}}',\n    icon: Sparkles,\n    section: 'main',\n    Component: {{pcase name}}Screen,\n  },$1",
      },
    ],
  });

  plop.setGenerator('command', {
    description:
      'Wire a new Tauri command across the 3 places (lowercase domain:action, e.g. demo:ping)',
    prompts: [
      {
        type: 'input',
        name: 'channel',
        message: 'Channel name (lowercase domain:action, e.g. demo:ping):',
        validate: (value) =>
          /^[a-z][a-z0-9-]*:[a-z][a-z0-9-]*$/.test(sanitize(value)) ||
          'Use lowercase domain:action format (e.g., demo:ping).',
      },
    ],
    actions: (data) => {
      const channel = sanitize(data.channel);
      const [domain, action] = channel.split(':');
      data.domain = domain;
      data.action = action;

      const commandPath = resolve(`src-tauri/src/commands/${domain}.rs`);

      if (existsSync(commandPath)) {
        throw new Error(
          `${commandPath} already exists. Pick a different domain or wire the command manually.`,
        );
      }

      return [
        // 1. The Rust command.
        {
          type: 'add',
          path: 'src-tauri/src/commands/{{domain}}.rs',
          templateFile: 'scripts/templates/command/command.rs.hbs',
        },

        // 2. Register it: module list + generate_handler![].
        {
          type: 'modify',
          path: 'src-tauri/src/commands/mod.rs',
          pattern: /(\r?\npub mod window;)/,
          template: '\npub mod {{domain}};$1',
        },
        {
          type: 'modify',
          path: 'src-tauri/src/lib.rs',
          pattern: /(use state::AppState;)/,
          template: "use commands::{{domain}}::{{scase action}};\n$1",
        },
        {
          type: 'modify',
          path: 'src-tauri/src/lib.rs',
          pattern: /(\r?\n {8}\]\r?\n {8}\.run\()/,
          template: '\n            {{scase action}},$1',
        },

        // 3. The typed bridge surface.
        {
          type: 'append',
          path: 'src/shared/ipc/types.ts',
          template:
            '\n// {{channel}}\nexport interface {{pcase action}}Request {\n  value: string;\n}\n\nexport interface {{pcase action}}Response {\n  success: boolean;\n  echo: string;\n}\n',
        },
        {
          type: 'modify',
          path: 'src/lib/bridge.ts',
          pattern: /(import type \{ DndBlessAndResolveResponse, DndResolvedPath \} from '@shared\/ipc\/types';)/,
          template:
            "$1\nimport type { {{pcase action}}Request, {{pcase action}}Response } from '@shared/ipc/types';",
        },
        {
          type: 'modify',
          path: 'src/lib/bridge.ts',
          pattern: /(\r?\n {2}fsStat: \(data\) => invoke\('fs_stat', \{ data \}\),)/,
          template:
            "$1\n\n  {{ccase action}}: (data: {{pcase action}}Request) =>\n    invoke<{{pcase action}}Response>('{{scase action}}', { data }),",
        },
        {
          type: 'modify',
          path: 'src/shared/ipc/preload-api.ts',
          pattern: /(\} from '\.\/types';)/,
          template:
            "$1\nimport type { {{pcase action}}Request, {{pcase action}}Response } from './types';",
        },
        {
          type: 'modify',
          path: 'src/shared/ipc/preload-api.ts',
          pattern: /(\r?\n {2}fsStat: \(data: FsStatRequest\) => Promise<FsStatResponse>;)/,
          template:
            '$1\n  {{ccase action}}: (data: {{pcase action}}Request) => Promise<{{pcase action}}Response>;',
        },
      ];
    },
  });
}
