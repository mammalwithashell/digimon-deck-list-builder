import { create } from 'zustand';
import * as authApi from '@/api/authApi';
import type { User } from '@/types/auth';

interface AuthState {
  user: User | null;
  accessToken: string | null;
  refreshToken: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;

  login: (username: string, password: string) => Promise<void>;
  register: (username: string, email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  hydrate: () => Promise<void>;
}

export const useAuthStore = create<AuthState>((set, get) => ({
  user: null,
  accessToken: localStorage.getItem('access_token'),
  refreshToken: localStorage.getItem('refresh_token'),
  isAuthenticated: !!localStorage.getItem('access_token'),
  isLoading: false,
  error: null,

  login: async (username, password) => {
    set({ isLoading: true, error: null });
    try {
      const tokens = await authApi.login(username, password);
      localStorage.setItem('access_token', tokens.access_token);
      localStorage.setItem('refresh_token', tokens.refresh_token);
      let user: User | null = null;
      try {
        user = await authApi.getMe();
      } catch {
        user = { id: '', username, email: '', roles: [] };
      }
      set({
        accessToken: tokens.access_token,
        refreshToken: tokens.refresh_token,
        isAuthenticated: true,
        isLoading: false,
        user,
      });
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : 'Login failed';
      set({ error: message, isLoading: false });
      throw err;
    }
  },

  register: async (username, email, password) => {
    set({ isLoading: true, error: null });
    try {
      const user = await authApi.register(username, email, password);
      set({ user, isLoading: false });
    } catch (err: unknown) {
      const message =
        err instanceof Error ? err.message : 'Registration failed';
      set({ error: message, isLoading: false });
      throw err;
    }
  },

  logout: async () => {
    const { refreshToken } = get();
    if (refreshToken) {
      try {
        await authApi.logout(refreshToken);
      } catch {
        // Ignore logout failures
      }
    }
    localStorage.removeItem('access_token');
    localStorage.removeItem('refresh_token');
    set({
      user: null,
      accessToken: null,
      refreshToken: null,
      isAuthenticated: false,
    });
  },

  hydrate: async () => {
    const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

    if (IS_DESKTOP) {
      // Desktop alpha: mint/reuse a guest session. No account flows.
      const { ensureGuestSession } = await import('@/bootstrap/guest');
      try {
        const session = await ensureGuestSession();
        // The existing axios interceptor reads `access_token`; mirror the
        // guest token there so authenticated calls "just work" without
        // rewriting the interceptor.
        localStorage.setItem('access_token', session.token);
        set({
          accessToken: session.token,
          refreshToken: null,
          isAuthenticated: true,
          user: {
            id: session.userId,
            username: session.displayName,
            email: '',
            roles: [],
          },
        });
      } catch {
        // Offline: HomePage will render an offline banner and disable PvP.
        localStorage.removeItem('access_token');
        set({
          accessToken: null,
          refreshToken: null,
          isAuthenticated: false,
          user: null,
        });
      }
      return;
    }

    // Web build (unchanged)
    const accessToken = localStorage.getItem('access_token');
    const refreshToken = localStorage.getItem('refresh_token');
    let user: User | null = null;
    if (accessToken) {
      try {
        user = await authApi.getMe();
      } catch {
        user = null;
      }
    }
    set({
      accessToken,
      refreshToken,
      isAuthenticated: !!accessToken,
      user,
    });
  },
}));
