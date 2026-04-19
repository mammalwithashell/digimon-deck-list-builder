import { Navigate, Outlet } from 'react-router-dom';
import { useAuthStore } from '@/stores/authStore';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

export function AuthGuard() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated);
  if (isAuthenticated) {
    return <Outlet />;
  }
  if (IS_DESKTOP) {
    // Desktop: hydrate mints a guest session on boot; if it failed (offline),
    // fall through to home so the offline banner can render.
    return <Navigate to="/" replace />;
  }
  return <Navigate to="/login" replace />;
}
