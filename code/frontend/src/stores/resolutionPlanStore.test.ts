import { describe, it, expect, beforeEach } from 'vitest';
import {
  triggerChoiceKey,
  useResolutionPlanStore,
} from './resolutionPlanStore';
import { GamePhase, type PendingSelection } from '@/types/game';

const DECLINE = 62;

function makeMask(ids: number[]): number[] {
  const mask = Array(2192).fill(0);
  for (const id of ids) mask[id] = 1;
  return mask;
}

describe('resolutionPlanStore', () => {
  beforeEach(() => {
    useResolutionPlanStore.getState().clearPlan();
  });

  it('auto-dispatches remaining ordered triggers and optional presets sequentially', () => {
    const c1 = { index: 0, cardId: 'BT1-010', cardName: 'Agumon', label: 'On Play draw', actionId: 30, timing: 'OnPlay', isOptional: true };
    const c2 = { index: 1, cardId: 'BT1-085', cardName: 'Tai', label: 'Gain 1 memory', actionId: 31, timing: 'StartOfYourTurn', isOptional: false };
    const k1 = triggerChoiceKey(c1);
    const k2 = triggerChoiceKey(c2);

    useResolutionPlanStore.getState().startPlan({
      orderedKeys: [k1, k2],
      tierKeys: [k1, k2],
      presets: { [k1]: 'no' },
    });

    // Step 1: Optional yes/no follow-up for c1 -> preset 'no' dispatches DECLINE
    const optPrompt: PendingSelection = {
      phase: GamePhase.SelectEffectChoice,
      validIndices: [30, DECLINE],
      isOptional: true,
      prompt: 'Activate Agumon?',
      selectingPlayer: 1,
    };
    const a1 = useResolutionPlanStore
      .getState()
      .consumeNextAction(optPrompt, makeMask([30, DECLINE]), DECLINE);
    expect(a1).toBe(DECLINE);

    // Step 2: Engine returns to remaining simultaneous trigger c2 -> auto-dispatches 31
    const nextTierPrompt: PendingSelection = {
      phase: GamePhase.SelectEffectChoice,
      validIndices: [31],
      isOptional: false,
      prompt: 'Choose next trigger',
      selectingPlayer: 1,
      effectChoices: [c2],
    };
    const a2 = useResolutionPlanStore
      .getState()
      .consumeNextAction(nextTierPrompt, makeMask([31]), DECLINE);
    expect(a2).toBe(31);
  });

  it('cancels plan safely when a newly derived trigger interrupts (CR 15-4-5-2/3)', () => {
    const c1 = { index: 0, cardId: 'BT1-010', cardName: 'Agumon', label: 'Trigger A', actionId: 30, timing: 'OnPlay' };
    const c2 = { index: 1, cardId: 'BT1-085', cardName: 'Tai', label: 'Trigger B', actionId: 31, timing: 'OnPlay' };
    const derived = { index: 0, cardId: 'BT2-067', cardName: 'Impmon', label: 'Derived OnDeletion', actionId: 30, timing: 'OnDeletion' };

    const k1 = triggerChoiceKey(c1);
    const k2 = triggerChoiceKey(c2);

    useResolutionPlanStore.getState().startPlan({
      orderedKeys: [k1, k2],
      tierKeys: [k1, k2],
      presets: {},
    });

    const interruptPrompt: PendingSelection = {
      phase: GamePhase.SelectEffectChoice,
      validIndices: [30, 31],
      isOptional: false,
      prompt: 'Choose next trigger',
      selectingPlayer: 1,
      effectChoices: [derived, c2],
    };
    const next = useResolutionPlanStore
      .getState()
      .consumeNextAction(interruptPrompt, makeMask([30, 31]), DECLINE);
    expect(next).toBeNull();
    expect(useResolutionPlanStore.getState().plan).toBeNull();
  });
});
