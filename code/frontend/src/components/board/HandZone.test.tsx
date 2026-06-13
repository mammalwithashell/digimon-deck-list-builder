import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { DndContext } from '@dnd-kit/core';
import { HandZone } from './HandZone';

function renderHand(props: Partial<Parameters<typeof HandZone>[0]> = {}) {
  return render(
    <DndContext>
      <HandZone cardIds={['BT24-018']} isOpponent={false} {...props} />
    </DndContext>,
  );
}

describe('HandZone right-click inspect', () => {
  it('right-clicking your own hand card opens the inspect view and suppresses the native menu', () => {
    const onCardInspect = vi.fn();
    renderHand({ onCardInspect });
    const card = screen.getByTitle('BT24-018');
    const evt = fireEvent.contextMenu(card);
    expect(onCardInspect).toHaveBeenCalledWith('BT24-018');
    expect(evt).toBe(false); // default (native context menu) prevented
  });

  it("right-clicking the opponent's face-down hand does nothing", () => {
    const onCardInspect = vi.fn();
    renderHand({ isOpponent: true, onCardInspect });
    // Opponent cards render face-down (no title attr) — right-click the card wrapper.
    const wrapper = document.querySelector('.ib-hand-card');
    expect(wrapper).not.toBeNull();
    const evt = fireEvent.contextMenu(wrapper!);
    expect(onCardInspect).not.toHaveBeenCalled();
    expect(evt).toBe(false); // still suppress the native menu over the board
  });
});
