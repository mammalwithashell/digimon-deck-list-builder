import { create } from 'zustand';

/**
 * Dual-polarity x 3-preset theme state.
 *
 * - `theme` (`'dark' | 'light'`) projects onto `<html data-theme="...">`
 * - `stylePreset` (`'aegis' | 'digi-os' | 'adventure'`) projects onto `<html data-style="...">`
 *
 * The pre-paint bootstrap in `index.html` sets both attributes before React mounts
 * so there is no flash of wrong palette.
 */
export type Theme = 'dark' | 'light';
export type StylePreset = 'aegis' | 'digi-os' | 'adventure';

const THEME_STORAGE_KEY = 'desktop.theme';
const STYLE_STORAGE_KEY = 'desktop.stylePreset';
const THEMES: Theme[] = ['dark', 'light'];
export const STYLE_PRESETS: readonly { id: StylePreset; label: string; description: string }[] = [
  { id: 'aegis', label: 'Aegis Clean', description: 'Crisp neutral slate & off-white' },
  { id: 'digi-os', label: 'Soft Digi-OS', description: 'Muted slate-pine & sage mist' },
  { id: 'adventure', label: 'Soft Adventure', description: 'Warm charcoal & tactile paper' },
] as const;

const DEFAULT_THEME: Theme = 'dark';
const DEFAULT_STYLE_PRESET: StylePreset = 'aegis';

export function loadPersistedTheme(): Theme {
  if (typeof window === 'undefined') return DEFAULT_THEME;
  try {
    const raw = window.localStorage.getItem(THEME_STORAGE_KEY);
    return THEMES.includes(raw as Theme) ? (raw as Theme) : DEFAULT_THEME;
  } catch {
    return DEFAULT_THEME;
  }
}

export function loadPersistedStylePreset(): StylePreset {
  if (typeof window === 'undefined') return DEFAULT_STYLE_PRESET;
  try {
    const raw = window.localStorage.getItem(STYLE_STORAGE_KEY);
    return STYLE_PRESETS.some((p) => p.id === raw)
      ? (raw as StylePreset)
      : DEFAULT_STYLE_PRESET;
  } catch {
    return DEFAULT_STYLE_PRESET;
  }
}

function persistTheme(value: Theme): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(THEME_STORAGE_KEY, value);
  } catch {
    // Best-effort persistence
  }
}

function persistStylePreset(value: StylePreset): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(STYLE_STORAGE_KEY, value);
  } catch {
    // Best-effort persistence
  }
}

/** Apply the theme polarity to the document root so CSS keyed on `[data-theme]` resolves. */
export function applyThemeAttribute(theme: Theme): void {
  if (typeof document === 'undefined') return;
  document.documentElement.dataset.theme = theme;
}

/** Apply the style preset to the document root so CSS keyed on `[data-style]` resolves. */
export function applyStyleAttribute(stylePreset: StylePreset): void {
  if (typeof document === 'undefined') return;
  document.documentElement.dataset.style = stylePreset;
}

interface ThemeStore {
  theme: Theme;
  stylePreset: StylePreset;
  setTheme: (theme: Theme) => void;
  setStylePreset: (stylePreset: StylePreset) => void;
  toggle: () => void;
}

export const useThemeStore = create<ThemeStore>((set, get) => ({
  theme: loadPersistedTheme(),
  stylePreset: loadPersistedStylePreset(),
  setTheme: (theme) => {
    persistTheme(theme);
    applyThemeAttribute(theme);
    set({ theme });
  },
  setStylePreset: (stylePreset) => {
    persistStylePreset(stylePreset);
    applyStyleAttribute(stylePreset);
    set({ stylePreset });
  },
  toggle: () => {
    const next: Theme = get().theme === 'dark' ? 'light' : 'dark';
    persistTheme(next);
    applyThemeAttribute(next);
    set({ theme: next });
  },
}));

// Exported for tests and the pre-paint bootstrap parity check.
export const __themeStoreInternals = {
  THEME_STORAGE_KEY,
  STYLE_STORAGE_KEY,
  DEFAULT_THEME,
  DEFAULT_STYLE_PRESET,
  loadPersistedTheme,
  loadPersistedStylePreset,
};
