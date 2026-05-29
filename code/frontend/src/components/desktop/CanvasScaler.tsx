import { useEffect, useState, type ReactNode } from 'react';
import { useUiStore } from '@/stores/uiStore';
import { DESIGN_CANVAS } from '@/utils/constants';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

/**
 * Set CSS custom properties on `<html>` so any descendant of the canvas
 * can use `var(--app-vh100, 100vh)` (and friends) to size against the
 * fixed canvas instead of the actual window viewport.
 *
 * Without this, pages using `100vh` or `min-h-screen` size themselves
 * to 720px (the real viewport at the smallest preset) and leave ~360px
 * of dead space inside the 1080px canvas.
 *
 * Variables are only set on the desktop build. Web builds get the
 * `100vh`/`100vw` fallback so the responsive layout is untouched.
 */
function installCanvasViewportVars() {
  if (!IS_DESKTOP || typeof document === 'undefined') return;
  const root = document.documentElement;
  root.style.setProperty('--app-vh100', `${DESIGN_CANVAS.height}px`);
  root.style.setProperty('--app-vw100', `${DESIGN_CANVAS.width}px`);
  // Single-vh equivalents for finer-grained math (`calc(var(--app-vh) * 50)`).
  root.style.setProperty('--app-vh', `${DESIGN_CANVAS.height / 100}px`);
  root.style.setProperty('--app-vw', `${DESIGN_CANVAS.width / 100}px`);
}

interface CanvasScalerProps {
  children: ReactNode;
}

/**
 * Fixed-canvas scaler for the desktop game UI.
 *
 * Renders `children` inside a 1920×1080 internal box and applies a
 * uniform `transform: scale(s)` where
 *   s = min(window.innerWidth / 1920, window.innerHeight / 1080).
 *
 * The wrapper flexes-center the inner box so windows whose aspect ratio
 * exceeds 16:9 (e.g. 3440×1440) letterbox with side bars in the
 * background color rather than stretching the canvas. Smaller-than-16:9
 * windows pillarbox top/bottom for the same reason.
 *
 * Non-desktop builds (`VITE_BUILD_TARGET !== 'desktop'`) render
 * `children` directly with no scaling so the responsive web layout is
 * untouched.
 *
 * On mount the scaler applies the persisted `graphicsPreset` and
 * `fullscreen` flags from `useUiStore` to the Tauri window. The Tauri
 * window API is loaded dynamically so the import doesn't break the web
 * bundle.
 */
export function CanvasScaler({ children }: CanvasScalerProps) {
  const graphicsPreset = useUiStore((s) => s.graphicsPreset);
  const fullscreen = useUiStore((s) => s.fullscreen);
  const setCanvasScale = useUiStore((s) => s.setCanvasScale);
  const [scale, setScale] = useState<number>(() => computeScale());

  // Install canvas-viewport CSS variables on `<html>` so descendants
  // that use `100vh` / `min-h-screen` size to the canvas (1080px) and
  // not the actual window viewport. Runs once on mount; the values are
  // constants (DESIGN_CANVAS), so no need to recompute on resize.
  useEffect(() => {
    installCanvasViewportVars();
  }, []);

  // Mirror the current scale into the UI store so callers outside this
  // component (e.g. the game's DnD modifier) can compensate for the
  // CSS transform when interpreting window-pixel pointer deltas.
  useEffect(() => {
    setCanvasScale(IS_DESKTOP ? scale : 1);
  }, [scale, setCanvasScale]);

  // Dev-only window helpers for Playwright-driven browser testing.
  // Exposes the engine API + game store so a test can bootstrap a game
  // without going through the Tauri-only deck-select flow. No-op in
  // production builds and in actual Tauri runtime — the conditions
  // guarantee this only fires in `npm run dev:desktop` opened in a
  // plain browser.
  useEffect(() => {
    if (!IS_DESKTOP) return;
    if (typeof window === 'undefined') return;
    if ((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) return;
    let cancelled = false;
    void (async () => {
      const [gameApi, { useGameStore }] = await Promise.all([
        import('@/api/gameApi'),
        import('@/stores/gameStore'),
      ]);
      if (cancelled) return;
      (window as unknown as { __dev__?: unknown }).__dev__ = {
        gameApi,
        useGameStore,
      };
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // Recompute scale on window resize and when the user toggles
  // fullscreen / picks a new preset (the window dimensions change
  // asynchronously after the Tauri API call lands).
  useEffect(() => {
    if (!IS_DESKTOP) return;
    const handler = () => setScale(computeScale());
    window.addEventListener('resize', handler);
    // Also recompute on next tick after preset changes — Tauri resize
    // doesn't fire 'resize' immediately under some conditions.
    const t = window.setTimeout(handler, 50);
    return () => {
      window.removeEventListener('resize', handler);
      window.clearTimeout(t);
    };
  }, [graphicsPreset, fullscreen]);

  // Apply persisted preset + fullscreen to the Tauri window on mount.
  // Dynamic import keeps `@tauri-apps/api/window` out of the web bundle.
  useEffect(() => {
    if (!IS_DESKTOP) return;
    let cancelled = false;
    void (async () => {
      try {
        const { getCurrentWindow, LogicalSize } = await import(
          '@tauri-apps/api/window'
        );
        if (cancelled) return;
        const w = getCurrentWindow();
        if (fullscreen) {
          await w.setFullscreen(true);
        } else {
          await w.setFullscreen(false);
          await w.setSize(
            new LogicalSize(graphicsPreset.width, graphicsPreset.height),
          );
        }
        // Recompute one more time after Tauri reports the new size.
        if (!cancelled) setScale(computeScale());
      } catch {
        // Either not running under Tauri or window API failed — fall back
        // to whatever scale the current window produces.
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [graphicsPreset, fullscreen]);

  if (!IS_DESKTOP) {
    return <>{children}</>;
  }

  return (
    <div
      data-testid="canvas-scaler"
      style={{
        position: 'fixed',
        inset: 0,
        background: '#000',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        overflow: 'hidden',
      }}
    >
      <div
        data-testid="canvas-scaler-inner"
        style={{
          width: DESIGN_CANVAS.width,
          height: DESIGN_CANVAS.height,
          transform: `scale(${scale})`,
          transformOrigin: 'center center',
          flexShrink: 0,
          // Children using `h-full` / `height: 100%` need a flex column
          // here so they fill the canvas's 1080px height rather than
          // sizing to their intrinsic content.
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        {children}
      </div>
    </div>
  );
}

/** Compute the uniform scale factor for the current window. Exported
 *  via `__canvasScalerInternals` for unit-testing without rendering. */
function computeScale(): number {
  if (typeof window === 'undefined') return 1;
  const w = window.innerWidth || DESIGN_CANVAS.width;
  const h = window.innerHeight || DESIGN_CANVAS.height;
  return Math.min(w / DESIGN_CANVAS.width, h / DESIGN_CANVAS.height);
}

export const __canvasScalerInternals = { computeScale };
