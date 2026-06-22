import { useEffect, useRef, useState, type ReactNode } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTheme } from '@/design/theme/ThemeProvider';
import { useUiStore } from '@/stores/uiStore';
import { GRAPHICS_MODAL_ID } from '@/components/settings/GraphicsSettingsModal';

/**
 * Custom desktop window chrome (replaces the native OS title bar, which is
 * disabled via `"decorations": false` in tauri.conf.json).
 *
 * Two-row layout, themed entirely in TitleBar.css keyed on `[data-theme]`:
 *   - caption row  — draggable, app title + minimize/close controls
 *       light → Windows-95 navy→blue gradient + raised beveled buttons
 *       dark  → terminal surface + neon-green title and a glowing hairline
 *   - menu strip   — File / View / Help dropdown menus (VS Code style)
 *
 * Rendered globally and OUTSIDE the CanvasScaler so it lives in real,
 * unscaled window pixels; the scaler reserves `TITLEBAR_HEIGHT` for it.
 *
 * Window operations go through Tauri's window API (dynamically imported so
 * the dependency never lands in the web bundle). When not running under
 * Tauri — e.g. `npm run dev:desktop` opened in a plain browser for a quick
 * visual demo — the drag region and the min/close buttons are inert.
 */

const IS_TAURI =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

/** Resolve the current Tauri window handle lazily; keeps
 *  `@tauri-apps/api/window` out of the web bundle. */
async function withTauriWindow(fn: (w: WindowApi) => Promise<void> | void) {
  if (!IS_TAURI) return;
  try {
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await fn(getCurrentWindow() as unknown as WindowApi);
  } catch {
    // Not under Tauri or the window API failed — no-op (e.g. browser dev).
  }
}

interface WindowApi {
  minimize: () => Promise<void>;
  close: () => Promise<void>;
}

type MenuItem =
  | { kind: 'separator' }
  | { kind: 'item'; label: string; accel?: string; onSelect: () => void };

interface Menu {
  label: string;
  items: MenuItem[];
}

export function TitleBar() {
  const navigate = useNavigate();
  const location = useLocation();
  const { theme, toggle } = useTheme();
  const fullscreen = useUiStore((s) => s.fullscreen);
  const setFullscreen = useUiStore((s) => s.setFullscreen);
  const openModal = useUiStore((s) => s.openModal);

  // Back/forward availability from React Router's history index (stored on
  // `window.history.state.idx`). Recomputed on every navigation.
  const [nav, setNav] = useState({ back: false, forward: false });
  useEffect(() => {
    const idx =
      typeof window !== 'undefined' &&
      typeof window.history.state?.idx === 'number'
        ? (window.history.state.idx as number)
        : 0;
    const len = typeof window !== 'undefined' ? window.history.length : 0;
    setNav({ back: idx > 0, forward: idx < len - 1 });
  }, [location]);

  const menus: Menu[] = [
    {
      label: 'File',
      items: [
        { kind: 'item', label: 'New Game', onSelect: () => navigate('/play') },
        {
          kind: 'item',
          label: 'Deck Builder',
          onSelect: () => navigate('/deckbuilder'),
        },
        { kind: 'separator' },
        {
          kind: 'item',
          label: 'Exit',
          onSelect: () => void withTauriWindow((w) => w.close()),
        },
      ],
    },
    {
      label: 'View',
      items: [
        {
          kind: 'item',
          label: theme === 'dark' ? 'Light Theme' : 'Dark Theme',
          onSelect: toggle,
        },
        {
          kind: 'item',
          label: 'Toggle Fullscreen',
          onSelect: () => setFullscreen(!fullscreen),
        },
        {
          kind: 'item',
          label: 'Graphics Settings…',
          onSelect: () => openModal(GRAPHICS_MODAL_ID),
        },
      ],
    },
    {
      label: 'Help',
      items: [
        {
          kind: 'item',
          label: 'Patch Notes',
          onSelect: () => navigate('/patch-notes'),
        },
        {
          kind: 'item',
          label: 'About',
          onSelect: () =>
            window.alert('Digimon TCG Simulator\nDesktop alpha v0.4.0'),
        },
      ],
    },
  ];

  const navButtons = (
    <div className="ds-titlebar__nav" role="group" aria-label="History navigation">
      <button
        type="button"
        className="ds-titlebar__navbtn"
        aria-label="Back"
        title="Back"
        disabled={!nav.back}
        onClick={() => navigate(-1)}
      >
        ←
      </button>
      <button
        type="button"
        className="ds-titlebar__navbtn"
        aria-label="Forward"
        title="Forward"
        disabled={!nav.forward}
        onClick={() => navigate(1)}
      >
        →
      </button>
    </div>
  );

  return (
    <header className="ds-titlebar" data-testid="titlebar">
      <div className="ds-titlebar__caption">
        <span className="ds-titlebar__brand" data-tauri-drag-region>
          Digimon TCG
        </span>
        <div className="ds-titlebar__drag" data-tauri-drag-region />
        <div className="ds-titlebar__controls">
          <button
            type="button"
            className="ds-titlebar__ctl ds-titlebar__ctl--min"
            aria-label="Minimize"
            title="Minimize"
            onClick={() => void withTauriWindow((w) => w.minimize())}
          >
            <span className="ds-titlebar__glyph-min" aria-hidden="true" />
          </button>
          <button
            type="button"
            className="ds-titlebar__ctl ds-titlebar__ctl--close"
            aria-label="Close"
            title="Close"
            onClick={() => void withTauriWindow((w) => w.close())}
          >
            <span aria-hidden="true">✕</span>
          </button>
        </div>
      </div>
      <MenuBar menus={menus} leading={navButtons} />
    </header>
  );
}

