import { useCallback, useEffect, useRef, useState } from 'react';
import type { PendingSelection, TriggerPlan } from '@/types/game';
import { nextTriggerPlanStep } from '@/utils/triggerPlan';

export function useTriggerPlan({ gameId, pending, mask, sendAction, localPlayer, gameOver, transportError }: {
  gameId: string | null;
  pending: PendingSelection | null;
  mask: number[];
  sendAction: (id: number) => Promise<void>;
  localPlayer: number;
  gameOver: boolean;
  transportError?: string | null;
}) {
  const [plan, setPlan] = useState<TriggerPlan | null>(null);
  const [busy, setBusy] = useState(false);
  const inFlight = useRef(false);
  const sent = useRef('');
  const session = useRef(gameId);
  const cancel = useCallback(() => { setPlan(null); sent.current = ''; }, []);
  const submit = useCallback((next: TriggerPlan) => {
    sent.current = '';
    setPlan(next);
  }, []);

  useEffect(() => {
    if (session.current !== gameId || gameOver || transportError) {
      session.current = gameId;
      cancel();
      return;
    }
    if (!plan || inFlight.current) return;
    const step = nextTriggerPlanStep(plan, pending, mask, localPlayer);
    if (step.kind === 'stop') { cancel(); return; }
    if (step.kind !== 'action') return;
    const fingerprint = JSON.stringify([step.triggerId, step.actionId, step.activation]);
    if (sent.current === fingerprint) return;
    sent.current = fingerprint;
    inFlight.current = true;
    setBusy(true);
    // Each planned choice uses the normal transport, mask validation and recording.
    void sendAction(step.actionId).catch(cancel).finally(() => {
      inFlight.current = false;
      setBusy(false);
    });
  }, [gameId, gameOver, transportError, plan, pending, mask, sendAction, localPlayer, cancel, busy]);

  return { submit, cancel, active: plan !== null, busy };
}
