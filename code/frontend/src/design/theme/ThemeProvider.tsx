import { useEffect, type ReactNode } from 'react';
import { useThemeStore, applyThemeAttribute, type Theme } from './themeStore';

/**
 * Syncs the active theme to the `data-theme` attribute on <html> and exposes
 * `useTheme()`. The index.html bootstrap sets the attribute pre-paint; this
 * provider keeps it in sync with the store after mount (e.g. if the store
 * hydrated to a value the bootstrap didn't apply).
 */
export function ThemeProvider({ children }: { children: ReactNode }) {
  const theme = useThemeStore((s) => s.theme);

  useEffect(() => {
    applyThemeAttribute(theme);
  }, [theme]);

  return <>{children}</>;
}

export interface UseThemeResult {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggle: () => void;
}

export function useTheme(): UseThemeResult {
  return {
    theme: useThemeStore((s) => s.theme),
    setTheme: useThemeStore((s) => s.setTheme),
    toggle: useThemeStore((s) => s.toggle),
  };
}
