import { describe, expect, it } from 'vitest';
import type { PendingSelection, TriggerChoice, TriggerPlan } from '@/types/game';
import { GamePhase } from '@/types/game';
import { nextTriggerPlanStep } from './triggerPlan';

function triggerChoice(id: string, actionId: number, optional = false): TriggerChoice {
  return { triggerId: id, actionId, cardId: `TEST-${id}`, cardName: `Effect ${id}`, effectText: `Gain ${id} memory`, timing: 'EndOfYourTurn', sourceLabel: 'Battle area 1', isOptional: optional, isInherited: false, activationPrompt: false };
}

const plan: TriggerPlan = { order: ['b', 'a', 'c'], offered: ['a', 'b', 'c'], optionalAnswers: { b: false } };
const mask = new Array(2192).fill(1);
function pending(choices: TriggerChoice[], kind = 'TriggerOrder'): PendingSelection {
  return { phase: GamePhase.SelectEffectChoice, selectingPlayer: 1, kind, validIndices: choices.map((c) => c.actionId), prompt: '', isOptional: false, triggerChoices: choices };
}

describe('trigger plan execution', () => {
  it('uses the current action for the planned identity after queue reindexing', () => {
    expect(nextTriggerPlanStep(plan, pending([triggerChoice('a', 30), triggerChoice('b', 31)]), mask, 1)).toMatchObject({ actionId: 31, triggerId: 'b' });
    expect(nextTriggerPlanStep(plan, pending([triggerChoice('a', 30), triggerChoice('c', 31)]), mask, 1)).toMatchObject({ actionId: 30, triggerId: 'a' });
  });
  it('interrupts on an unseen activation even if the old planned trigger is offered', () => {
    expect(nextTriggerPlanStep(plan, pending([triggerChoice('b', 30), triggerChoice('new', 31)]), mask, 1).kind).toBe('stop');
  });
  it('reopens after new triggers auto-resolved between responses', () => {
    const snapshot = pending([{ ...triggerChoice('b', 30), generation: '8' }]);
    expect(nextTriggerPlanStep({ ...plan, generation: '6' }, snapshot, mask, 1).kind).toBe('stop');
  });
  it('pauses for target choices and for Ask activation prompts', () => {
    expect(nextTriggerPlanStep(plan, { ...pending([]), kind: 'OwnField' }, mask, 1).kind).toBe('wait');
    expect(nextTriggerPlanStep(plan, pending([{ ...triggerChoice('a', 30, true), activationPrompt: true }]), mask, 1).kind).toBe('wait');
  });
  it('applies No only at a legal optional activation gate', () => {
    const gate = { ...pending([{ ...triggerChoice('b', 30, true), activationPrompt: true }], 'Replacement'), isOptional: true };
    expect(nextTriggerPlanStep(plan, gate, mask, 1)).toMatchObject({ actionId: 62, activation: true });
    expect(nextTriggerPlanStep(plan, { ...gate, isOptional: false }, mask, 1).kind).toBe('stop');
    expect(nextTriggerPlanStep(plan, gate, [], 1).kind).toBe('stop');
  });
  it('applies Yes only to the activation gate and pauses for an opponent target decision', () => {
    const yes = { ...plan, optionalAnswers: { b: true } };
    const gate = { ...pending([{ ...triggerChoice('b', 59, true), activationPrompt: true }], 'Replacement'), isOptional: true };
    expect(nextTriggerPlanStep(yes, gate, mask, 1)).toMatchObject({ actionId: 59, activation: true });
    expect(nextTriggerPlanStep(yes, { ...pending([]), kind: 'Hand', selectingPlayer: 2 }, mask, 1).kind).toBe('wait');
  });
  it('never acts for the opponent or past a partial plan', () => {
    expect(nextTriggerPlanStep(plan, { ...pending([triggerChoice('b', 30)]), selectingPlayer: 2 }, mask, 1).kind).toBe('stop');
    expect(nextTriggerPlanStep({ ...plan, order: ['b'] }, pending([triggerChoice('a', 30)]), mask, 1).kind).toBe('stop');
  });
});
