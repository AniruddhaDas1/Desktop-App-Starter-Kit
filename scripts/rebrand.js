import { readFileSync, writeFileSync } from 'node:fs';

const args = process.argv.slice(2);
const getArg = (name) => {
  const index = args.indexOf(`--${name}`);
  return index !== -1 ? args[index + 1] : undefined;
};

const name = getArg('name');
const identifier = getArg('id');

if (!name || !identifier) {
  console.error('Usage: npm run rebrand -- --name "My App" --id com.me.myapp');
  process.exit(1);
}

if (!/^[a-zA-Z0-9][a-zA-Z0-9.-]*$/.test(identifier) || !identifier.includes('.')) {
  console.error('The identifier must look like com.me.myapp');
  process.exit(1);
}

const slug = name
  .toLowerCase()
  .replace(/[^a-z0-9]+/g, '-')
  .replace(/^-|-$/g, '');

const readJson = (path) => JSON.parse(readFileSync(path, 'utf8'));
const writeJson = (path, value) => writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
const replaceInFile = (path, pattern, replacement) => {
  const before = readFileSync(path, 'utf8');
  const after = before.replace(pattern, replacement);
  if (after !== before) writeFileSync(path, after);
};

// package.json
const pkg = readJson('package.json');
pkg.name = `${slug}-tauri`;
pkg.productName = name;
writeJson('package.json', pkg);

// Tauri configs (window titles + identity)
for (const configPath of [
  'src-tauri/tauri.conf.json',
  'src-tauri/tauri.macos.conf.json',
  'src-tauri/tauri.windows.conf.json',
  'src-tauri/tauri.linux.conf.json',
]) {
  try {
    const config = readJson(configPath);
    if (configPath === 'src-tauri/tauri.conf.json') {
      config.productName = name;
      config.identifier = identifier;
    }
    if (config.app && Array.isArray(config.app.windows)) {
      config.app.windows = config.app.windows.map((window) => ({
        ...window,
        ...(window.label === 'main' ? { title: name } : {}),
      }));
    }
    writeJson(configPath, config);
  } catch {
    // A platform override may not exist yet.
  }
}

// The browser tab / loading screen title.
replaceInFile('index.html', /<title>.*?<\/title>/, `<title>${name}</title>`);
replaceInFile(
  'index.html',
  /<div class="app-name">.*?<\/div>/,
  `<div class="app-name">${name}</div>`,
);

console.log(`Renamed to "${name}" (${identifier}).`);
console.log('Next: npx tauri icon resources/icon.png');
console.log(
  'Note: the Rust crate name is unchanged on purpose — only the bundle identity matters.',
);
