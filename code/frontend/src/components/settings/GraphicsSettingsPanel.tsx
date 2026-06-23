import { useUiStore, type Motion, type TextScale } from '@/stores/uiStore';
import { ThemeSwitch } from '@/design/components/ThemeSwitch';
import {
  RESOLUTION_PRESETS,
  type ResolutionPreset,
} from '@/utils/constants';

const MOTION_OPTIONS: { value: Motion; label: string }[] = [
  { value: 'full', label: 'Full' },
  { value: 'reduced', label: 'Reduced' },
  { value: 'off', label: 'Off' },
];

const TEXT_SCALE_OPTIONS: { value: TextScale; label: string }[] = [
  { value: 'sm', label: 'Small' },
  { value: 'md', label: 'Default' },
  { value: 'lg', label: 'Large' },
  { value: 'xl', label: 'X-Large' },
];

/**
 * Graphics settings content (theme, text size, motion, live background,
 * fullscreen, resolution presets). Extracted from the former standalone page
 * so it can render inside the centered Graphics modal (GraphicsSettingsModal).
 *
 * Desktop-only — the Tauri window API is imported dynamically so a stray
 * web-context render does not error.
 */
export function GraphicsSettingsPanel() {
  const graphicsPreset = useUiStore((s) => s.graphicsPreset);
  const fullscreen = useUiStore((s) => s.fullscreen);
  const setGraphicsPreset = useUiStore((s) => s.setGraphicsPreset);
  const setFullscreen = useUiStore((s) => s.setFullscreen);
  const motion = useUiStore((s) => s.motion);
  const setMotion = useUiStore((s) => s.setMotion);
  const liveBackground = useUiStore((s) => s.liveBackground);
  const setLiveBackground = useUiStore((s) => s.setLiveBackground);
  const textScale = useUiStore((s) => s.textScale);
  const setTextScale = useUiStore((s) => s.setTextScale);

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
    <div>
      <h1
        className="mb-2 text-2xl font-semibold text-[var(--ink-0)]"
        style={{ fontFamily: 'var(--font-display)' }}
      >
        Graphics Settings
      </h1>
      <p className="mb-8 text-sm text-[var(--ink-1)]">
        Pick a window size. The game board renders inside a fixed 1920×1080
        internal canvas and is uniformly scaled to fit the selected window.
        Ultrawide presets letterbox the canvas with side bars.
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

      <div className="mb-4 flex items-center justify-between border border-[var(--line-1)] bg-[var(--surface-raised)] px-4 py-3">
        <div>
          <div className="text-sm font-medium text-[var(--ink-0)]">Text size</div>
          <div className="text-xs text-[var(--ink-2)]">
            Scales interface text in menus. Pick what’s comfortable to read.
          </div>
        </div>
        <div className="flex gap-1" role="group" aria-label="Text size">
          {TEXT_SCALE_OPTIONS.map((opt) => {
            const active = textScale === opt.value;
            return (
              <button
                key={opt.value}
                type="button"
                data-testid={`graphics-textscale-${opt.value}`}
                onClick={() => setTextScale(opt.value)}
                aria-pressed={active}
                className={`border px-3 py-1.5 text-xs font-semibold uppercase tracking-wider transition-colors ${
                  active
                    ? 'border-[var(--accent)] bg-[var(--surface-raised)] text-[var(--ink-0)]'
                    : 'border-[var(--line-1)] bg-[var(--surface-sunken)] text-[var(--ink-1)] hover:border-[var(--accent)] hover:text-[var(--ink-0)]'
                }`}
              >
                {opt.label}
              </button>
            );
          })}
        </div>
      </div>

      <div className="mb-4 flex items-center justify-between border border-[var(--line-1)] bg-[var(--surface-raised)] px-4 py-3">
        <div>
          <div className="text-sm font-medium text-[var(--ink-0)]">Motion</div>
          <div className="text-xs text-[var(--ink-2)]">
            How much animation the app plays. Reduced keeps essential feedback;
            Off stops non-essential motion.
          </div>
        </div>
        <div className="flex gap-1" role="group" aria-label="Motion level">
          {MOTION_OPTIONS.map((opt) => {
            const active = motion === opt.value;
            return (
              <button
                key={opt.value}
                type="button"
                data-testid={`graphics-motion-${opt.value}`}
                onClick={() => setMotion(opt.value)}
                aria-pressed={active}
                className={`border px-3 py-1.5 text-xs font-semibold uppercase tracking-wider transition-colors ${
                  active
                    ? 'border-[var(--accent)] bg-[var(--surface-raised)] text-[var(--ink-0)]'
                    : 'border-[var(--line-1)] bg-[var(--surface-sunken)] text-[var(--ink-1)] hover:border-[var(--accent)] hover:text-[var(--ink-0)]'
                }`}
              >
                {opt.label}
              </button>
            );
          })}
        </div>
      </div>

      <div className="mb-8 flex items-center justify-between border border-[var(--line-1)] bg-[var(--surface-raised)] px-4 py-3">
        <div>
          <div className="text-sm font-medium text-[var(--ink-0)]">Live background</div>
          <div className="text-xs text-[var(--ink-2)]">
            Animated theme background. Only active when Motion is Full.
          </div>
        </div>
        <button
          type="button"
          data-testid="graphics-livebg-toggle"
          onClick={() => setLiveBackground(!liveBackground)}
          aria-pressed={liveBackground}
          className={`relative h-7 w-14 shrink-0 rounded-full border transition-colors ${
            liveBackground
              ? 'border-[var(--accent)] bg-[var(--accent)]'
              : 'border-[var(--line-1)] bg-[var(--surface-sunken)]'
          }`}
        >
          <span
            className="absolute left-1 top-1/2 h-5 w-5 rounded-full bg-[var(--ink-0)] transition-transform"
            style={{
              transform: `translateY(-50%) translateX(${liveBackground ? '26px' : '0px'})`,
            }}
          />
        </button>
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
          className={`relative h-7 w-14 shrink-0 rounded-full border transition-colors ${
            fullscreen
              ? 'border-[var(--accent)] bg-[var(--accent)]'
              : 'border-[var(--line-1)] bg-[var(--surface-sunken)]'
          }`}
        >
          <span
            className="absolute left-1 top-1/2 h-5 w-5 rounded-full bg-[var(--ink-0)] transition-transform"
            style={{
              transform: `translateY(-50%) translateX(${fullscreen ? '26px' : '0px'})`,
            }}
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
  );
}
