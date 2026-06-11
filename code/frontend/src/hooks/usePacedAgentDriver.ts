import { useCallback, useEffect, useState } from 'react';
import { useGameStore } from '@/stores/gameStore';
import { useUiStore, BOT_SPEED_DELAY_MS } from '@/stores/uiStore';
import * as gameApi from '@/api/gameApi';
import type { ActionTrace, GameEvent, GameState } from '@/types/game';

export interface PacedStepResponse {
  state: GameState;
  action_mask: number[];
  logs?: string[];
  events?: GameEvent[];
  action_traces?: ActionTrace[];
  agent_pending?: boolean;
}

/**
 * Bot action pacing driver (add-bot-action-pacing).
 *
 * While the engine reports `agent_pending` (a non-human agent still has
 * actions to take in paced mode), this hook requests the next single-action
 * beat after the user-configured delay, so each bot action renders as a
 * perceivable step instead of the whole bot turn snapping in at once.
 *
 * - Re-arms after every applied response (`actionMask` is a fresh array per
 *   response, so the effect re-runs even though `agentPending` stays true).
 * - Speed changes apply from the next beat; switching to Instant
 *   mid-sequence drains the remainder in one legacy unpaced call.
 * - A failed beat retries once; a second failure stalls pacing behind the
 *   returned `resumePacing` affordance instead of silently spinning.
 * - Fully inert when `active` is false (WebSocket games are server-paced)
 *   or when nothing is pending.
 */
export function usePacedAgentDriver(opts: {
  /** False for WebSocket (PvP / vs-AI-online / spectator) games. */
  active: boolean;
  /** Applies a step-shaped response to the game store, including
   *  `agent_pending` (GamePage's `applyGameResponse`). */
  applyGameResponse: (res: PacedStepResponse) => void;
}) {
  const { active, applyGameResponse } = opts;
  const botSpeed = useUiStore((s) => s.botSpeed);
  const gameId = useGameStore((s) => s.gameId);
  const agentPending = useGameStore((s) => s.agentPending);
  const isGameOver = useGameStore((s) => s.isGameOver);
  const actionMask = useGameStore((s) => s.actionMask);
  const [pacingStalled, setPacingStalled] = useState(false);

  const advanceAgentBeat = useCallback(async () => {
    const gid = useGameStore.getState().gameId;
    if (!gid) return;
    try {
      const res = await gameApi.stepGame(gid, { paced: true });
      applyGameResponse(res);
    } catch (firstErr) {
      console.error('Paced agent step failed, retrying once:', firstErr);
      try {
        const res = await gameApi.stepGame(gid, { paced: true });
        applyGameResponse(res);
      } catch (secondErr) {
        console.error('Paced agent step failed twice — stalling:', secondErr);
        setPacingStalled(true);
      }
    }
  }, [applyGameResponse]);

  const resumePacing = useCallback(() => {
    setPacingStalled(false);
    void advanceAgentBeat();
  }, [advanceAgentBeat]);

  // A stall is only meaningful mid-sequence; once nothing is pending
  // (game advanced anyway, undo, new game) clear it so no stale banner
  // lingers.
  useEffect(() => {
    if (!agentPending) setPacingStalled(false);
  }, [agentPending]);

  useEffect(() => {
    if (!active || !gameId) return;
    if (!agentPending || isGameOver || pacingStalled) return;
    if (botSpeed === 'instant') {
      // Switched to Instant mid-sequence — drain the rest in one legacy
      // unpaced call (its response reports agent_pending: false).
      void (async () => {
        try {
          const res = await gameApi.stepGame(gameId);
          applyGameResponse(res);
        } catch (err) {
          console.error('Instant drain of agent turn failed:', err);
          setPacingStalled(true);
        }
      })();
      return;
    }
    const timer = setTimeout(() => {
      void advanceAgentBeat();
    }, BOT_SPEED_DELAY_MS[botSpeed]);
    return () => clearTimeout(timer);
  }, [
    active,
    advanceAgentBeat,
    agentPending,
    applyGameResponse,
    // Re-arm key: a fresh mask array arrives with every applied response.
    actionMask,
    botSpeed,
    gameId,
    isGameOver,
    pacingStalled,
  ]);

  return { pacingStalled, resumePacing };
}
