import '@testing-library/jest-dom/vitest';
import { render, screen, act } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { GraphicsSettingsPage } from './GraphicsSettingsPage';

// The page renders a <Link> ("Back to Launcher"), so it needs a router context.
function renderPage() {
  return render(
    <MemoryRouter>
      <GraphicsSettingsPage />
    </MemoryRouter>,
  );
}
import { DEFAULT_PRESET, RESOLUTION_PRESETS } from '@/utils/constants';
import { useUiStore } from '@/stores/uiStore';

// Stub the Tauri window API — the page imports it dynamically, so we
// register a vi.mock for the bare module path. The mock records calls
// so we can assert setSize / setFullscreen were invoked. Typed
// signatures preserve `mock.calls[i][j]` element types for TS.
const setSize = vi.fn(async (_size: { width: number; height: number }) => {});
const setFullscreen = vi.fn(async (_value: boolean) => {});

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({ setSize, setFullscreen }),
  LogicalSize: class {
    width: number;
    height: number;
    constructor(w: number, h: number) {
      this.width = w;
      this.height = h;
    }
  },
}));

describe('GraphicsSettingsPage', () => {
  beforeEach(() => {
    window.localStorage.clear();
    setSize.mockClear();
    setFullscreen.mockClear();
    // Reset the store to defaults so each test starts clean.
    useUiStore.setState({
      graphicsPreset: DEFAULT_PRESET,
      fullscreen: false,
    });
  });

  it('renders all 8 resolution preset buttons in order', () => {
    renderPage();
    for (const preset of RESOLUTION_PRESETS) {
      const btn = screen.getByTestId(
        `graphics-preset-${preset.width}x${preset.height}`,
      );
      expect(btn).toBeInTheDocument();
      expect(btn).toHaveTextContent(`${preset.width}×${preset.height}`);
    }
  });

  it('clicking a preset updates the store, persists to localStorage, and calls setSize', async () => {
    renderPage();
    const preset = RESOLUTION_PRESETS[4]!; // 2560x1440
    const btn = screen.getByTestId(
      `graphics-preset-${preset.width}x${preset.height}`,
    );

    await act(async () => {
      btn.click();
      // Let the microtask queue drain for the dynamic import.
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(useUiStore.getState().graphicsPreset).toEqual(preset);
    expect(
      JSON.parse(window.localStorage.getItem('desktop.graphicsPreset')!),
    ).toEqual(preset);
    expect(setSize).toHaveBeenCalled();
    const call = setSize.mock.calls[0]?.[0];
    expect(call?.width).toBe(preset.width);
    expect(call?.height).toBe(preset.height);
  });

  it('clicking the fullscreen toggle flips state and calls setFullscreen', async () => {
    renderPage();
    const toggle = screen.getByTestId('graphics-fullscreen-toggle');

    await act(async () => {
      toggle.click();
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(useUiStore.getState().fullscreen).toBe(true);
    expect(setFullscreen).toHaveBeenCalledWith(true);
  });

  it('disables preset buttons while fullscreen is active', () => {
    useUiStore.setState({ fullscreen: true });
    renderPage();
    const preset = RESOLUTION_PRESETS[0]!;
    const btn = screen.getByTestId(
      `graphics-preset-${preset.width}x${preset.height}`,
    );
    expect(btn).toBeDisabled();
  });
});

describe('GraphicsSettingsPage motion + live background', () => {
  beforeEach(() => {
    window.localStorage.clear();
    useUiStore.setState({ motion: 'full', liveBackground: true });
  });

  it('renders the three motion options and the live-background toggle', () => {
    renderPage();
    expect(screen.getByTestId('graphics-motion-full')).toBeInTheDocument();
    expect(screen.getByTestId('graphics-motion-reduced')).toBeInTheDocument();
    expect(screen.getByTestId('graphics-motion-off')).toBeInTheDocument();
    expect(screen.getByTestId('graphics-livebg-toggle')).toBeInTheDocument();
  });

  it('changes the motion level via the store on click', () => {
    renderPage();
    act(() => {
      screen.getByTestId('graphics-motion-reduced').click();
    });
    expect(useUiStore.getState().motion).toBe('reduced');
  });

  it('toggles the live background via the store on click', () => {
    renderPage();
    act(() => {
      screen.getByTestId('graphics-livebg-toggle').click();
    });
    expect(useUiStore.getState().liveBackground).toBe(false);
  });

  it('reflects the active motion level with aria-pressed', () => {
    useUiStore.setState({ motion: 'off' });
    renderPage();
    expect(screen.getByTestId('graphics-motion-off')).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    expect(screen.getByTestId('graphics-motion-full')).toHaveAttribute(
      'aria-pressed',
      'false',
    );
  });

  it('renders the text-size options and changes the scale via the store', () => {
    useUiStore.setState({ textScale: 'md' });
    renderPage();
    expect(screen.getByTestId('graphics-textscale-sm')).toBeInTheDocument();
    expect(screen.getByTestId('graphics-textscale-xl')).toBeInTheDocument();
    act(() => {
      screen.getByTestId('graphics-textscale-lg').click();
    });
    expect(useUiStore.getState().textScale).toBe('lg');
  });
});
