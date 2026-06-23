import '@testing-library/jest-dom/vitest';
import { render, screen, act } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { GraphicsSettingsPanel } from './GraphicsSettingsPanel';
import { DEFAULT_PRESET, RESOLUTION_PRESETS } from '@/utils/constants';
import { useUiStore } from '@/stores/uiStore';

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

describe('GraphicsSettingsPanel', () => {
  beforeEach(() => {
    window.localStorage.clear();
    setSize.mockClear();
    setFullscreen.mockClear();
    useUiStore.setState({
      graphicsPreset: DEFAULT_PRESET,
      fullscreen: false,
      motion: 'full',
      liveBackground: true,
      textScale: 'md',
    });
  });

  it('renders all resolution presets', () => {
    render(<GraphicsSettingsPanel />);
    for (const preset of RESOLUTION_PRESETS) {
      expect(
        screen.getByTestId(`graphics-preset-${preset.width}x${preset.height}`),
      ).toBeInTheDocument();
    }
  });

  it('clicking a preset updates the store and calls setSize', async () => {
    render(<GraphicsSettingsPanel />);
    const preset = RESOLUTION_PRESETS[4]!;
    await act(async () => {
      screen
        .getByTestId(`graphics-preset-${preset.width}x${preset.height}`)
        .click();
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(useUiStore.getState().graphicsPreset).toEqual(preset);
    expect(setSize).toHaveBeenCalled();
  });

  it('exposes the motion, live-background, and text-size controls', () => {
    render(<GraphicsSettingsPanel />);
    expect(screen.getByTestId('graphics-motion-reduced')).toBeInTheDocument();
    expect(screen.getByTestId('graphics-livebg-toggle')).toBeInTheDocument();
    expect(screen.getByTestId('graphics-textscale-lg')).toBeInTheDocument();
    act(() => {
      screen.getByTestId('graphics-textscale-lg').click();
    });
    expect(useUiStore.getState().textScale).toBe('lg');
  });
});
