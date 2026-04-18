import { useAuthStore } from '@/stores/authStore';
import { useNavigate } from 'react-router-dom';
import {
  GUEST_TOKEN_KEY,
  GUEST_USER_ID_KEY,
  GUEST_NAME_KEY,
} from '@/bootstrap/guest';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

export function UserMenu() {
  const { user, isAuthenticated, logout } = useAuthStore();
  const navigate = useNavigate();

  const handleWebLogout = async () => {
    await logout();
    navigate('/login');
  };

  const handleDesktopLogout = async () => {
    // Clear the guest identity + token mirror, then reload. On mount the
    // authStore hydrate() re-runs ensureGuestSession(), which finds no
    // cached keys and mints a fresh guest — effectively "new game as a
    // new guest" rather than dead-ending at /login.
    localStorage.removeItem(GUEST_TOKEN_KEY);
    localStorage.removeItem(GUEST_USER_ID_KEY);
    localStorage.removeItem(GUEST_NAME_KEY);
    localStorage.removeItem('access_token');
    window.location.reload();
  };

  const handleLogout = IS_DESKTOP ? handleDesktopLogout : handleWebLogout;

  if (!isAuthenticated) {
    if (IS_DESKTOP) {
      // Desktop uses guest sessions; no login UI.
      return null;
    }
    return (
      <button
        onClick={() => navigate('/login')}
        className="text-sm text-gray-300 hover:text-white transition-colors"
      >
        Log In
      </button>
    );
  }

  return (
    <div className="flex items-center gap-3">
      <span className="text-sm text-gray-300">{user?.username ?? 'User'}</span>
      <button
        onClick={handleLogout}
        className="text-sm text-gray-400 hover:text-red-400 transition-colors"
      >
        Logout
      </button>
    </div>
  );
}
