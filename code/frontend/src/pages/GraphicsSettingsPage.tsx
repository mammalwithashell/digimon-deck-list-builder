import { Link } from 'react-router-dom';
import { useUiStore } from '@/stores/uiStore';
import { ThemeSwitch } from '@/design/components/ThemeSwitch';
import {
  RESOLUTION_PRESETS,
  type ResolutionPreset,
} from '@/utils/constants';

/**
 * Desktop Graphics Settings page.
 *
 * Lists the eight DCGO-matched resolution presets plus a fullscreen
 * toggle. Selecting a preset applies it immediately via the Tauri
 * window API and persists through `useUiStore` (localStorage-backed).
 *
 * The page is gated behind `VITE_BUILD_TARGET === 'desktop'` at the
 * route level in `App.tsx`; this component assumes it only renders in
 * desktop builds. The Tauri API import is dynamic so a stray hot-reload
 * in a web context does not error.
 */
export function GraphicsSettingsPage() {
  const graphicsPreset = useUiStore((s) => s.graphicsPreset);
  const fullscreen = useUiStore((s) => s.fullscreen);
  const setGraphicsPreset = useUiStore((s) => s.setGraphicsPreset);
  const setFullscreen = useUiStore((s) => s.setFullscreen);

  const applyPreset = async (preset: ResolutionPreset) => {
    setGraphicsPreset(preset);
    try {
      const { getCurrentWindow, LogicalSize } = await import(
        '@tauri-apps/api/window'
      );
      const w = getCurrentWindow();
      // Make sure we're windowed before applying the preset — otherwise
      // setSize is a no-op while fullscreen.
      await w.setFullscreen(false);
      setFullscreen(false);
      await w.setSize(new LogicalSize(preset.width, preset.height));
    } catch (err) {
      console.warn('GraphicsSettings: failed to apply preset', err);
    }
  };

  const applyFullscreen = async (value: boolean) => {
    setFullscreen(value);
    try {
      const { getCurrentWindow, LogicalSize } = await import(
        '@tauri-apps/api/window'
      );
      const w = getCurrentWindow();
      await w.setFullscreen(value);
      if (!value) {
        // Returning from fullscreen — restore to the last selected preset.
        await w.setSize(
          new LogicalSize(graphicsPreset.width, graphicsPreset.height),
        );
      }
    } catch (err) {
      console.warn('GraphicsSettings: failed to toggle fullscreen', err);
    }
  };

  const isPresetActive = (preset: ResolutionPreset): boolean =>
    preset.width === graphicsPreset.width &&
    preset.height === graphicsPreset.height;

  return (
    <div className="mx-auto max-w-3xl px-6 py-10">
      <Link
        to="/"
        className="mb-4 inline-flex items-center gap-2 border border-[var(--line-1)] bg-[var(--surface)] px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-[var(--ink-1)] shadow-[var(--bevel-shadow)] transition-colors hover:text-[var(--ink-0)]"
      >
        ← Back to Launcher
      </Link>

      <div className="border border-[var(--line-1)] bg-[var(--surface)] p-6 shadow-[var(--bevel-shadow)]">
        <h1 className="mb-2 text-2xl font-semibold text-[var(--ink-0)]" style={{ fontFamily: 'var(--font-display)' }}>
          Graphics Settings
        </h1>
        <p className="mb-8 text-sm text-[var(--ink-1)]">
          Pick a window size. The game board renders inside a fixed
          1920×1080 internal canvas and is uniformly scaled to fit the
          selected window. Ultrawide presets letterbox the canvas with
          side bars.
        </p>

        <div className="mb-4 flex items-center justify-between border border-[var(--line-1)] bg-[var(--surface-raised)] px-4 py-3">
          <div>
            <div className="text-sm font-medium text-[var(--ink-0)]">Theme</div>
            <div className="text-xs text-[var(--ink-2)]">
              Switch between the dark “Digi-OS” and light “Adventure ’99” looks.
            </div>
          </div>
          <ThemeSwitch />
        </div>

        <div className="mb-8 flex items-center justify-between border border-[var(--line-1)] bg-[var(--surface-raised)] px-4 py-3">
          <div>
            <div className="text-sm font-medium text-[var(--ink-0)]">Fullscreen</div>
            <div className="text-xs text-[var(--ink-2)]">
              Toggling off returns the window to the selected preset size.
            </div>
          </div>
          <button
            type="button"
            data-testid="graphics-fullscreen-toggle"
            onClick={() => void applyFullscreen(!fullscreen)}
            aria-pressed={fullscreen}
            className={`relative h-7 w-14 rounded-full border transition-colors ${
              fullscreen
                ? 'border-[var(--accent)] bg-[var(--accent)]'
                : 'border-[var(--line-1)] bg-[var(--surface-sunken)]'
            }`}
          >
            <span
              className={`absolute top-0.5 h-6 w-6 rounded-full bg-[var(--ink-0)] transition-transform ${
                fullscreen ? 'translate-x-7' : 'translate-x-0.5'
              }`}
            />
          </button>
        </div>

        <div className="grid grid-cols-2 gap-3">
          {RESOLUTION_PRESETS.map((preset) => {
            const active = isPresetActive(preset);
            return (
              <button
                key={`${preset.width}x${preset.height}`}
                type="button"
                data-testid={`graphics-preset-${preset.width}x${preset.height}`}
                onClick={() => void applyPreset(preset)}
                disabled={fullscreen}
                className={`border px-6 py-4 text-lg font-semibold transition-colors ${
                  active
                    ? 'border-[var(--accent)] bg-[var(--surface-raised)] text-[var(--ink-0)]'
                    : 'border-[var(--line-1)] bg-[var(--surface-sunken)] text-[var(--ink-1)] hover:border-[var(--accent)] hover:text-[var(--ink-0)]'
                } ${fullscreen ? 'cursor-not-allowed opacity-50' : ''}`}
              >
                {preset.width}×{preset.height}
              </button>
            );
          })}
        </div>

        {fullscreen && (
          <p className="mt-6 text-xs text-[var(--ink-2)]">
            Disable fullscreen to change the window preset.
          </p>
        )}
      </div>
    </div>
  );
}
