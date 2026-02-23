import { Link, useLocation } from 'react-router-dom';
import { UserMenu } from '@/components/auth/UserMenu';
import { useAuthStore } from '@/stores/authStore';

const NAV_LINKS = [
  { to: '/', label: 'Home' },
  { to: '/deckbuilder', label: 'Deck Builder' },
  { to: '/game', label: 'Play' },
];

const ADMIN_LINKS = [
  { to: '/admin/issues', label: 'Issues' },
  { to: '/admin/tasks', label: 'AI Tasks' },
  { to: '/admin/promotions', label: 'Promotions' },
];

export function NavBar() {
  const location = useLocation();
  const isAdmin = useAuthStore((s) => s.user?.roles?.includes('admin') ?? false);
  const links = isAdmin ? [...NAV_LINKS, ...ADMIN_LINKS] : NAV_LINKS;

  return (
    <nav className="bg-gray-800 border-b border-gray-700 px-4 py-3 flex items-center justify-between">
      <div className="flex items-center gap-6">
        <Link to="/" className="text-lg font-bold text-blue-400 hover:text-blue-300">
          Digimon TCG
        </Link>
        <div className="flex gap-4">
          {links.map((link) => (
            <Link
              key={link.to}
              to={link.to}
              className={`text-sm transition-colors ${
                location.pathname === link.to
                  ? 'text-white font-medium'
                  : 'text-gray-400 hover:text-gray-200'
              }`}
            >
              {link.label}
            </Link>
          ))}
        </div>
      </div>
      <UserMenu />
    </nav>
  );
}
