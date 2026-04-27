import { Link } from 'react-router-dom';

interface LauncherActionsProps {
  hasDecks: boolean;
}

export function LauncherActions({ hasDecks }: LauncherActionsProps) {
  return (
    <>
      <div className="launcher-actions">
        <Link className="launcher-action primary" to="/game">
          <span className="launcher-action-label">// PRIMARY ACTION</span>
          <span className="launcher-action-name">PLAY</span>
          <span className="launcher-action-desc">Start a local match on the redesigned board.</span>
          <span className="launcher-action-meta">ENTER</span>
        </Link>
        <Link className="launcher-action" to={hasDecks ? '/deckbuilder' : '/deckbuilder?import=1'}>
          <span className="launcher-action-label">// LIBRARY</span>
          <span className="launcher-action-name">MY DECKS</span>
          <span className="launcher-action-desc">Saved locally for desktop play.</span>
          <span className="launcher-action-meta">D</span>
        </Link>
      </div>
      <div className="launcher-quick">
        <Link to="/patch-notes"><span>// Updates</span><b>Patch notes</b></Link>
        <Link to="/lobby"><span>// Online</span><b>Lobby</b></Link>
        <Link to="/deckbuilder"><span>// Build</span><b>Deck builder</b></Link>
      </div>
    </>
  );
}
