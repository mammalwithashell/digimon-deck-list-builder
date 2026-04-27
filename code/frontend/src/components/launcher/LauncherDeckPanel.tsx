import { Link } from 'react-router-dom';
import type { LauncherDeckRow } from './launcherData';

interface LauncherDeckPanelProps {
  decks: LauncherDeckRow[];
  loaded: boolean;
}

export function LauncherDeckPanel({ decks, loaded }: LauncherDeckPanelProps) {
  return (
    <section className="launcher-deck-panel" aria-labelledby="launcher-decks-heading">
      <div className="launcher-panel-head">
        <h2 id="launcher-decks-heading">Saved Decks</h2>
        <span>{String(decks.length).padStart(2, '0')} RECENT</span>
      </div>
      <div className="launcher-deck-list">
        {decks.map((deck) => (
          <Link className="launcher-deck-row" to={deck.href} key={deck.id}>
            <div className="launcher-deck-color"><span>{deck.levelLabel}</span></div>
            <div className="launcher-deck-info">
              <div className="launcher-deck-name">{deck.name}</div>
              <div className="launcher-deck-meta">
                <span className={`launcher-${deck.statusKind}`}>● {deck.statusLabel}</span>
                <span>{deck.countLabel}</span>
                <span>{deck.editedLabel}</span>
              </div>
            </div>
            <div className="launcher-deck-stat">{deck.metaLabel}</div>
          </Link>
        ))}
        {loaded && decks.length === 0 ? (
          <div className="launcher-empty">No saved decks yet.</div>
        ) : null}
      </div>
      <div className="launcher-panel-foot">
        <Link className="launcher-btn accent" to="/deckbuilder?new=1">+ New Deck</Link>
        <Link className="launcher-btn" to="/deckbuilder?import=1">Import</Link>
        <Link className="launcher-btn" to="/deckbuilder">View All →</Link>
      </div>
    </section>
  );
}