function MenuBar({ menus, leading }: { menus: Menu[]; leading?: ReactNode }) {
  const [openIndex, setOpenIndex] = useState<number | null>(null);
  const navRef = useRef<HTMLElement>(null);

  // While a menu is open, close it on outside click or Escape.
  useEffect(() => {
    if (openIndex === null) return;
    const onDown = (e: MouseEvent) => {
      if (navRef.current && !navRef.current.contains(e.target as Node)) {
        setOpenIndex(null);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpenIndex(null);
    };
    document.addEventListener('mousedown', onDown);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onDown);
      document.removeEventListener('keydown', onKey);
    };
  }, [openIndex]);

  const select = (run: () => void) => {
    setOpenIndex(null);
    run();
  };

  return (
    <nav className="ds-titlebar__menubar" ref={navRef} role="menubar">
      {leading}
      {menus.map((menu, i) => {
        const open = openIndex === i;
        return (
          <div className="ds-menu" key={menu.label}>
            <button
              type="button"
              className={`ds-menu__btn${open ? ' is-open' : ''}`}
              aria-haspopup="menu"
              aria-expanded={open}
              onClick={() => setOpenIndex(open ? null : i)}
              // Hovering a sibling while a menu is open switches to it,
              // matching native menu-bar behavior.
              onMouseEnter={() => openIndex !== null && setOpenIndex(i)}
            >
              {menu.label}
            </button>
            {open && (
              <ul className="ds-menu__dropdown" role="menu">
                {menu.items.map((item, j): ReactNode =>
                  item.kind === 'separator' ? (
                    <li
                      key={`sep-${j}`}
                      className="ds-menu__sep"
                      role="separator"
                    />
                  ) : (
                    <li key={item.label} role="none">
                      <button
                        type="button"
                        role="menuitem"
                        className="ds-menu__item"
                        onClick={() => select(item.onSelect)}
                      >
                        <span>{item.label}</span>
                        {item.accel && (
                          <span className="ds-menu__accel">{item.accel}</span>
                        )}
                      </button>
                    </li>
                  ),
                )}
              </ul>
            )}
          </div>
        );
      })}
    </nav>
  );
}
