import { beforeEach, describe, expect, it } from 'vitest';
import { usePlayFlowStore } from './playFlowStore';

describe('playFlowStore', () => {
  beforeEach(() => {
    sessionStorage.clear();
    usePlayFlowStore.getState().reset();
  });

  it('stores the selected format, opponent mode, and deck id', () => {
    usePlayFlowStore.getState().selectFormat('standard');
    usePlayFlowStore.getState().selectOpponentMode('quick');
    usePlayFlowStore.getState().selectDeck('deck-1');

    expect(usePlayFlowStore.getState()).toMatchObject({
      formatId: 'standard',
      opponentMode: 'quick',
      deckId: 'deck-1',
    });
  });

  it('resets transient queue and room fields without clearing the selected format', () => {
    usePlayFlowStore.getState().selectFormat('standard');
    usePlayFlowStore.getState().setQueue({
      ticketId: 'ticket-1',
      roomCode: 'ABC123',
      seed: '777',
    });
    usePlayFlowStore.getState().clearLaunchState();

    expect(usePlayFlowStore.getState().formatId).toBe('standard');
    expect(usePlayFlowStore.getState().ticketId).toBeNull();
    expect(usePlayFlowStore.getState().roomCode).toBeNull();
    expect(usePlayFlowStore.getState().seed).toBeNull();
  });

  it('stores the effective game seed as a string', () => {
    usePlayFlowStore.getState().setQueue({ gameId: 'game-1', seed: '18446744073709551615' });

    expect(usePlayFlowStore.getState().seed).toBe('18446744073709551615');
  });
});
