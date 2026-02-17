import { useAuthStore } from '@/stores/authStore';
import { useNavigate } from 'react-router-dom';

export function UserMenu() {
  const { user, isAuthenticated, logout } = useAuthStore();
  const navigate = useNavigate();

  const handleLogout = async () => {
    await logout();
    navigate('/login');
  };

  if (!isAuthenticated) {
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
