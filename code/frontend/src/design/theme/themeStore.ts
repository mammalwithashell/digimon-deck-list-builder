import { create } from 'zustand';

/**
 * Theme state for the dual-theme desktop design system
 * (add-desktop-design-system).
 *
 * The active theme is carried by the `data-theme` attribute on <html>; this
 * store is the source of that attribute and persists the choice. A pre-paint
 * bootstrap in index.html sets the attribute before React mounts (no
 * flash-of-wrong-theme); this store re-asserts it on every change.
 *
 * The storage key MUST match the literal used by the index.html bootstrap.
 */
export type Theme = 'dark' | 'light';

const THEME_STORAGE_KEY = 'desktop.theme';
const THEMES: Theme[] = ['dark', 'light'];
const DEFAULT_THEME: Theme = 'dark';

export function loadPersistedTheme(): Theme {
  if (typeof window === 'undefined') return DEFAULT_THEME;
  try {
    const raw = window.localStorage.getItem(THEME_STORAGE_KEY);
    return THEMES.includes(raw as Theme) ? (raw as Theme) : DEFAULT_THEME;
  } catch {
    return DEFAULT_THEME;
  }
}

function persistTheme(value: Theme): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(THEME_STORAGE_KEY, value);
  } catch {
    // Best-effort persistence — quota exceeded / private mode etc.
  }
}

/** Apply the theme to the document root so CSS keyed on `[data-theme]` resolves. */
export function applyThemeAttribute(theme: Theme): void {
  if (typeof document === 'undefined') return;
  document.documentElement.dataset.theme = theme;
}

interface ThemeStore {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggle: () => void;
}

export const useThemeStore = create<ThemeStore>((set, get) => ({
  theme: loadPersistedTheme(),
  setTheme: (theme) => {
    persistTheme(theme);
    applyThemeAttribute(theme);
    set({ theme });
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
  DEFAULT_THEME,
  loadPersistedTheme,
};
