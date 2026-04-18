import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ensureGuestSession, GUEST_TOKEN_KEY, GUEST_USER_ID_KEY, GUEST_NAME_KEY } from './guest';

describe('ensureGuestSession', () => {
  const originalFetch = globalThis.fetch;
  beforeEach(() => {
    localStorage.clear();
  });
  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it('mints a new guest token if localStorage is empty', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 201,
      json: async () => ({
        access_token: 'guest-jwt',
        user_id: 'guest_abc',
        display_name: 'Guest-ABCD',
      }),
    }) as unknown as typeof fetch;

    const session = await ensureGuestSession();

    expect(session).toEqual({
      token: 'guest-jwt',
      userId: 'guest_abc',
      displayName: 'Guest-ABCD',
    });
    expect(localStorage.getItem(GUEST_TOKEN_KEY)).toBe('guest-jwt');
    expect(localStorage.getItem(GUEST_USER_ID_KEY)).toBe('guest_abc');
    expect(localStorage.getItem(GUEST_NAME_KEY)).toBe('Guest-ABCD');
    expect(globalThis.fetch).toHaveBeenCalledTimes(1);
  });

  it('reuses cached token on subsequent boots without hitting the network', async () => {
    localStorage.setItem(GUEST_TOKEN_KEY, 'existing');
    localStorage.setItem(GUEST_USER_ID_KEY, 'guest_existing');
    localStorage.setItem(GUEST_NAME_KEY, 'Guest-EEEE');
    const fetchMock = vi.fn();
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    const session = await ensureGuestSession();

    expect(session.token).toBe('existing');
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('throws if the network mint fails so the UI can show an offline banner', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false, status: 500, text: async () => 'boom',
    }) as unknown as typeof fetch;
    await expect(ensureGuestSession()).rejects.toThrow(/guest session/i);
    expect(localStorage.getItem(GUEST_TOKEN_KEY)).toBeNull();
  });
});
