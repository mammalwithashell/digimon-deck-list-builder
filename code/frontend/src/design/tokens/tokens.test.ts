import { describe, it, expect } from 'vitest';
// @ts-expect-error @types/node is intentionally absent from the frontend tsconfig
// (browser target); `fs` is available in the vitest Node runtime. CSS file
// contents aren't exposed via Vite's `?raw` under vitest, so we read from disk.
import { readFileSync } from 'node:fs';

/**
 * Token-completeness guardrail (add-desktop-design-system): every role token
 * defined in the dark theme block MUST also be defined in the light block, and
 * vice versa, so no token resolves empty when a theme is active. The path is
 * relative to the package root, where `npm test` runs vitest.
 */
const tokensCss: string = readFileSync('src/design/tokens/tokens.css', 'utf8');

function rolesBetween(start: string, end: string): string[] {
  const s = tokensCss.indexOf(start);
  const e = tokensCss.indexOf(end);
  if (s === -1 || e === -1) {
    throw new Error(`tokens.css sentinel markers not found: ${start} / ${end}`);
  }
  const block = tokensCss.slice(s + start.length, e);
  // Match declared custom-property names (followed by a colon), not var() refs.
  const names: string[] = block.match(/--[\w-]+(?=\s*:)/g) ?? [];
  return [...new Set<string>(names)].sort();
}

const dark = rolesBetween('/* @tokens:dark:start */', '/* @tokens:dark:end */');
const light = rolesBetween('/* @tokens:light:start */', '/* @tokens:light:end */');

describe('design tokens', () => {
  it('defines the core role tokens', () => {
    for (const role of ['--surface-bg', '--ink-0', '--accent', '--player', '--opp', '--font-display']) {
      expect(dark).toContain(role);
    }
  });

  it('light theme defines every role the dark theme defines', () => {
    expect(dark.filter((r) => !light.includes(r))).toEqual([]);
  });

  it('dark theme defines every role the light theme defines', () => {
    expect(light.filter((r) => !dark.includes(r))).toEqual([]);
  });
});
