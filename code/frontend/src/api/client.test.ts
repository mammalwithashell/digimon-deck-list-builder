import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { InternalAxiosRequestConfig } from 'axios';

const remintMock = vi.fn();
vi.mock('@/bootstrap/guest', () => ({
  isGuestToken: (token: string | null) =>
    token === 'offline-guest-token' || token === 'guest-jwt-stale',
  remintGuestSession: (...args: unknown[]) => remintMock(...args),
}));
vi.mock('@/stores/authStore', () => ({
  useAuthStore: { setState: vi.fn() },
}));

import client from './client';

/** Install a scripted adapter: each call shifts the next status from the
 * queue; 401s reject the way axios does (error.response.status). */
function scriptAdapter(statuses: number[]) {
  const calls: InternalAxiosRequestConfig[] = [];
  client.defaults.adapter = async (config: InternalAxiosRequestConfig) => {
    calls.push(config);
    const status = statuses.shift() ?? 200;
    const response = { data: { ok: status < 400 }, status, statusText: '', headers: {}, config };
    if (status >= 400) {
      const err = new Error(`Request failed with status code ${status}`) as Error & {
        response: typeof response;
        config: InternalAxiosRequestConfig;
        isAxiosError: boolean;
      };
      err.response = response;
      err.config = config;
      err.isAxiosError = true;
      throw err;
    }
    return response;
  };
  return calls;
}

describe('client 401 guest re-mint', () => {
  beforeEach(() => {
    localStorage.clear();
    remintMock.mockReset();
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('re-mints once on 401 with a guest token and retries with the new bearer', async () => {
    localStorage.setItem('access_token', 'guest-jwt-stale');
    remintMock.mockResolvedValue({
      token: 'guest-jwt-fresh',
      userId: 'guest_new',
      displayName: 'Guest-NEWW',
    });
    const calls = scriptAdapter([401, 200]);

    const resp = await client.get('/lobby/x/state');

    expect(resp.status).toBe(200);
    expect(remintMock).toHaveBeenCalledTimes(1);
    expect(localStorage.getItem('access_token')).toBe('guest-jwt-fresh');
    expect(calls[1]?.headers.Authorization).toBe('Bearer guest-jwt-fresh');
  });

  it('surfaces the error when the retried request 401s again (no mint loop)', async () => {
    localStorage.setItem('access_token', 'guest-jwt-stale');
    remintMock.mockResolvedValue({
      token: 'guest-jwt-fresh',
      userId: 'guest_new',
      displayName: 'Guest-NEWW',
    });
    scriptAdapter([401, 401]);

    await expect(client.get('/lobby/x/state')).rejects.toMatchObject({
      response: { status: 401 },
    });
    expect(remintMock).toHaveBeenCalledTimes(1);
  });

  it('does not re-mint for non-guest tokens', async () => {
    localStorage.setItem('access_token', 'registered-user-jwt');
    scriptAdapter([401]);

    await expect(client.get('/lobby/x/state')).rejects.toMatchObject({
      response: { status: 401 },
    });
    expect(remintMock).not.toHaveBeenCalled();
  });

  it('surfaces the original 401 when the re-mint itself fails (offline)', async () => {
    localStorage.setItem('access_token', 'offline-guest-token');
    remintMock.mockRejectedValue(new Error('network down'));
    scriptAdapter([401]);

    await expect(client.get('/lobby/x/state')).rejects.toMatchObject({
      response: { status: 401 },
    });
    expect(remintMock).toHaveBeenCalledTimes(1);
  });
});
