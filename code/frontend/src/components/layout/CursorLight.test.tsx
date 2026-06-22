import '@testing-library/jest-dom/vitest';
import { render, act } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { CursorLight } from './CursorLight';
import { useUiStore } from '@/stores/uiStore';

function flushFrame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

describe('CursorLight', () => {
  beforeEach(() => {
    useUiStore.setState({ motion: 'full' });
  });

  it('renders a non-interactive glow layer at full motion', () => {
    const { container } = render(<CursorLight />);
    const layer = container.querySelector('.cursor-light');
    expect(layer).toBeInTheDocument();
    expect(layer).toHaveAttribute('aria-hidden', 'true');
    expect(container.querySelector('.cursor-light__glow')).toBeInTheDocument();
  });

  it('renders nothing at reduced motion', () => {
    useUiStore.setState({ motion: 'reduced' });
    const { container } = render(<CursorLight />);
    expect(container.querySelector('.cursor-light')).toBeNull();
  });

  it('renders nothing at off motion', () => {
    useUiStore.setState({ motion: 'off' });
    const { container } = render(<CursorLight />);
    expect(container.querySelector('.cursor-light')).toBeNull();
  });

  it('tracks the pointer into CSS custom properties (rAF-throttled)', async () => {
    const { container } = render(<CursorLight />);
    const glow = container.querySelector('.cursor-light__glow') as HTMLElement;
    await act(async () => {
      const ev = new Event('pointermove');
      Object.assign(ev, { clientX: 120, clientY: 80 });
      window.dispatchEvent(ev);
      await flushFrame();
    });
    // jsdom getBoundingClientRect is all-zero, so the relative coords equal the
    // client coords.
    expect(glow.style.getPropertyValue('--cursor-x')).toBe('120px');
    expect(glow.style.getPropertyValue('--cursor-y')).toBe('80px');
  });
});
