import { describe, expect, it } from 'vitest';
import { connectionScreenState } from './connectionScreen';

describe('connectionScreenState', () => {
  it('never gates local (non-WebSocket) games', () => {
    expect(
      connectionScreenState({
        useWebSocket: false,
        status: 'error',
        retryCount: 9,
        boardHydrated: false,
      }),
    ).toBe('ready');
  });

  it('reports error for a failed WebSocket connection', () => {
    expect(
      connectionScreenState({
        useWebSocket: true,
        status: 'error',
        retryCount: 5,
        boardHydrated: false,
      }),
    ).toBe('error');
  });

  it('is ready once the board has hydrated from the first state_update', () => {
    expect(
      connectionScreenState({
        useWebSocket: true,
        status: 'connected',
        retryCount: 0,
        boardHydrated: true,
      }),
    ).toBe('ready');
  });

  it('shows connecting on the initial attempt before any state arrives', () => {
    expect(
      connectionScreenState({
        useWebSocket: true,
        status: 'connecting',
        retryCount: 0,
        boardHydrated: false,
      }),
    ).toBe('connecting');
  });

  it('treats a socket that opened but has not yet sent state as connecting', () => {
    expect(
      connectionScreenState({
        useWebSocket: true,
        status: 'connected',
        retryCount: 0,
        boardHydrated: false,
      }),
    ).toBe('connecting');
  });

  it('shows reconnecting while the socket is down and retrying', () => {
    expect(
      connectionScreenState({
        useWebSocket: true,
        status: 'disconnected',
        retryCount: 0,
        boardHydrated: false,
      }),
    ).toBe('reconnecting');
  });

  it('shows reconnecting once at least one retry has been attempted', () => {
    expect(
      connectionScreenState({
        useWebSocket: true,
        status: 'connecting',
        retryCount: 2,
        boardHydrated: false,
      }),
    ).toBe('reconnecting');
  });

  it('prioritises a hydrated board over a transient disconnected status', () => {
    // A mid-game blip that still has board data should keep playing, not gate.
    expect(
      connectionScreenState({
        useWebSocket: true,
        status: 'disconnected',
        retryCount: 1,
        boardHydrated: true,
      }),
    ).toBe('ready');
  });
});
