/**
 * Anonymous guest-session bootstrap for the desktop alpha build.
 *
 * On first launch, POST /auth/guest and cache the JWT + display name in
 * localStorage. On subsequent launches, the cached token is reused.
 *
 * Policy: we never silently re-mint a token on 401. Losing the guest
 * user_id silently mid-session would be surprising; if a 401 comes back
 * for a decodable token, the caller surfaces it as an auth error.
 */

export const GUEST_TOKEN_KEY = 'guest_access_token';
export const GUEST_USER_ID_KEY = 'guest_user_id';
export const GUEST_NAME_KEY = 'guest_display_name';
const OFFLINE_GUEST_SESSION: GuestSession = {
  token: 'offline-guest-token',
  userId: 'offline_guest',
  displayName: 'Guest',
};

export interface GuestSession {
  token: string;
  userId: string;
  displayName: string;
}

const API_BASE = (import.meta.env.VITE_API_URL as string | undefined) ?? '';

interface GuestResponse {
  access_token: string;
  user_id: string;
  display_name: string;
}

function persistGuestSession(session: GuestSession): GuestSession {
  try {
    localStorage.setItem(GUEST_TOKEN_KEY, session.token);
    localStorage.setItem(GUEST_USER_ID_KEY, session.userId);
    localStorage.setItem(GUEST_NAME_KEY, session.displayName);
  } catch (e) {
    // Partial writes would orphan the guest identity on next boot; if any
    // key write fails, roll back all three so the next launch starts clean.
    localStorage.removeItem(GUEST_TOKEN_KEY);
    localStorage.removeItem(GUEST_USER_ID_KEY);
    localStorage.removeItem(GUEST_NAME_KEY);
    throw new Error(`Failed to persist guest session: ${e instanceof Error ? e.message : String(e)}`);
  }
  return session;
}

export async function ensureGuestSession(): Promise<GuestSession> {
  const cachedToken = localStorage.getItem(GUEST_TOKEN_KEY);
  const cachedId = localStorage.getItem(GUEST_USER_ID_KEY);
  const cachedName = localStorage.getItem(GUEST_NAME_KEY);
  if (cachedToken && cachedId && cachedName) {
    return { token: cachedToken, userId: cachedId, displayName: cachedName };
  }

  try {
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
  } catch {
    // Desktop must remain navigable when the hosted API is offline. PvP and
    // account-backed calls can still fail individually, but local routes such
    // as play setup, deck organization, import, and bot practice should open.
    return persistGuestSession(OFFLINE_GUEST_SESSION);
  }
}
