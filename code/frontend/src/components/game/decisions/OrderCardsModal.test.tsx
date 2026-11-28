import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { OrderCardsModal } from './OrderCardsModal';
import { useResolutionPlanStore } from '@/stores/resolutionPlanStore';
import { usePeekStore } from '@/stores/peekStore';
import { GamePhase, type PendingSelection } from '@/types/game';

describe('OrderCardsModal', () => {
  beforeEach(() => {
    useResolutionPlanStore.setState({ plan: null, permutationQueue: [] });
    usePeekStore.setState({ peeking: false });
  });

  const pending: PendingSelection = {
    phase: GamePhase.SelectPermutation,
    selectingPlayer: 1,
    validIndices: [30, 31, 32],
    isOptional: false,
    prompt: 'Return remaining cards to the bottom of your deck in any order',
    kind: 'OrderedPermutation { remaining: 3 }',
  };

  const entries = [
    { actionId: 30, cardId: 'BT1-010', label: 'Revealed' },
    { actionId: 31, cardId: 'BT1-015', label: 'Revealed' },
    { actionId: 32, cardId: 'BT1-025', label: 'Revealed' },
  ];

  it('numbers clicked cards 1..N, dispatches first pick, and queues remaining picks', () => {
    const onAction = vi.fn();
    render(
      <OrderCardsModal
        pendingSelection={pending}
        entries={entries}
        canDecline={false}
        onAction={onAction}
      />,
    );

    // Pick card 32 first, then 30, then 31
    fireEvent.click(screen.getByTitle('BT1-025'));
    fireEvent.click(screen.getByTitle('BT1-010'));
    fireEvent.click(screen.getByTitle('BT1-015'));

    fireEvent.click(screen.getByTestId('order-confirm'));

    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(32);
    expect(useResolutionPlanStore.getState().permutationQueue).toEqual([30, 31]);

    // Sequential drain validates and pops remaining picks
    const next1 = useResolutionPlanStore.getState().consumeNextPermutation(
      { ...pending, validIndices: [30, 31] },
      new Array(2192).fill(0),
    );
    expect(next1).toBe(30);
    const next2 = useResolutionPlanStore.getState().consumeNextPermutation(
      { ...pending, validIndices: [31] },
      new Array(2192).fill(0),
    );
    expect(next2).toBe(31);
    expect(useResolutionPlanStore.getState().permutationQueue).toEqual([]);
  });

  it('supports View Board peek toggle and return pill', () => {
    render(
      <OrderCardsModal
        pendingSelection={pending}
        entries={entries}
        canDecline={false}
        onAction={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId('order-view-board'));
    expect(screen.getByTestId('order-cards-modal')).toHaveClass('is-peeking');
    expect(screen.getByRole('button', { name: 'Return to Card Order' })).toBeInTheDocument();
  });
});
