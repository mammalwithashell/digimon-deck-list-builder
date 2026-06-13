import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  ensureGuestSession,
  isGuestToken,
  remintGuestSession,
  GUEST_TOKEN_KEY,
  GUEST_USER_ID_KEY,
  GUEST_NAME_KEY,
  OFFLINE_GUEST_TOKEN,
} from './guest';

function guestJwt(payload: Record<string, unknown>): string {
  const b64 = (obj: Record<string, unknown>) =>
    btoa(JSON.stringify(obj)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  return `${b64({ alg: 'HS256', typ: 'JWT' })}.${b64(payload)}.sig`;
}

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

  it('falls back to an in-memory offline session without persisting it', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false, status: 500, text: async () => 'boom',
    }) as unknown as typeof fetch;

    const session = await ensureGuestSession();

    expect(session.offline).toBe(true);
    expect(session.token).toBe(OFFLINE_GUEST_TOKEN);
    // Nothing persisted — the next launch retries a real mint.
    expect(localStorage.getItem(GUEST_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(GUEST_USER_ID_KEY)).toBeNull();
    expect(localStorage.getItem(GUEST_NAME_KEY)).toBeNull();
  });

  it('migrates a persisted offline sentinel by minting a real guest', async () => {
    // Older builds persisted the offline sentinel, poisoning every launch.
    localStorage.setItem(GUEST_TOKEN_KEY, OFFLINE_GUEST_TOKEN);
    localStorage.setItem(GUEST_USER_ID_KEY, 'offline_guest');
    localStorage.setItem(GUEST_NAME_KEY, 'Guest');
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 201,
      json: async () => ({
        access_token: 'fresh-jwt',
        user_id: 'guest_fresh',
        display_name: 'Guest-FRSH',
      }),
    }) as unknown as typeof fetch;

    const session = await ensureGuestSession();

    expect(session.token).toBe('fresh-jwt');
    expect(localStorage.getItem(GUEST_TOKEN_KEY)).toBe('fresh-jwt');
    expect(globalThis.fetch).toHaveBeenCalledTimes(1);
  });

  it('rolls back all guest keys if a setItem throws mid-sequence', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 201,
      json: async () => ({
        access_token: 'jwt',
        user_id: 'guest_partial',
        display_name: 'Guest-PART',
      }),
    }) as unknown as typeof fetch;

    const realSetItem = Storage.prototype.setItem;
    let call = 0;
    const spy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(function (
      this: Storage,
      key: string,
      value: string,
    ) {
      call += 1;
      if (call === 3) {
        throw new DOMException('QuotaExceededError', 'QuotaExceededError');
      }
      return realSetItem.call(this, key, value);
    });

    await expect(ensureGuestSession()).rejects.toThrow(/Failed to persist guest session/);

    expect(localStorage.getItem(GUEST_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(GUEST_USER_ID_KEY)).toBeNull();
    expect(localStorage.getItem(GUEST_NAME_KEY)).toBeNull();

    spy.mockRestore();
  });
});

describe('remintGuestSession', () => {
  const originalFetch = globalThis.fetch;
  beforeEach(() => {
    localStorage.clear();
  });
  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it('discards the cached identity and mints a fresh one', async () => {
    localStorage.setItem(GUEST_TOKEN_KEY, 'stale');
    localStorage.setItem(GUEST_USER_ID_KEY, 'guest_stale');
    localStorage.setItem(GUEST_NAME_KEY, 'Guest-OLDD');
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 201,
      json: async () => ({
        access_token: 'new-jwt',
        user_id: 'guest_new',
        display_name: 'Guest-NEWW',
      }),
    }) as unknown as typeof fetch;

    const session = await remintGuestSession();

    expect(session.token).toBe('new-jwt');
    expect(localStorage.getItem(GUEST_TOKEN_KEY)).toBe('new-jwt');
    expect(localStorage.getItem(GUEST_NAME_KEY)).toBe('Guest-NEWW');
  });

  it('throws (no offline fallback) when the mint fails', async () => {
    localStorage.setItem(GUEST_TOKEN_KEY, 'stale');
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('network down'));

    await expect(remintGuestSession()).rejects.toThrow();
    // Stale identity is gone either way — next launch starts clean.
    expect(localStorage.getItem(GUEST_TOKEN_KEY)).toBeNull();
  });
});

describe('isGuestToken', () => {
  it('recognizes the offline sentinel', () => {
    expect(isGuestToken(OFFLINE_GUEST_TOKEN)).toBe(true);
  });

  it('recognizes a JWT with the is_guest claim', () => {
    expect(isGuestToken(guestJwt({ sub: 'guest_x', is_guest: true }))).toBe(true);
  });

  it('rejects registered-user JWTs, malformed tokens, and null', () => {
    expect(isGuestToken(guestJwt({ sub: 'user_x' }))).toBe(false);
    expect(isGuestToken('not-a-jwt')).toBe(false);
    expect(isGuestToken(null)).toBe(false);
  });
});
