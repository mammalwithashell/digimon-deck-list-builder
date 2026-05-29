import '@testing-library/jest-dom/vitest';
import { render, screen, act } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { GraphicsSettingsPage } from './GraphicsSettingsPage';
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
    render(<GraphicsSettingsPage />);
    for (const preset of RESOLUTION_PRESETS) {
      const btn = screen.getByTestId(
        `graphics-preset-${preset.width}x${preset.height}`,
      );
      expect(btn).toBeInTheDocument();
      expect(btn).toHaveTextContent(`${preset.width}×${preset.height}`);
    }
  });

  it('clicking a preset updates the store, persists to localStorage, and calls setSize', async () => {
    render(<GraphicsSettingsPage />);
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
    render(<GraphicsSettingsPage />);
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
    render(<GraphicsSettingsPage />);
    const preset = RESOLUTION_PRESETS[0]!;
    const btn = screen.getByTestId(
      `graphics-preset-${preset.width}x${preset.height}`,
    );
    expect(btn).toBeDisabled();
  });
});
