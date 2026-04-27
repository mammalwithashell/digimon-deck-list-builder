import { Link } from 'react-router-dom';
import type { ReactNode } from 'react';

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
  return (
    <div className="launcher-screen">
      <div className="launcher-frame">
        <div className="launcher-titlebar">
          <div className="launcher-dots" aria-hidden="true">
            <span className="launcher-dot launcher-dot-red" />
            <span className="launcher-dot launcher-dot-yellow" />
            <span className="launcher-dot launcher-dot-green" />
          </div>
          <div className="launcher-window-title">DIGIMON TCG DESKTOP · v{buildVersion}</div>
          <div className="launcher-titlebar-right">
            <span className={serverHealthy ? 'launcher-pill-live' : 'launcher-pill-offline'}>
              {serverHealthy ? 'CONNECTED' : 'OFFLINE'}
            </span>
          </div>
        </div>
        <div className="launcher-body-area">
          <aside className="launcher-side">
            <Link className="launcher-brand" to="/">
              <span className="launcher-brand-mark" aria-hidden="true">D</span>
              <span className="launcher-brand-name">DIGIMON<small>TCG</small></span>
            </Link>
            <nav className="launcher-side-section" aria-label="Main">
              <h5>Main</h5>
              <Link className="launcher-nav-item active" to="/">Home</Link>
              <Link className="launcher-nav-item" to="/game">Play</Link>
              <Link className="launcher-nav-item" to="/deckbuilder">Decks <span>{String(deckCount).padStart(2, '0')}</span></Link>
              <Link className="launcher-nav-item" to="/patch-notes">Patch Notes</Link>
            </nav>
            <nav className="launcher-side-section" aria-label="Tools">
              <h5>Tools</h5>
              <Link className="launcher-nav-item" to="/models">AI Models</Link>
              <Link className="launcher-nav-item" to="/lobby">Online Lobby</Link>
              <Link className="launcher-nav-item" to="/deckbuilder?import=1">Import</Link>
              <Link className="launcher-nav-item" to="/game">Sandbox</Link>
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
              <div className="launcher-user">Signed in as {username}</div>
            </div>
            <div className="launcher-content">{children}</div>
            <div className="launcher-statusbar">
              <span className={serverHealthy ? 'launcher-ok' : 'launcher-warn'}>
                {serverHealthy ? 'SERVER OK' : 'SERVER OFFLINE'}
              </span>
              <span>DESKTOP</span>
            </div>
          </main>
        </div>
      </div>
    </div>
  );
}
