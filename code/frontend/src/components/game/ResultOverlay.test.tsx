import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ResultOverlay } from './ResultOverlay';

describe('ResultOverlay seed controls', () => {
  it('keeps the seed copyable before returning to the launcher', () => {
    const writeText = vi.fn();
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    const onReturn = vi.fn();

    render(
      <ResultOverlay
        isGameOver
        winner={1}
        localPlayer={1}
        playerLabels={{ 1: 'You', 2: 'Bot' }}
        gameSeed="18446744073709551615"
        onReturnToMenu={onReturn}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: /copy seed/i }));
    expect(writeText).toHaveBeenCalledWith('18446744073709551615');

    fireEvent.click(screen.getByRole('button', { name: /return to launcher/i }));
    expect(onReturn).toHaveBeenCalledTimes(1);
  });
});
