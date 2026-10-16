// Bumps the patch version everywhere the app version is recorded.
// Run from the repo root. Writes `version=<new>` to $GITHUB_OUTPUT when set.

import { readFileSync, writeFileSync, appendFileSync } from 'node:fs';

const current = JSON.parse(readFileSync('apps/desktop/package.json', 'utf8')).version;
const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(current);
if (!match) {
  throw new Error(`Unsupported version in apps/desktop/package.json: ${current}`);
}
const next = `${match[1]}.${match[2]}.${Number(match[3]) + 1}`;

// Regex edits keep each file's existing formatting intact.
const regexEdits = [
  ['package.json', /^(  "version": ")[^"]+(")/m],
  ['apps/desktop/package.json', /^(  "version": ")[^"]+(")/m],
  ['apps/desktop/src-tauri/tauri.conf.json', /^(  "version": ")[^"]+(")/m],
  ['apps/desktop/src-tauri/Cargo.toml', /^(version = ")[^"]+(")/m],
  ['apps/desktop/src-tauri/Cargo.lock', /^(name = "ferrocodex"\r?\nversion = ")[^"]+(")/m],
];

for (const [file, pattern] of regexEdits) {
  const text = readFileSync(file, 'utf8');
  if (!pattern.test(text)) {
    throw new Error(`No version field found in ${file}`);
  }
  writeFileSync(file, text.replace(pattern, `$1${next}$2`));
}

// Lockfiles are machine-formatted JSON, so a parse/stringify round trip is lossless.
const lockEdits = [
  ['package-lock.json', ['', 'apps/desktop']],
  ['apps/desktop/package-lock.json', ['']],
];

for (const [file, packageKeys] of lockEdits) {
  const lock = JSON.parse(readFileSync(file, 'utf8'));
  lock.version = next;
  for (const key of packageKeys) {
    if (!lock.packages?.[key]) {
      throw new Error(`Missing packages["${key}"] in ${file}`);
    }
    lock.packages[key].version = next;
  }
  writeFileSync(file, JSON.stringify(lock, null, 2) + '\n');
}

console.log(`Bumped version ${current} -> ${next}`);
if (process.env.GITHUB_OUTPUT) {
  appendFileSync(process.env.GITHUB_OUTPUT, `version=${next}\n`);
}
