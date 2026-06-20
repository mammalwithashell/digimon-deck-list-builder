import { Link } from 'react-router-dom';
import { Window } from '@/design/components/Window';

/**
 * Primary launcher calls-to-action. The Play and My-Decks cards use the
 * design-system `Window` chrome so they carry the themed title bar — a green
 * terminal header in dark, a navy Win95 bar in light (add-desktop-menu-shell).
 */
export function LauncherActions() {
  return (
    <>
      <div className="launcher-actions">
        <Link className="launcher-action-window primary" to="/play" aria-label="Play">
          <Window title="PLAY">
            <span className="launcher-action-desc">Choose format, deck, and opponent.</span>
            <span className="launcher-action-meta">ENTER</span>
          </Window>
        </Link>
        <Link className="launcher-action-window" to="/deckbuilder" aria-label="My Decks">
          <Window title="MY DECKS">
            <span className="launcher-action-desc">Saved locally for desktop play.</span>
            <span className="launcher-action-meta">D</span>
          </Window>
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
