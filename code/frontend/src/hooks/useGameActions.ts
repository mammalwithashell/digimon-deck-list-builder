import { useCallback, useRef } from 'react';
import { useGameStore } from '@/stores/gameStore';
import * as gameApi from '@/api/gameApi';

export function useGameActions() {
  const { gameId, setGameState, setActionMask, appendEvents } = useGameStore();
  const isPending = useRef(false);

  const sendAction = useCallback(
    async (actionId: number) => {
      if (!gameId || isPending.current) return;
      isPending.current = true;

      try {
        // Send the human action
        const actionResult = await gameApi.sendAction(gameId, actionId);
        setGameState(actionResult.state);
        setActionMask(actionResult.action_mask);
        if (actionResult.events) appendEvents(actionResult.events);

        // Then step the game (runs agent turns, pauses on next human turn)
        if (!actionResult.is_game_over) {
          const stepResult = await gameApi.stepGame(gameId);
          setGameState(stepResult.state);
          setActionMask(stepResult.action_mask);
          if (stepResult.events) appendEvents(stepResult.events);
        }
      } finally {
        isPending.current = false;
      }
    },
    [gameId, setGameState, setActionMask, appendEvents],
  );

  return { sendAction, isPending };
}
