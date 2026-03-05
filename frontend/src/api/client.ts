import axios from 'axios';

const API_BASE = import.meta.env.VITE_API_URL ?? '/api';

// Desktop mode detection: Tauri injects __TAURI__ into the window object.
// When running as a Tauri desktop app, local game engine requests go to
// the bundled Python sidecar; online features go to the remote server.
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;
const LOCAL_SIDECAR_URL = 'http://localhost:8321';

/** Remote server client — handles auth, PvP, lobby, user data */
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
      }
    }
    return Promise.reject(error);
  },
);

/** Local sidecar client — game engine only, no auth needed (Tauri desktop) */
const localClient = axios.create({
  baseURL: LOCAL_SIDECAR_URL,
  headers: { 'Content-Type': 'application/json' },
});

/**
 * Get the appropriate client for a given feature.
 * In Tauri desktop mode, local game operations use the sidecar;
 * everything else goes to the remote server.
 */
export function getGameClient(isLocalGame: boolean = true) {
  return isTauri && isLocalGame ? localClient : client;
}

export { isTauri, LOCAL_SIDECAR_URL };
export default client;
