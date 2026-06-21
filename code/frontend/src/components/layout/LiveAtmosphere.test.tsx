import '@testing-library/jest-dom/vitest';
import { render, act } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { LiveAtmosphere } from './LiveAtmosphere';
import { useUiStore } from '@/stores/uiStore';
import { useThemeStore } from '@/design/theme/themeStore';

beforeEach(() => {
  useUiStore.setState({ motion: 'full', liveBackground: true });
  useThemeStore.setState({ theme: 'dark' });
  // jsdom has no 2d canvas; stub a no-op context and ResizeObserver so the
  // dark-rain effect can run without throwing.
  HTMLCanvasElement.prototype.getContext = vi.fn(() => ({
    fillRect: () => {},
    fillText: () => {},
    font: '',
    fillStyle: '',
  })) as unknown as typeof HTMLCanvasElement.prototype.getContext;
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
});

describe('LiveAtmosphere', () => {
  it('renders a non-interactive atmosphere layer', () => {
    const { container } = render(<LiveAtmosphere surface="menu" />);
    const layer = container.querySelector('.live-atmosphere');
    expect(layer).toBeInTheDocument();
    expect(layer).toHaveAttribute('aria-hidden', 'true');
    expect(layer).toHaveAttribute('data-surface', 'menu');
  });

  it('is live when effective live-background is on', () => {
    useUiStore.setState({ motion: 'full', liveBackground: true });
    const { container } = render(<LiveAtmosphere />);
    expect(container.querySelector('.live-atmosphere')).toHaveClass('is-live');
  });

  it('falls back to static when motion is reduced', () => {
    useUiStore.setState({ motion: 'reduced', liveBackground: true });
    const { container } = render(<LiveAtmosphere />);
    expect(container.querySelector('.live-atmosphere')).not.toHaveClass(
      'is-live',
    );
  });

  it('falls back to static when the live-background toggle is off', () => {
    useUiStore.setState({ motion: 'full', liveBackground: false });
    const { container } = render(<LiveAtmosphere />);
    expect(container.querySelector('.live-atmosphere')).not.toHaveClass(
      'is-live',
    );
  });

  it('renders the rain canvas in the dark theme', () => {
    useThemeStore.setState({ theme: 'dark' });
    const { container } = render(<LiveAtmosphere />);
    expect(container.querySelector('.live-atmosphere__rain')).toBeInTheDocument();
    expect(container.querySelector('.live-atmosphere__teal')).toBeNull();
  });

  it('renders the idle-desktop layers in the light theme', () => {
    useThemeStore.setState({ theme: 'light' });
    const { container } = render(<LiveAtmosphere />);
    expect(container.querySelector('.live-atmosphere__teal')).toBeInTheDocument();
    expect(container.querySelector('.live-atmosphere__rain')).toBeNull();
  });

  it('handles visibility changes and unmounts cleanly (dark + live)', () => {
    useThemeStore.setState({ theme: 'dark' });
    const { unmount } = render(<LiveAtmosphere />);
    expect(() => {
      act(() => {
        document.dispatchEvent(new Event('visibilitychange'));
      });
      unmount();
    }).not.toThrow();
  });
});
