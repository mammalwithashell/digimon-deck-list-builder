import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { MemoryGauge } from './MemoryGauge';
import { GamePhase } from '@/types/game';

describe('MemoryGauge arena variant', () => {
  it('passes when a handler is given', () => {
    const onEndTurn = vi.fn();
    render(<MemoryGauge value={2} localPlayer={1} currentPhase={GamePhase.Main} variant="arena" onEndTurn={onEndTurn} />);
    fireEvent.click(screen.getByTestId('board-end-turn'));
    expect(onEndTurn).toHaveBeenCalledOnce();
  });

  it('is disabled without a handler', () => {
    render(<MemoryGauge value={-2} localPlayer={1} currentPhase={GamePhase.Main} variant="arena" endTurnLabel="Enemy Turn" />);
    const button = screen.getByTestId('board-end-turn');
    expect(button).toBeDisabled();
    expect(button).toHaveTextContent('Enemy Turn');
  });

  it('marks the coin at the current memory', () => {
    const { container } = render(<MemoryGauge value={3} localPlayer={1} currentPhase={GamePhase.Main} variant="arena" />);
    const focal = container.querySelector('.ib-gauge-seg.is-focal');
    expect(focal).toHaveTextContent('3');
    expect(focal).toHaveClass('ib-gauge-seg--player');
  });
});

describe('MemoryGauge classic variant', () => {
  it('has the end-turn button and ticks between numbered steps', () => {
    const onEndTurn = vi.fn();
    const { container } = render(
      <MemoryGauge value={0} localPlayer={1} currentPhase={GamePhase.Main} onEndTurn={onEndTurn} />,
    );
    fireEvent.click(screen.getByTestId('board-end-turn'));
    expect(onEndTurn).toHaveBeenCalledOnce();
    expect(container.querySelectorAll('.ib-gauge-seg.is-tick').length).toBeGreaterThan(0);
  });
});
