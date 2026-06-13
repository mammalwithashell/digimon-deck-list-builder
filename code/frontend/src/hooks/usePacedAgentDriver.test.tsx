import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useGameStore } from '@/stores/gameStore';
import { useUiStore, BOT_SPEED_DELAY_MS } from '@/stores/uiStore';
import { usePacedAgentDriver, type PacedStepResponse } from './usePacedAgentDriver';
import type { GameState } from '@/types/game';

vi.mock('@/api/gameApi', () => ({
  stepGame: vi.fn(),
}));

import * as gameApi from '@/api/gameApi';

const stepGameMock = vi.mocked(gameApi.stepGame);

type StepResp = Awaited<ReturnType<typeof gameApi.stepGame>>;

// The driver only reads `agent_pending` and `action_mask` off the response
// (the rest flows through applyGameResponse, which the test implements
// directly against the store), so a skeletal state object suffices.
function beat(agentPending: boolean): StepResp {
  return {
    state: {} as GameState,
    action_mask: [0, 1],
    logs: [],
    events: [],
    is_human_turn: !agentPending,
    is_game_over: false,
    agent_pending: agentPending,
  };
}

/** Mimics GamePage's applyGameResponse: lands the response in the store so
 *  the driver's re-arm deps (fresh mask array, agentPending) update. */
function applyToStore(res: PacedStepResponse): void {
  useGameStore.setState({
    actionMask: [...res.action_mask],
    agentPending: res.agent_pending ?? false,
  });
}

function primeStore(overrides: Partial<ReturnType<typeof useGameStore.getState>> = {}) {
  useGameStore.setState({
    gameId: 'test-game',
    agentPending: true,
    isGameOver: false,
    actionMask: [1, 0],
    ...overrides,
  });
}

describe('usePacedAgentDriver', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    stepGameMock.mockReset();
    useGameStore.setState({
      gameId: null,
      agentPending: false,
      isGameOver: false,
      actionMask: [],
    });
    useUiStore.setState({ botSpeed: 'fast' });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('advances one paced beat per delay until agent_pending clears', async () => {
    stepGameMock
      .mockResolvedValueOnce(beat(true))
      .mockResolvedValueOnce(beat(false));
    primeStore();

    renderHook(() =>
      usePacedAgentDriver({ active: true, applyGameResponse: applyToStore }),
    );

    // Nothing fires before the configured delay elapses.
    expect(stepGameMock).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(BOT_SPEED_DELAY_MS.fast);
    });
    expect(stepGameMock).toHaveBeenCalledTimes(1);
    expect(stepGameMock).toHaveBeenLastCalledWith('test-game', { paced: true });
    expect(useGameStore.getState().agentPending).toBe(true);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(BOT_SPEED_DELAY_MS.fast);
    });
    expect(stepGameMock).toHaveBeenCalledTimes(2);
    expect(useGameStore.getState().agentPending).toBe(false);

    // Sequence drained — no further beats are scheduled.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BOT_SPEED_DELAY_MS.fast * 4);
    });
    expect(stepGameMock).toHaveBeenCalledTimes(2);
  });

  it('instant speed drains the rest in one unpaced call, bypassing timers', async () => {
    useUiStore.setState({ botSpeed: 'instant' });
    stepGameMock.mockResolvedValueOnce(beat(false));
    primeStore();

    renderHook(() =>
      usePacedAgentDriver({ active: true, applyGameResponse: applyToStore }),
    );

    // The instant drain fires immediately (no setTimeout) and unpaced.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(stepGameMock).toHaveBeenCalledTimes(1);
    expect(stepGameMock).toHaveBeenLastCalledWith('test-game');
    expect(useGameStore.getState().agentPending).toBe(false);
  });

  it('retries a failed beat once, then stalls behind resumePacing', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    stepGameMock
      .mockRejectedValueOnce(new Error('boom'))
      .mockRejectedValueOnce(new Error('boom again'))
      .mockResolvedValueOnce(beat(false));
    primeStore();

    const { result } = renderHook(() =>
      usePacedAgentDriver({ active: true, applyGameResponse: applyToStore }),
    );

    await act(async () => {
      await vi.advanceTimersByTimeAsync(BOT_SPEED_DELAY_MS.fast);
    });
    // First attempt + inline retry both failed → stalled, no timer re-armed.
    expect(stepGameMock).toHaveBeenCalledTimes(2);
    expect(result.current.pacingStalled).toBe(true);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(BOT_SPEED_DELAY_MS.fast * 4);
    });
    expect(stepGameMock).toHaveBeenCalledTimes(2);

    // Manual continue resumes the sequence.
    await act(async () => {
      result.current.resumePacing();
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(stepGameMock).toHaveBeenCalledTimes(3);
    expect(result.current.pacingStalled).toBe(false);
    expect(useGameStore.getState().agentPending).toBe(false);
    consoleError.mockRestore();
  });

  it('is inert for server-paced (WebSocket) games and when idle', async () => {
    primeStore(); // agentPending true, but driver inactive
    renderHook(() =>
      usePacedAgentDriver({ active: false, applyGameResponse: applyToStore }),
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BOT_SPEED_DELAY_MS.fast * 4);
    });
    expect(stepGameMock).not.toHaveBeenCalled();

    // Active but nothing pending → also inert.
    primeStore({ agentPending: false });
    renderHook(() =>
      usePacedAgentDriver({ active: true, applyGameResponse: applyToStore }),
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BOT_SPEED_DELAY_MS.fast * 4);
    });
    expect(stepGameMock).not.toHaveBeenCalled();
  });
});
