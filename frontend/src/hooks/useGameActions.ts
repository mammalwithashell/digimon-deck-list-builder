import { useCallback } from 'react';
import { useGameStore } from '@/stores/gameStore';
import * as gameApi from '@/api/gameApi';

export function useGameActions() {
  const { gameId, setGameState, setActionMask, appendLogs, appendEvents } = useGameStore();

  const sendAction = useCallback(
    async (actionId: number) => {
      if (!gameId) return;

      // Send the human action
      const actionResult = await gameApi.sendAction(gameId, actionId);
      setGameState(actionResult.state);
      setActionMask(actionResult.action_mask);
      if (actionResult.logs) appendLogs(actionResult.logs);
      if (actionResult.events) appendEvents(actionResult.events);

      // Then step the game (runs agent turns, pauses on next human turn)
      if (!actionResult.is_game_over) {
        const stepResult = await gameApi.stepGame(gameId);
        setGameState(stepResult.state);
        setActionMask(stepResult.action_mask);
        if (stepResult.logs) appendLogs(stepResult.logs);
        if (stepResult.events) appendEvents(stepResult.events);
      }
    },
    [gameId, setGameState, setActionMask, appendLogs, appendEvents],
  );

  return { sendAction };
}
