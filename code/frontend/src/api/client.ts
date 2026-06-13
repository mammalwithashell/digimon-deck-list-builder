import axios from 'axios';
import { isGuestToken, remintGuestSession } from '@/bootstrap/guest';

const API_BASE = import.meta.env.VITE_API_URL ?? '/api';

/** Remote server client — handles auth, PvP, lobby, user data. */
const client = axios.create({
  baseURL: API_BASE,
  headers: { 'Content-Type': 'application/json' },
});

client.interceptors.request.use((config) => {
  const token = localStorage.getItem('access_token');
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

client.interceptors.response.use(
  (response) => response,
  async (error) => {
    const original = error.config as typeof error.config & { _retry?: boolean };
    if (error.response?.status === 401 && !original._retry) {
      original._retry = true;
      const refreshToken = localStorage.getItem('refresh_token');
      if (refreshToken) {
        try {
          const { data } = await axios.post(`${API_BASE}/auth/refresh`, {
            refresh_token: refreshToken,
          });
          localStorage.setItem('access_token', data.access_token);
          localStorage.setItem('refresh_token', data.refresh_token);
          original.headers.Authorization = `Bearer ${data.access_token}`;
          return client(original);
        } catch {
          localStorage.removeItem('access_token');
          localStorage.removeItem('refresh_token');
          window.location.href = '/login';
        }
      } else if (isGuestToken(localStorage.getItem('access_token'))) {
        // Guest identities own nothing server-side, so a stale/poisoned
        // guest token is recovered by minting a fresh guest and retrying
        // once. `_retry` guarantees a second 401 surfaces to the caller
        // instead of looping on mints.
        try {
          const session = await remintGuestSession();
          localStorage.setItem('access_token', session.token);
          original.headers.Authorization = `Bearer ${session.token}`;
          // Keep the visible identity (UserMenu) in sync with the new
          // guest. Dynamic import avoids a static client→store→client cycle.
          void import('@/stores/authStore').then(({ useAuthStore }) => {
            useAuthStore.setState({
              accessToken: session.token,
              isAuthenticated: true,
              user: { id: session.userId, username: session.displayName, email: '', roles: [] },
            });
          });
          return client(original);
        } catch {
          // Mint failed (offline) — fall through to the original 401.
        }
      }
    }
    return Promise.reject(error);
  },
);

/**
 * Client for game-engine REST calls. On desktop builds, `gameApi.ts` /
 * `deckApi.ts` short-circuit to Tauri `invoke()` before this is reached,
 * so the HTTP client only runs in web builds pointing at the hosted API.
 */
export function getGameClient() {
  return client;
}

export default client;
