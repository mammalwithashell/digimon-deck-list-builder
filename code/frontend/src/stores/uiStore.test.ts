import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { DEFAULT_PRESET, RESOLUTION_PRESETS } from '@/utils/constants';
import {
  useUiStore,
  __uiStoreInternals,
  selectEffectiveLiveBackground,
} from './uiStore';

/** jsdom doesn't implement matchMedia; stub it so the OS reduced-motion
 *  derivation is deterministic. */
function mockMatchMedia(matches: boolean): void {
  window.matchMedia = ((query: string) => ({
    matches,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  })) as unknown as typeof window.matchMedia;
}

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

const DECK_BUILDER_VIEW_KEY = __uiStoreInternals.DECK_BUILDER_VIEW_STORAGE_KEY;

describe('uiStore deck builder view preference', () => {
  beforeEach(() => {
    window.localStorage.clear();
    useUiStore.setState({ deckBuilderView: 'browse' });
  });

  it('defaults to browse when localStorage is empty', () => {
    expect(__uiStoreInternals.loadPersistedDeckBuilderView()).toBe('browse');
  });

  it('hydrates a valid persisted view', () => {
    window.localStorage.setItem(DECK_BUILDER_VIEW_KEY, 'inspect');
    expect(__uiStoreInternals.loadPersistedDeckBuilderView()).toBe('inspect');
  });

  it('falls back to browse on a stale/invalid persisted value', () => {
    window.localStorage.setItem(DECK_BUILDER_VIEW_KEY, 'zoomed-out');
    expect(__uiStoreInternals.loadPersistedDeckBuilderView()).toBe('browse');
  });

  it('persists the view on setDeckBuilderView', () => {
    useUiStore.getState().setDeckBuilderView('inspect');
    expect(useUiStore.getState().deckBuilderView).toBe('inspect');
    expect(window.localStorage.getItem(DECK_BUILDER_VIEW_KEY)).toBe('inspect');
  });

  it('hydrates the decklist view', () => {
    window.localStorage.setItem(DECK_BUILDER_VIEW_KEY, 'decklist');
    expect(__uiStoreInternals.loadPersistedDeckBuilderView()).toBe('decklist');
  });

  it('persists the decklist view on setDeckBuilderView', () => {
    useUiStore.getState().setDeckBuilderView('decklist');
    expect(useUiStore.getState().deckBuilderView).toBe('decklist');
    expect(window.localStorage.getItem(DECK_BUILDER_VIEW_KEY)).toBe('decklist');
  });

  it('cycles browse -> inspect -> decklist -> browse and persists each flip', () => {
    const order = ['inspect', 'decklist', 'browse'];
    for (const expected of order) {
      useUiStore.getState().toggleDeckBuilderView();
      expect(useUiStore.getState().deckBuilderView).toBe(expected);
      expect(window.localStorage.getItem(DECK_BUILDER_VIEW_KEY)).toBe(expected);
    }
  });
});

const MOTION_KEY = __uiStoreInternals.MOTION_STORAGE_KEY;
const LIVE_BG_KEY = __uiStoreInternals.LIVE_BG_STORAGE_KEY;

describe('uiStore motion preference', () => {
  const realMatchMedia = window.matchMedia;
  beforeEach(() => {
    window.localStorage.clear();
  });
  afterEach(() => {
    window.matchMedia = realMatchMedia;
  });

  it('derives reduced when the OS requests reduced motion', () => {
    mockMatchMedia(true);
    expect(__uiStoreInternals.deriveDefaultMotion()).toBe('reduced');
  });

  it('derives full when the OS does not request reduced motion', () => {
    mockMatchMedia(false);
    expect(__uiStoreInternals.deriveDefaultMotion()).toBe('full');
  });

  it('uses the OS default when nothing is persisted', () => {
    mockMatchMedia(true);
    expect(__uiStoreInternals.loadPersistedMotion()).toBe('reduced');
  });

  it('hydrates a valid persisted motion level', () => {
    mockMatchMedia(false);
    window.localStorage.setItem(MOTION_KEY, 'off');
    expect(__uiStoreInternals.loadPersistedMotion()).toBe('off');
  });

  it('falls back to the OS default on an invalid persisted value', () => {
    mockMatchMedia(true);
    window.localStorage.setItem(MOTION_KEY, 'sideways');
    expect(__uiStoreInternals.loadPersistedMotion()).toBe('reduced');
  });

  it('persists motion and projects the data-motion attribute on setMotion', () => {
    useUiStore.getState().setMotion('reduced');
    expect(useUiStore.getState().motion).toBe('reduced');
    expect(window.localStorage.getItem(MOTION_KEY)).toBe('reduced');
    expect(document.documentElement.dataset.motion).toBe('reduced');
  });
});

describe('uiStore live background preference', () => {
  beforeEach(() => {
    window.localStorage.clear();
    useUiStore.setState({ liveBackground: true, motion: 'full' });
  });

  it('defaults to on when nothing is persisted', () => {
    expect(__uiStoreInternals.loadPersistedLiveBackground()).toBe(true);
  });

  it('hydrates a persisted off value', () => {
    window.localStorage.setItem(LIVE_BG_KEY, 'false');
    expect(__uiStoreInternals.loadPersistedLiveBackground()).toBe(false);
  });

  it('persists on setLiveBackground', () => {
    useUiStore.getState().setLiveBackground(false);
    expect(useUiStore.getState().liveBackground).toBe(false);
    expect(window.localStorage.getItem(LIVE_BG_KEY)).toBe('false');
  });
});

describe('selectEffectiveLiveBackground', () => {
  it('is true only when motion is full and the toggle is on', () => {
    useUiStore.setState({ motion: 'full', liveBackground: true });
    expect(selectEffectiveLiveBackground(useUiStore.getState())).toBe(true);
    useUiStore.setState({ motion: 'reduced', liveBackground: true });
    expect(selectEffectiveLiveBackground(useUiStore.getState())).toBe(false);
    useUiStore.setState({ motion: 'off', liveBackground: true });
    expect(selectEffectiveLiveBackground(useUiStore.getState())).toBe(false);
    useUiStore.setState({ motion: 'full', liveBackground: false });
    expect(selectEffectiveLiveBackground(useUiStore.getState())).toBe(false);
  });
});
