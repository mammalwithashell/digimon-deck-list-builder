import { Link } from 'react-router-dom';

export function LauncherActions() {
  return (
    <>
      <div className="launcher-actions">
        <Link className="launcher-action primary" to="/play">
          <span className="launcher-action-label">// PRIMARY ACTION</span>
          <span className="launcher-action-name">PLAY</span>
          <span className="launcher-action-desc">Choose format, deck, and opponent.</span>
          <span className="launcher-action-meta">ENTER</span>
        </Link>
        <Link className="launcher-action" to="/deckbuilder">
          <span className="launcher-action-label">// LIBRARY</span>
          <span className="launcher-action-name">MY DECKS</span>
          <span className="launcher-action-desc">Saved locally for desktop play.</span>
          <span className="launcher-action-meta">D</span>
        </Link>
      </div>
      <div className="launcher-quick">
        <Link to="/patch-notes"><span>// Updates</span><b>Patch notes</b></Link>
        <Link to="/deckbuilder"><span>// Library</span><b>My decks</b></Link>
        <Link to="/deckbuilder/new"><span>// Build</span><b>Deck builder</b></Link>
      </div>
    </>
  );
}
