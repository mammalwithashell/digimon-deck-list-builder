import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { GamePhase, type PendingSelection, type TriggerChoice } from '@/types/game';
import { useTriggerPlan } from './useTriggerPlan';

function selection(ids: string[]): PendingSelection {
  return { phase: GamePhase.SelectEffectChoice, kind: 'TriggerOrder', selectingPlayer: 1, isOptional: false, prompt: '', validIndices: ids.map((_, i) => 30 + i),
    triggerChoices: ids.map((id, i): TriggerChoice => ({ triggerId: id, actionId: 30 + i, cardId: id, cardName: id, effectText: '', timing: '', sourceLabel: '', isOptional: false, isInherited: false, activationPrompt: false })) };
}

describe('trigger plan transport coordination', () => {
  it('waits for server acknowledgement before sending another WebSocket action', async () => {
    const sendAction = vi.fn(async () => {});
    const initial = selection(['a', 'b', 'c']);
    const { result, rerender } = renderHook(({ pending }) => useTriggerPlan({ gameId: 'game', pending, mask: new Array(2192).fill(1), localPlayer: 1, gameOver: false, sendAction }), { initialProps: { pending: initial } });
    act(() => result.current.submit({ order: ['b', 'c', 'a'], offered: ['a', 'b', 'c'], optionalAnswers: {} }));
    await waitFor(() => expect(sendAction).toHaveBeenCalledTimes(1));
    rerender({ pending: { ...initial } });
    expect(sendAction).toHaveBeenCalledTimes(1);
    rerender({ pending: selection(['a', 'c']) });
    await waitFor(() => expect(sendAction).toHaveBeenCalledTimes(2));
    expect(sendAction.mock.calls).toEqual([[31], [31]]);
    rerender({ pending: selection(['a', 'new']) });
    await waitFor(() => expect(result.current.active).toBe(false));
    expect(sendAction).toHaveBeenCalledTimes(2);
  });
});
