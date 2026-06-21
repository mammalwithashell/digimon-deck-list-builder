import { renderHook, act } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useWebSocketGame, type UseWebSocketGameOptions } from './useWebSocketGame';

// Minimal WebSocket stand-in: records instances and lets the test drive the
// lifecycle callbacks (open / message / close) the hook wires up.
class MockWebSocket {
  static instances: MockWebSocket[] = [];
  url: string;
  onopen: (() => void) | null = null;
  onmessage: ((e: { data: string }) => void) | null = null;
  onclose: ((e: { code: number; reason: string }) => void) | null = null;
  onerror: (() => void) | null = null;
  sent: string[] = [];
  closeArgs: { code?: number; reason?: string } | null = null;
  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }
  send(data: string) {
    this.sent.push(data);
  }
  close(code?: number, reason?: string) {
    this.closeArgs = { code, reason };
    this.onclose?.({ code: code ?? 1000, reason: reason ?? '' });
  }
}

function opts(overrides: Partial<UseWebSocketGameOptions> = {}): UseWebSocketGameOptions {
  return { gameId: 'g1', role: 'player', ...overrides };
}

function last(): MockWebSocket {
  return MockWebSocket.instances[MockWebSocket.instances.length - 1]!;
}

function stateUpdate(playerId?: number): string {
  return JSON.stringify({ type: 'state_update', your_player_id: playerId, state: {} });
}

beforeEach(() => {
  MockWebSocket.instances = [];
  vi.stubGlobal('WebSocket', MockWebSocket);
  localStorage.setItem('access_token', 'tok');
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
  localStorage.clear();
});

describe('useWebSocketGame', () => {
  it('errors out when the socket opens but never sends game state', () => {
    vi.useFakeTimers();
    const { result } = renderHook(() =>
      useWebSocketGame(opts({ connectTimeoutMs: 15000 })),
    );
    act(() => {
      last().onopen?.();
    });
    expect(result.current.status).toBe('connected');

    act(() => {
      vi.advanceTimersByTime(15000);
    });
    expect(result.current.status).toBe('error');
    expect(result.current.error).toBeTruthy();
  });

  it('does not time out once the first state_update arrives', () => {
    vi.useFakeTimers();
    const onStateUpdate = vi.fn();
    const { result } = renderHook(() =>
      useWebSocketGame(opts({ connectTimeoutMs: 15000, onStateUpdate })),
    );
    act(() => {
      last().onopen?.();
      last().onmessage?.({ data: stateUpdate(1) });
    });
    expect(result.current.status).toBe('connected');
    expect(result.current.myPlayerId).toBe(1);
    expect(onStateUpdate).toHaveBeenCalledTimes(1);

    act(() => {
      vi.advanceTimersByTime(60000);
    });
    expect(result.current.status).toBe('connected');
    expect(result.current.error).toBeNull();
  });

  it('reports an error and does not retry on a terminal close code', () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useWebSocketGame(opts()));
    act(() => {
      last().onclose?.({ code: 4004, reason: 'Game not found' });
    });
    expect(result.current.status).toBe('error');
    expect(result.current.error).toMatch(/game not found/i);
    // No reconnect attempt for a terminal rejection.
    act(() => {
      vi.advanceTimersByTime(60000);
    });
    expect(MockWebSocket.instances).toHaveLength(1);
  });

  it('marks status disconnected and retries on a transient close', () => {
    vi.useFakeTimers();
    // Make the game live first so the connect-timeout deadline is cleared and
    // we exercise the mid-game reconnect path.
    const { result } = renderHook(() => useWebSocketGame(opts()));
    act(() => {
      last().onopen?.();
      last().onmessage?.({ data: stateUpdate(1) });
    });

    act(() => {
      last().onclose?.({ code: 1006, reason: '' });
    });
    expect(result.current.status).toBe('disconnected');
    expect(result.current.retryCount).toBe(1);

    // First backoff is 1000ms — after it a fresh socket is opened.
    act(() => {
      vi.advanceTimersByTime(1000);
    });
    expect(MockWebSocket.instances).toHaveLength(2);
  });

  it('errors out after exhausting reconnect retries', () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useWebSocketGame(opts()));
    act(() => {
      last().onopen?.();
      last().onmessage?.({ data: stateUpdate(1) });
    });

    // 5 transient drops each reconnect; the 6th exhausts the retry budget.
    for (let i = 0; i < 5; i++) {
      act(() => {
        last().onclose?.({ code: 1006, reason: '' });
      });
      expect(result.current.status).toBe('disconnected');
      act(() => {
        vi.advanceTimersByTime(60000);
      });
    }
    act(() => {
      last().onclose?.({ code: 1006, reason: '' });
    });
    expect(result.current.status).toBe('error');
    expect(result.current.error).toBeTruthy();
  });

  it('reconnect() clears the error and opens a fresh socket', () => {
    vi.useFakeTimers();
    const { result } = renderHook(() => useWebSocketGame(opts()));
    act(() => {
      last().onclose?.({ code: 4004, reason: 'nope' });
    });
    expect(result.current.status).toBe('error');

    act(() => {
      result.current.reconnect();
    });
    expect(result.current.status).toBe('connecting');
    expect(result.current.error).toBeNull();
    expect(MockWebSocket.instances).toHaveLength(2);
  });

  it('errors immediately when there is no auth token', () => {
    localStorage.removeItem('access_token');
    const onError = vi.fn();
    const { result } = renderHook(() => useWebSocketGame(opts({ onError })));
    expect(result.current.status).toBe('error');
    expect(result.current.error).toBeTruthy();
    expect(onError).toHaveBeenCalled();
    expect(MockWebSocket.instances).toHaveLength(0);
  });
});
