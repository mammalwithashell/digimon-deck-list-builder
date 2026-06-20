import { Fragment, useEffect, useState } from 'react';
import { Outlet, useLocation, useNavigate } from 'react-router-dom';
import { NavRail, NavRailItem } from '@/design/components/NavRail';
import { ThemeSwitch } from '@/design/components/ThemeSwitch';
import { useAppVersion } from '@/hooks/useAppVersion';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import * as deckStore from '@/storage/deckStore';
import type { DeckSummary } from '@/types/deck';
import { isActivePath } from '@/utils/navActive';
import { RailProvider, useRailSectionSlot } from './RailContext';
import './MenuShell.css';

interface NavItem {
  to: string;
  label: string;
  /** Single-character mark shown in the collapsed icon-rail and as the
   *  leading glyph when expanded. */
  glyph: string;
  exact?: boolean;
  /** Optional right-aligned count (e.g. saved-deck total on Decks). */
  badge?: number;
}

interface NavGroup {
  key: string;
  title: string;
  items: NavItem[];
}

/**
 * Persistent desktop navigation shell wrapping every menu route (Home,
 * Play, Decks, Patch Notes, Graphics, Models). Built from the design-system
 * `NavRail` so the active item lights up in `--accent` (neon green in dark,
 * navy in light) exactly like the style guide. The in-game board route is
 * intentionally NOT wrapped by this shell — it renders full-bleed.
 *
 * Active state is derived from the current route, never page-managed, so it
 * always reflects the URL. A page may contribute contextual sub-navigation
 * (e.g. the deck library's Folders/Formats) through `RailContext`; that
 * sub-nav renders beneath the active item and disappears when the page
 * unmounts.
 */
export function MenuShell() {
  return (
    <RailProvider>
      <MenuShellInner />
    </RailProvider>
  );
}

function MenuShellInner() {
  const location = useLocation();
  const navigate = useNavigate();
  const buildVersion = useAppVersion();
  const username = useAuthStore((s) => s.user?.username ?? 'Guest');
  const collapsed = useUiStore((s) => s.railCollapsed);
  const toggleRail = useUiStore((s) => s.toggleRail);
  const railSection = useRailSectionSlot();

  const [decks, setDecks] = useState<DeckSummary[]>([]);
  useEffect(() => {
    let active = true;
    void deckStore
      .listDecks()
      .then((list) => {
        if (active) setDecks(list);
      })
      .catch(() => {
        /* best-effort — the badge/foot just show 0 */
      });
    return () => {
      active = false;
    };
  }, [location.key]);

  const deckCount = decks.length;
  const draftCount = decks.filter((d) => !d.is_valid).length;

  const groups: NavGroup[] = [
    {
      key: 'main',
      title: 'Main',
      items: [
        { to: '/', label: 'Home', glyph: 'H', exact: true },
        { to: '/play', label: 'Play', glyph: 'P' },
        { to: '/deckbuilder', label: 'Decks', glyph: 'D', badge: deckCount },
        { to: '/patch-notes', label: 'Patch Notes', glyph: 'N' },
      ],
    },
    {
      key: 'tools',
      title: 'Tools',
      items: [
        { to: '/models', label: 'AI Models', glyph: 'M' },
        { to: '/settings/graphics', label: 'Graphics', glyph: 'G' },
      ],
    },
  ];

  return (
    <div className="menu-shell" data-collapsed={collapsed ? 'true' : 'false'}>
      <NavRail className="menu-shell__rail" aria-label="Main navigation">
        <div className="menu-shell__railtop">
          <button
            type="button"
            className="menu-shell__brand"
            onClick={() => navigate('/')}
            aria-label="Home"
          >
            <span className="menu-shell__brand-mark" aria-hidden="true">
              D
            </span>
            {!collapsed && (
              <span className="menu-shell__brand-name">
                DIGIMON<small>TCG</small>
              </span>
            )}
          </button>
          <button
            type="button"
            className="menu-shell__collapse"
            onClick={toggleRail}
            aria-pressed={collapsed}
            aria-label={collapsed ? 'Expand navigation' : 'Collapse navigation'}
            title={collapsed ? 'Expand' : 'Collapse'}
          >
            <span className="menu-shell__glyph" aria-hidden="true">
              {collapsed ? '»' : '«'}
            </span>
          </button>
        </div>

        <div className="menu-shell__nav">
          {groups.map((group) => (
            <div className="menu-shell__group" key={group.key}>
              {!collapsed && <h5>{group.title}</h5>}
              {group.items.map((item) => {
                const active = isActivePath(location.pathname, item);
                return (
                  <Fragment key={item.to}>
                    <NavRailItem
                      active={active}
                      onClick={() => navigate(item.to)}
                      title={collapsed ? item.label : undefined}
                    >
                      <span className="menu-shell__glyph" aria-hidden="true">
                        {item.glyph}
                      </span>
                      {!collapsed && (
                        <span className="menu-shell__label">{item.label}</span>
                      )}
                      {!collapsed && item.badge !== undefined && (
                        <span className="menu-shell__badge">
                          {String(item.badge).padStart(2, '0')}
                        </span>
                      )}
                    </NavRailItem>
                    {active && !collapsed && railSection ? (
                      <div className="menu-shell__subnav">{railSection}</div>
                    ) : null}
                  </Fragment>
                );
              })}
            </div>
          ))}
        </div>

        <div className="menu-shell__rail-foot">
          {!collapsed && (
            <div className="menu-shell__account">
              <span className="menu-shell__user">Signed in as {username}</span>
              <ThemeSwitch />
            </div>
          )}
          {!collapsed && (
            <div className="menu-shell__stats">
              <div>
                <span>BUILD</span>
                <b>{buildVersion}</b>
              </div>
              <div>
                <span>DECKS</span>
                <b>{String(deckCount).padStart(2, '0')}</b>
              </div>
              <div>
                <span>DRAFTS</span>
                <b>{String(draftCount).padStart(2, '0')}</b>
              </div>
            </div>
          )}
        </div>
      </NavRail>

      <main className="menu-shell__main">
        <div className="menu-shell__content">
          <Outlet />
        </div>
        <div className="menu-shell__statusbar">
          <span>DESKTOP</span>
        </div>
      </main>
    </div>
  );
}
