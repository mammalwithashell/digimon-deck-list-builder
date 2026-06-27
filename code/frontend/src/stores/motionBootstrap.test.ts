import { describe, expect, it } from 'vitest';
import indexHtml from '../../index.html?raw';
import { __uiStoreInternals } from './uiStore';

/**
 * Parity guard for the pre-paint motion bootstrap in index.html: the inline
 * script reads/writes localStorage with a key literal that MUST match the
 * store's MOTION_STORAGE_KEY, and it must set the data-motion attribute the
 * CSS gating keys on. (Mirrors the theme bootstrap contract.) The HTML is
 * imported as a raw string (Vite `?raw`) so the test needs no Node APIs.
 */
describe('index.html motion bootstrap parity', () => {
  it('references the same motion storage key as the store', () => {
    expect(indexHtml).toContain(`'${__uiStoreInternals.MOTION_STORAGE_KEY}'`);
  });

  it('sets the data-motion attribute pre-paint', () => {
    expect(indexHtml).toContain('data-motion');
  });
});
