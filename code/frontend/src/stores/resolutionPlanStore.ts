import { create } from 'zustand';
import { GamePhase, type EffectChoice, type PendingSelection } from '@/types/game';

export type OptionalPreset = 'yes' | 'no';

/**
 * Stable identity key for a trigger choice across sequential re-offers of the
 * same simultaneous-trigger tier (matches cardId + timing + label).
 */
export function triggerChoiceKey(choice: Pick<EffectChoice, 'cardId' | 'timing' | 'label'>): string {
  return `${choice.cardId}::${choice.timing ?? ''}::${choice.label}`;
}

export interface ActiveResolutionPlan {
  /** Remaining trigger keys to pick in order. */
  queue: string[];
  /** Keys originally present in this tier (used to detect derived-trigger interrupts). */
  tierKeys: string[];
  /** Optional yes/no presets per trigger key. */
  presets: Record<string, OptionalPreset>;
  /** Currently resolving trigger key (for matching follow-up optional prompt). */
  resolvingKey: string | null;
  /** PendingSelection snapshot that spawned/last-consumed this plan; wait for a new object before auto-dispatching again. */
  originSelection: PendingSelection | null;
}

interface ResolutionPlanStore {
  plan: ActiveResolutionPlan | null;
  /** Remaining actionIds queued from OrderCardsModal for sequential SelectPermutation dispatch. */
  permutationQueue: number[];
  /** PendingSelection snapshot that spawned/last-consumed permutationQueue. */
  permutationOrigin: PendingSelection | null;
  startPlan: (params: {
    orderedKeys: string[];
    tierKeys: string[];
    presets: Record<string, OptionalPreset>;
    originSelection?: PendingSelection | null;
  }) => void;
  /** Advance or cancel the plan against the newly arrived PendingSelection.
   *  Returns the concrete actionId to auto-dispatch, or null if human input is needed. */
  consumeNextAction: (
    pending: PendingSelection | null,
    actionMask: number[],
    declineActionId: number,
  ) => number | null;
  clearPlan: () => void;
  startPermutationOrder: (
    remainingActionIds: number[],
    originSelection?: PendingSelection | null,
  ) => void;
  consumeNextPermutation: (
    pending: PendingSelection | null,
    actionMask: number[],
  ) => number | null;
  clearPermutationOrder: () => void;
}

export const useResolutionPlanStore = create<ResolutionPlanStore>((set, get) => ({
  plan: null,
  permutationQueue: [],
  permutationOrigin: null,

  startPlan: ({ orderedKeys, tierKeys, presets, originSelection = null }) => {
    const [first, ...rest] = orderedKeys;
    set({
      plan: {
        queue: rest,
        tierKeys,
        presets,
        resolvingKey: first ?? null,
        originSelection,
      },
    });
  },

  consumeNextAction: (pending, actionMask, declineActionId) => {
    const plan = get().plan;
    if (!plan || !pending) {
      if (plan) set({ plan: null });
      return null;
    }

    // Do not auto-consume the exact PendingSelection snapshot that just spawned
    // or last dispatched a step; wait for the server response to deliver a new state.
    if (plan.originSelection && pending === plan.originSelection) {
      return null;
    }

    // Case 1: Optional yes/no follow-up for the currently resolving trigger.
    const nonDeclineValid = pending.validIndices.filter(
      (id) => id !== declineActionId && actionMask[id] === 1,
    );
    const canDecline = actionMask[declineActionId] === 1;
    const isSingleOptionalPrompt =
      (Boolean(pending.keywordPrompt) || pending.isOptional) &&
      canDecline &&
      nonDeclineValid.length === 1 &&
      (pending.effectChoices?.length ?? 0) <= 1;

    if (isSingleOptionalPrompt && plan.resolvingKey) {
      const preset = plan.presets[plan.resolvingKey];
      if (preset === 'yes') {
        set({ plan: { ...plan, resolvingKey: null, originSelection: pending } });
        return nonDeclineValid[0]!;
      }
      if (preset === 'no') {
        set({ plan: { ...plan, resolvingKey: null, originSelection: pending } });
        return declineActionId;
      }
      // Preset is 'Ask' — pause auto-dispatch so human answers, keep remaining queue
      set({ plan: { ...plan, resolvingKey: null, originSelection: pending } });
      return null;
    }

    // Case 2: Next simultaneous-trigger ordering prompt in the same tier.
    // Never auto-consume mutually exclusive EffectChoice branches — pause for human input.
    if (
      (!pending.kind || pending.kind === 'TriggerOrder') &&
      pending.effectChoices &&
      pending.effectChoices.length > 0 &&
      plan.queue.length > 0
    ) {
      const offeredEntries = pending.effectChoices.map((c) => ({
        key: triggerChoiceKey(c),
        actionId: c.actionId,
      }));
      const tierSet = new Set(plan.tierKeys);
      // Safety bail-out (CR §15-4-5-2/3): if ANY newly derived trigger appeared
      // outside the original planned tier, cancel the plan and ask the player!
      const hasDerivedInterrupt = offeredEntries.some((e) => !tierSet.has(e.key));
      if (hasDerivedInterrupt) {
        set({ plan: null });
        return null;
      }

      const [nextKey, ...remaining] = plan.queue;
      const match = offeredEntries.find(
        (e) => e.key === nextKey && e.actionId != null && actionMask[e.actionId] === 1,
      );
      if (match && match.actionId != null) {
        set({
          plan: {
            ...plan,
            queue: remaining,
            resolvingKey: nextKey ?? null,
            originSelection: pending,
          },
        });
        return match.actionId;
      }

      // Planned entry is no longer legal (e.g. source departed) — bail out cleanly
      set({ plan: null });
      return null;
    }

    // Any target/card/source selection requires human judgment — clear resolvingKey
    // and let the user pick; if queue is empty, clear the plan.
    if (plan.queue.length === 0) {
      set({ plan: null });
    }
    return null;
  },

  clearPlan: () => set({ plan: null }),

  startPermutationOrder: (remainingActionIds, originSelection = null) => {
    set({
      permutationQueue: [...remainingActionIds],
      permutationOrigin: originSelection,
    });
  },

  consumeNextPermutation: (pending, actionMask) => {
    const queue = get().permutationQueue;
    const origin = get().permutationOrigin;
    if (queue.length === 0) return null;
    if (origin && pending === origin) return null;
    if (!pending || pending.phase !== GamePhase.SelectPermutation) {
      set({ permutationQueue: [], permutationOrigin: null });
      return null;
    }
    const [nextId, ...rest] = queue;
    const isLegal =
      nextId != null &&
      (pending.validIndices.includes(nextId) || actionMask[nextId] === 1);
    if (!isLegal) {
      set({ permutationQueue: [], permutationOrigin: null });
      return null;
    }
    set({ permutationQueue: rest, permutationOrigin: pending });
    return nextId;
  },

  clearPermutationOrder: () => set({ permutationQueue: [], permutationOrigin: null }),
}));
