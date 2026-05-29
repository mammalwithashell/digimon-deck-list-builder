import { beforeEach, describe, expect, it } from 'vitest';
import { DEFAULT_PRESET, RESOLUTION_PRESETS } from '@/utils/constants';
import { useUiStore, __uiStoreInternals } from './uiStore';

const PRESET_KEY = __uiStoreInternals.PRESET_STORAGE_KEY;
const FULLSCREEN_KEY = __uiStoreInternals.FULLSCREEN_STORAGE_KEY;

/**
 * The store initializes once at module load time. To test the
 * hydration logic for different localStorage states we call the
 * exported `loadPersisted*` helpers directly and re-seed the store
 * state. This is preferable to re-importing the module under a fake
 * query path (which esbuild rejects with an "Invalid loader value"
 * error).
 */
function rehydrate() {
  useUiStore.setState({
    graphicsPreset: __uiStoreInternals.loadPersistedPreset(),
    fullscreen: __uiStoreInternals.loadPersistedFullscreen(),
  });
}

describe('uiStore graphics state', () => {
  beforeEach(() => {
    window.localStorage.clear();
    // Reset to a known baseline before each test.
    useUiStore.setState({
      graphicsPreset: DEFAULT_PRESET,
      fullscreen: false,
    });
  });

  it('hydrates to the default preset when localStorage is empty', () => {
    rehydrate();
    expect(useUiStore.getState().graphicsPreset).toEqual(DEFAULT_PRESET);
    expect(useUiStore.getState().fullscreen).toBe(false);
  });

  it('hydrates a persisted preset that matches a known DCGO size', () => {
    const want = RESOLUTION_PRESETS[4]!; // 2560x1440
    window.localStorage.setItem(PRESET_KEY, JSON.stringify(want));
    window.localStorage.setItem(FULLSCREEN_KEY, 'true');
    rehydrate();
    expect(useUiStore.getState().graphicsPreset).toEqual(want);
    expect(useUiStore.getState().fullscreen).toBe(true);
  });

  it('falls back to default when a non-preset size is persisted', () => {
    // 999x1234 isn't in RESOLUTION_PRESETS — should snap back.
    window.localStorage.setItem(
      PRESET_KEY,
      JSON.stringify({ width: 999, height: 1234 }),
    );
    rehydrate();
    expect(useUiStore.getState().graphicsPreset).toEqual(DEFAULT_PRESET);
  });

  it('persists preset selection to localStorage on setGraphicsPreset', () => {
    const preset = RESOLUTION_PRESETS[3]!; // 1920x1080
    useUiStore.getState().setGraphicsPreset(preset);
    expect(useUiStore.getState().graphicsPreset).toEqual(preset);
    expect(JSON.parse(window.localStorage.getItem(PRESET_KEY)!)).toEqual(preset);
  });

  it('persists fullscreen flag to localStorage on setFullscreen', () => {
    useUiStore.getState().setFullscreen(true);
    expect(useUiStore.getState().fullscreen).toBe(true);
    expect(window.localStorage.getItem(FULLSCREEN_KEY)).toBe('true');

    useUiStore.getState().setFullscreen(false);
    expect(window.localStorage.getItem(FULLSCREEN_KEY)).toBe('false');
  });

  it('returns default on malformed JSON in localStorage', () => {
    window.localStorage.setItem(PRESET_KEY, 'not json');
    rehydrate();
    expect(useUiStore.getState().graphicsPreset).toEqual(DEFAULT_PRESET);
  });
});
