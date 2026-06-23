import { useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useThemeStore, type Theme } from '@/design/theme/themeStore';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

/**
 * Dev-only bridge between the desktop debug bridge and the app's client-side
 * router + theme store. The screenshot skill (`update-landing-screenshots`)
 * POSTs `/navigate {route, theme}` to the bridge, which emits a `debug:navigate`
 * window event this component consumes to drive the real window page-by-page.
 *
 * Mounted only when `IS_DESKTOP && import.meta.env.DEV`, so production desktop
 * builds (`vite build --mode desktop`) tree-shake it out. The Tauri event API
 * is imported lazily so the web/test build never touches it.
 */
export function DebugBridgeNav() {
  const navigate = useNavigate();
  useEffect(() => {
    if (!IS_DESKTOP || !import.meta.env.DEV) return;
    let un: (() => void) | undefined;
    let cancelled = false;
    void (async () => {
      const { listen } = await import('@tauri-apps/api/event');
      const unlisten = await listen<{ route?: string; theme?: Theme }>(
        'debug:navigate',
        (e) => {
          const { route, theme } = e.payload ?? {};
          if (theme === 'dark' || theme === 'light') {
            useThemeStore.getState().setTheme(theme);
          }
          if (route) navigate(route);
        },
      );
      if (cancelled) unlisten();
      else un = unlisten;
    })();
    return () => {
      cancelled = true;
      un?.();
    };
  }, [navigate]);
  return null;
}
