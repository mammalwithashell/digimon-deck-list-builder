import { useEffect, useRef, useState } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { UserMenu } from '@/components/auth/UserMenu';
import { useAuthStore } from '@/stores/authStore';

type NavItem = {
  to: string;
  label: string;
  exact?: boolean;
};

type NavGroup = {
  key: string;
  title: string;
  adminOnly?: boolean;
  items: NavItem[];
};

const NAV_GROUPS: NavGroup[] = [
  {
    key: 'home',
    title: 'Overview',
    items: [{ to: '/', label: 'Home', exact: true }],
  },
  {
    key: 'deck-play',
    title: 'Gameplay',
    items: [
      { to: '/deckbuilder', label: 'Deck Builder' },
      { to: '/game', label: 'Play' },
    ],
  },
  {
    key: 'admin-ops',
    title: 'AI Ops',
    adminOnly: true,
    items: [
      { to: '/admin/issues', label: 'Issues' },
      { to: '/admin/tasks', label: 'AI Tasks' },
      { to: '/admin/promotions', label: 'Promotions' },
    ],
  },
  {
    key: 'admin-training',
    title: 'Training',
    adminOnly: true,
    items: [
      { to: '/admin/barracks', label: 'Barracks' },
      { to: '/admin/arena', label: 'Arena' },
      { to: '/admin/gauntlet', label: 'Gauntlet' },
    ],
  },
];

function isActivePath(pathname: string, item: NavItem): boolean {
  if (item.exact) return pathname === item.to;
  return pathname === item.to || pathname.startsWith(`${item.to}/`);
}

function isGroupActive(pathname: string, group: NavGroup): boolean {
  return group.items.some((item) => isActivePath(pathname, item));
}

export function NavBar() {
  const location = useLocation();
  const navRef = useRef<HTMLDivElement | null>(null);
  const [openGroup, setOpenGroup] = useState<string | null>(null);
  const isAdmin = useAuthStore((s) => s.user?.roles?.includes('admin') ?? false);
  const groups = NAV_GROUPS.filter((group) => !group.adminOnly || isAdmin);

  useEffect(() => {
    setOpenGroup(null);
  }, [location.pathname]);

  useEffect(() => {
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) return;
      if (navRef.current && !navRef.current.contains(target)) {
        setOpenGroup(null);
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setOpenGroup(null);
      }
    };
    document.addEventListener('mousedown', onPointerDown);
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('mousedown', onPointerDown);
      document.removeEventListener('keydown', onKeyDown);
    };
  }, []);

  return (
    <nav ref={navRef} className="bg-gray-800 border-b border-gray-700 px-4 py-3 flex items-center justify-between gap-4">
      <div className="flex items-center gap-6 min-w-0">
        <Link to="/" className="text-lg font-bold text-blue-400 hover:text-blue-300">
          Digimon TCG
        </Link>
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2 min-w-0">
          {groups.map((group) => {
            const active = isGroupActive(location.pathname, group);
            const open = openGroup === group.key;
            return (
              <div key={group.key} className="relative">
                <button
                  type="button"
                  onClick={() => setOpenGroup((prev) => (prev === group.key ? null : group.key))}
                  className={`inline-flex items-center gap-2 rounded-md border px-3 py-1.5 text-sm transition-colors ${
                    active
                      ? 'border-blue-500 text-white bg-blue-500/10'
                      : 'border-gray-600 text-gray-300 hover:text-white hover:border-gray-500'
                  }`}
                >
                  <span>{group.title}</span>
                  <span className={`text-xs transition-transform ${open ? 'rotate-180' : ''}`}>v</span>
                </button>

                {open ? (
                  <div className="absolute left-0 top-full z-50 mt-2 min-w-[13rem] rounded-md border border-gray-700 bg-gray-900 shadow-lg">
                    {group.items.map((item) => (
                      <Link
                        key={item.to}
                        to={item.to}
                        onClick={() => setOpenGroup(null)}
                        className={`block px-3 py-2 text-sm transition-colors ${
                          isActivePath(location.pathname, item)
                            ? 'text-white bg-gray-800'
                            : 'text-gray-300 hover:bg-gray-800 hover:text-white'
                        }`}
                      >
                        {item.label}
                      </Link>
                    ))}
                  </div>
                ) : null}
              </div>
            );
          })}
        </div>
      </div>
      <UserMenu />
    </nav>
  );
}
