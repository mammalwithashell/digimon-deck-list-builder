import type { PendingSelection, TriggerPlan } from '@/types/game';
import { SELECTION } from './constants';

export type TriggerPlanStep =
  | { kind: 'action'; actionId: number; triggerId: string; activation: boolean }
  | { kind: 'wait' | 'stop' };

/** Rebind a human's plan to the current engine prompt, never to old action IDs. */
export function nextTriggerPlanStep(
  plan: TriggerPlan,
  pending: PendingSelection | null,
  mask: number[],
  localPlayer: number,
): TriggerPlanStep {
  if (!pending) return { kind: 'stop' };
  if (pending.selectingPlayer !== localPlayer) {
    return { kind: pending.kind === 'TriggerOrder' || pending.triggerChoices?.length ? 'stop' : 'wait' };
  }
  const choices = pending.triggerChoices;
  if (!choices?.length) return { kind: 'wait' };
  if (plan.generation && choices[0]?.generation !== plan.generation) return { kind: 'stop' };
  if (choices.some((choice) => !plan.offered.includes(choice.triggerId))) return { kind: 'stop' };
  const activation = choices.find((choice) => choice.activationPrompt);
  if (activation) {
    const answer = plan.optionalAnswers[activation.triggerId];
    if (answer === undefined) return { kind: 'wait' };
    const actionId = answer ? activation.actionId : SELECTION.DECLINE;
    if (!activation.isOptional || !pending.isOptional || mask[actionId] !== 1) return { kind: 'stop' };
    return { kind: 'action', actionId, triggerId: activation.triggerId, activation: true };
  }
  if (pending.kind !== 'TriggerOrder') return { kind: 'wait' };
  const next = plan.order.map((id) => choices.find((choice) => choice.triggerId === id)).find(Boolean);
  if (!next || !pending.validIndices.includes(next.actionId) || mask[next.actionId] !== 1) return { kind: 'stop' };
  return { kind: 'action', actionId: next.actionId, triggerId: next.triggerId, activation: false };
}
