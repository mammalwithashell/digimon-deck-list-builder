import '@testing-library/jest-dom/vitest';
import { render } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { GameLog } from './GameLog';

// Regression tests for "clicking the log moves the UI": the log's auto-scroll
// used `scrollIntoView`, which scrolls EVERY scrollable ancestor to reveal the
// target — including the game board's `overflow-hidden` container the log
// drawer is absolutely positioned in (overflow:hidden boxes are still
// programmatically scrollable). Opening the drawer therefore shifted the whole
// board, and the user had no way to scroll it back. The fix scrolls only the
// log's own container via `scrollTop`.

const originalScrollIntoView = Element.prototype.scrollIntoView;

afterEach(() => {
  Element.prototype.scrollIntoView = originalScrollIntoView;
});

describe('GameLog auto-scroll containment', () => {
  it('never calls scrollIntoView (it would scroll the board container too)', () => {
    const spy = vi.fn();
    // jsdom does not implement scrollIntoView, so the old code's call was a
    // silent no-op in tests — install a spy to catch any regression.
    Element.prototype.scrollIntoView = spy;

    const { rerender } = render(<GameLog logs={['first line']} />);
    rerender(<GameLog logs={['first line', 'second line']} />);

    expect(spy).not.toHaveBeenCalled();
  });

  it('scrolls only its own container to the bottom when logs grow', () => {
    const { rerender, getByTestId } = render(<GameLog logs={['first line']} />);
    const scroller = getByTestId('game-log-scroll');

    // Simulate overflowing content (jsdom has no layout, so scrollHeight
    // defaults to 0).
    Object.defineProperty(scroller, 'scrollHeight', {
      configurable: true,
      value: 500,
    });

    rerender(<GameLog logs={['first line', 'second line']} />);

    expect(scroller.scrollTop).toBe(500);
  });
});
