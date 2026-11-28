import { describe, it, expect } from 'vitest';
// @ts-expect-error @types/node is intentionally absent from the frontend tsconfig
// (browser target); `fs` is available in the vitest Node runtime.
import { readFileSync } from 'node:fs';

/**
 * Token-completeness & WCAG readability guardrail across all 6
 * (polarity x stylePreset) token blocks in tokens.css.
 */
const tokensCss: string = readFileSync('src/design/tokens/tokens.css', 'utf8');

const BLOCKS = [
  { name: 'digi-os dark', start: '/* @tokens:dark:start */', end: '/* @tokens:dark:end */' },
  { name: 'digi-os light', start: '/* @tokens:light:start */', end: '/* @tokens:light:end */' },
  { name: 'adventure dark', start: '/* @tokens:adventure-dark:start */', end: '/* @tokens:adventure-dark:end */' },
  { name: 'adventure light', start: '/* @tokens:adventure-light:start */', end: '/* @tokens:adventure-light:end */' },
  { name: 'aegis dark', start: '/* @tokens:aegis-dark:start */', end: '/* @tokens:aegis-dark:end */' },
  { name: 'aegis light', start: '/* @tokens:aegis-light:start */', end: '/* @tokens:aegis-light:end */' },
] as const;

function extractBlock(start: string, end: string): string {
  const s = tokensCss.indexOf(start);
  const e = tokensCss.indexOf(end);
  if (s === -1 || e === -1) {
    throw new Error(`tokens.css sentinel markers not found: ${start} / ${end}`);
  }
  return tokensCss.slice(s + start.length, e);
}

function rolesOf(block: string): string[] {
  const names: string[] = block.match(/--[\w-]+(?=\s*:)/g) ?? [];
  return [...new Set<string>(names)].sort();
}

function hexValue(block: string, token: string): string {
  const re = new RegExp(`${token}\\s*:\\s*(#[0-9a-fA-F]{6})\\s*;`);
  const match = block.match(re);
  if (!match?.[1]) throw new Error(`Expected hex color for ${token}`);
  return match[1];
}

function luminance(hex: string): number {
  const channels = [1, 3, 5].map((i) => {
    const c = Number.parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return (channels[0] ?? 0) * 0.2126 + (channels[1] ?? 0) * 0.7152 + (channels[2] ?? 0) * 0.0722;
}

function contrastRatio(a: string, b: string): number {
  const l1 = luminance(a);
  const l2 = luminance(b);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}

const parsed = BLOCKS.map((b) => {
  const raw = extractBlock(b.start, b.end);
  return { ...b, raw, roles: rolesOf(raw) };
});

describe('design tokens', () => {
  const baseline = parsed[0]!.roles;

  it('defines core role and arena tokens', () => {
    for (const role of [
      '--surface-bg',
      '--surface',
      '--ink-0',
      '--accent',
      '--player',
      '--opp',
      '--font-display',
      '--arena-bg',
      '--arena-felt',
      '--arena-dais',
      '--arena-rim-ready',
    ]) {
      expect(baseline).toContain(role);
    }
  });

  for (const preset of parsed.slice(1)) {
    it(`${preset.name} defines the exact same role set as baseline`, () => {
      expect(baseline.filter((r) => !preset.roles.includes(r))).toEqual([]);
      expect(preset.roles.filter((r) => !baseline.includes(r))).toEqual([]);
    });
  }

  for (const preset of parsed) {
    it(`${preset.name} meets WCAG AA (>= 4.5:1) contrast on primary ink & accent`, () => {
      const surface = hexValue(preset.raw, '--surface');
      const surfaceBg = hexValue(preset.raw, '--surface-bg');
      const ink0 = hexValue(preset.raw, '--ink-0');
      const ink1 = hexValue(preset.raw, '--ink-1');
      const accent = hexValue(preset.raw, '--accent');
      const accentInk = hexValue(preset.raw, '--accent-ink');

      expect(contrastRatio(ink0, surface)).toBeGreaterThanOrEqual(4.5);
      expect(contrastRatio(ink0, surfaceBg)).toBeGreaterThanOrEqual(4.5);
      expect(contrastRatio(ink1, surface)).toBeGreaterThanOrEqual(4.5);
      expect(contrastRatio(accent, surface)).toBeGreaterThanOrEqual(4.5);
      expect(contrastRatio(accentInk, accent)).toBeGreaterThanOrEqual(4.5);
    });
  }
});
