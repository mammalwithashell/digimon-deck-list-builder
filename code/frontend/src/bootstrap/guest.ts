/**
 * Anonymous guest-session bootstrap for the desktop alpha build.
 *
 * On first launch, POST /auth/guest and cache the JWT + display name in
 * localStorage. On subsequent launches, the cached token is reused.
 *
 * Policy (add-room-match-pvp): the offline fallback session is NEVER
 * persisted — it lives in memory for the current launch only, so the next
 * launch retries a real mint. A 401 on a guest token is recovered by
 * re-minting a fresh guest (see remintGuestSession / api/client.ts):
 * guests own nothing server-side, so a new identity on auth failure is
 * free, whereas a permanently poisoned token bricks every PvP feature.
 */

export const GUEST_TOKEN_KEY = 'guest_access_token';
export const GUEST_USER_ID_KEY = 'guest_user_id';
export const GUEST_NAME_KEY = 'guest_display_name';
export const OFFLINE_GUEST_TOKEN = 'offline-guest-token';
const OFFLINE_GUEST_SESSION: GuestSession = {
  token: OFFLINE_GUEST_TOKEN,
  userId: 'offline_guest',
  displayName: 'Guest',
  offline: true,
};

export interface GuestSession {
  token: string;
  userId: string;
  displayName: string;
  /** True when the hosted API was unreachable and this is the in-memory
   * fallback identity. Never persisted; authed calls will 401. */
  offline?: boolean;
}

const API_BASE = (import.meta.env.VITE_API_URL as string | undefined) ?? '';

interface GuestResponse {
  access_token: string;
  user_id: string;
  display_name: string;
}

export function clearGuestSession(): void {
  localStorage.removeItem(GUEST_TOKEN_KEY);
  localStorage.removeItem(GUEST_USER_ID_KEY);
  localStorage.removeItem(GUEST_NAME_KEY);
}

/** True if the bearer token belongs to a guest identity — either the
 * offline sentinel or a JWT carrying the `is_guest` claim. */
export function isGuestToken(token: string | null): boolean {
  if (!token) return false;
  if (token === OFFLINE_GUEST_TOKEN) return true;
  const parts = token.split('.');
  if (parts.length !== 3 || !parts[1]) return false;
  try {
    const payload = JSON.parse(atob(parts[1].replace(/-/g, '+').replace(/_/g, '/')));
    return payload?.is_guest === true;
  } catch {
    return false;
  }
}

function persistGuestSession(session: GuestSession): GuestSession {
  try {
    localStorage.setItem(GUEST_TOKEN_KEY, session.token);
    localStorage.setItem(GUEST_USER_ID_KEY, session.userId);
    localStorage.setItem(GUEST_NAME_KEY, session.displayName);
  } catch (e) {
    // Partial writes would orphan the guest identity on next boot; if any
    // key write fails, roll back all three so the next launch starts clean.
    clearGuestSession();
    throw new Error(`Failed to persist guest session: ${e instanceof Error ? e.message : String(e)}`);
  }
  return session;
}

async function mintGuest(): Promise<GuestSession> {
  const resp = await fetch(`${API_BASE}/auth/guest`, { method: 'POST' });
  if (!resp.ok) {
    throw new Error(`Failed to mint guest session (${resp.status})`);
  }
  const body = (await resp.json()) as GuestResponse;
  return persistGuestSession({
    token: body.access_token,
    userId: body.user_id,
    displayName: body.display_name,
  });
}

/**
 * Discard the cached guest identity and mint a fresh one. Used by the 401
 * recovery path in api/client.ts. Throws if the mint fails — the caller
 * surfaces the original auth error instead of looping.
 */
export async function remintGuestSession(): Promise<GuestSession> {
  clearGuestSession();
  return mintGuest();
}

export async function ensureGuestSession(): Promise<GuestSession> {
  const cachedToken = localStorage.getItem(GUEST_TOKEN_KEY);
  const cachedId = localStorage.getItem(GUEST_USER_ID_KEY);
  const cachedName = localStorage.getItem(GUEST_NAME_KEY);
  if (cachedToken === OFFLINE_GUEST_TOKEN) {
    // Migration: older builds persisted the offline sentinel, permanently
    // poisoning every authed call. Treat it as absent and mint for real.
    clearGuestSession();
  } else if (cachedToken && cachedId && cachedName) {
    return { token: cachedToken, userId: cachedId, displayName: cachedName };
  }

  try {
    return await mintGuest();
  } catch (e) {
    if (e instanceof Error && e.message.startsWith('Failed to persist')) {
      throw e;
    }
    // Desktop must remain navigable when the hosted API is offline. PvP and
    // account-backed calls can still fail individually, but local routes such
    // as play setup, deck organization, import, and bot practice should open.
    // In-memory only — never persisted (see module policy above).
    return OFFLINE_GUEST_SESSION;
  }
}
