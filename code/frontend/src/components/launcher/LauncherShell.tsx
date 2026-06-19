import { Link } from 'react-router-dom';
import type { ReactNode } from 'react';
import { ThemeSwitch } from '@/design/components/ThemeSwitch';

interface LauncherShellProps {
  buildVersion: string;
  cardCountLabel: string;
  deckCount: number;
  draftCount: number;
  serverHealthy: boolean;
  username: string;
  children: ReactNode;
}

export function LauncherShell({
  buildVersion,
  cardCountLabel,
  deckCount,
  draftCount,
  serverHealthy,
  username,
  children,
}: LauncherShellProps) {
  void serverHealthy;
  return (
    <div className="launcher-screen">
      <div className="launcher-frame">
        <div className="launcher-body-area">
          <aside className="launcher-side">
            <Link className="launcher-brand" to="/">
              <span className="launcher-brand-mark" aria-hidden="true">D</span>
              <span className="launcher-brand-name">DIGIMON<small>TCG</small></span>
            </Link>
            <nav className="launcher-side-section" aria-label="Main">
              <h5>Main</h5>
              <Link className="launcher-nav-item active" to="/">Home</Link>
              <Link className="launcher-nav-item" to="/play">Play</Link>
              <Link className="launcher-nav-item" to="/deckbuilder">Decks <span>{String(deckCount).padStart(2, '0')}</span></Link>
              <Link className="launcher-nav-item" to="/patch-notes">Patch Notes</Link>
            </nav>
            <nav className="launcher-side-section" aria-label="Tools">
              <h5>Tools</h5>
              <Link className="launcher-nav-item" to="/deckbuilder/new?import=1">Import</Link>
              <Link className="launcher-nav-item" to="/settings/graphics">Graphics</Link>
            </nav>
            <div className="launcher-side-foot">
              <div><span>BUILD</span><b>{buildVersion}</b></div>
              <div><span>CARDS</span><b>{cardCountLabel}</b></div>
              <div><span>DRAFTS</span><b>{String(draftCount).padStart(2, '0')}</b></div>
            </div>
          </aside>
          <main className="launcher-main">
            <div className="launcher-topbar">
              <div><span>00</span><b> Launcher</b></div>
              <div className="launcher-topbar-meta">
                <div className="launcher-user">Signed in as {username}</div>
                <ThemeSwitch />
              </div>
            </div>
            <div className="launcher-content">{children}</div>
            <div className="launcher-statusbar">
              <span>DESKTOP</span>
            </div>
          </main>
        </div>
      </div>
    </div>
  );
}
