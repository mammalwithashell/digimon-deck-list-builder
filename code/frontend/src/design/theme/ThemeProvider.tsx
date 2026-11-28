import { useEffect, type ReactNode } from 'react';
import {
  useThemeStore,
  applyThemeAttribute,
  applyStyleAttribute,
  type Theme,
  type StylePreset,
} from './themeStore';

/**
 * Syncs the active theme polarity (`data-theme`) and style preset (`data-style`)
 * to <html> and exposes `useTheme()`.
 */
export function ThemeProvider({ children }: { children: ReactNode }) {
  const theme = useThemeStore((s) => s.theme);
  const stylePreset = useThemeStore((s) => s.stylePreset);

  useEffect(() => {
    applyThemeAttribute(theme);
  }, [theme]);

  useEffect(() => {
    applyStyleAttribute(stylePreset);
  }, [stylePreset]);

  return <>{children}</>;
}

export interface UseThemeResult {
  theme: Theme;
  stylePreset: StylePreset;
  setTheme: (theme: Theme) => void;
  setStylePreset: (stylePreset: StylePreset) => void;
  toggle: () => void;
}

export function useTheme(): UseThemeResult {
  return {
    theme: useThemeStore((s) => s.theme),
    stylePreset: useThemeStore((s) => s.stylePreset),
    setTheme: useThemeStore((s) => s.setTheme),
    setStylePreset: useThemeStore((s) => s.setStylePreset),
    toggle: useThemeStore((s) => s.toggle),
  };
}
