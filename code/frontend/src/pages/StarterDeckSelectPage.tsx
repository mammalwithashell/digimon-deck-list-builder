import { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { normalizeSeedInput } from '@/api/gameApi';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { createAiStarterGame, listStarterDecks } from '@/features/play/playApi';
import type { DeckResponse } from '@/types/deck';
import { getCardImageUrl } from '@/utils/cardImages';
import './DeckSelectPage.css';

export function StarterDeckSelectPage() {
  const navigate = useNavigate();
  const [decks, setDecks] = useState<DeckResponse[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [seedInput, setSeedInput] = useState('');
  const [seedError, setSeedError] = useState('');
  const [launching, setLaunching] = useState(false);

  useEffect(() => {
    listStarterDecks()
      .then((items) => {
        setDecks(items);
        setSelectedId(items[0]?.id ?? null);
      })
      .catch(() => setDecks([]));
  }, []);

  const selected = useMemo(
    () => decks.find((deck) => deck.id === selectedId) ?? decks[0] ?? null,
    [decks, selectedId],
  );

  const handleConfirm = async () => {
    if (!selected || launching) return;
    let normalizedSeed: string | null = null;
    try {
      normalizedSeed = normalizeSeedInput(seedInput);
      setSeedError('');
    } catch (err) {
      setSeedError((err as Error).message);
      return;
    }
    setLaunching(true);
    try {
      const response = await createAiStarterGame({
        deck: selected,
        starterDecks: decks,
        seed: normalizedSeed,
      });
      navigate(`/game/${response.game_id}`);
    } finally {
      setLaunching(false);
    }
  };

  return (
    <InBetweenShell
      title="CHOOSE STARTER"
      stepLabel="02"
      crumbs={[{ label: 'PLAY', href: '/play' }, { label: 'AI STARTER DECK' }]}
      rightSlot={<span>STARTER - vs AI</span>}
    >
      <main className="deck-select-main">
        <section className="deck-select-banner">
          <div>
            <span className="label">MODE //</span>
            <h1>AI STARTER DECK</h1>
            <p>Pick a starter deck. The AI plays a random one of the six.</p>
          </div>
          <Link to="/play">CHANGE</Link>
        </section>

        <section className="deck-select-grid">
          {decks.map((deck) => (
            <button
              key={deck.id}
              type="button"
              aria-label={deck.name}
              className={`deck-select-card ${deck.id === selected?.id ? 'selected' : ''}`}
              onClick={() => setSelectedId(deck.id)}
            >
              <span className="glyph">
                <img
                  src={getCardImageUrl(deck.egg_deck[0] ?? deck.main_deck[0] ?? '')}
                  alt=""
                  loading="lazy"
                  draggable={false}
                  onError={(event) => {
                    event.currentTarget.style.display = 'none';
                  }}
                />
              </span>
              <span className="name">{deck.name}</span>
              <span className="meta">
                {deck.main_deck.length}/{deck.egg_deck.length} - {deck.meta_archetype ?? 'Starter'}
              </span>
              <span className="legal">READY</span>
            </button>
          ))}
        </section>

        <div className="deck-confirm-bar">
          <div className="deck-confirm-info">
            <span>{selected ? 'DECK READY' : 'NO DECK SELECTED'}</span>
            <label className="deck-seed-control">
              <span>SHUFFLE SEED</span>
              <input
                value={seedInput}
                onChange={(event) => {
                  setSeedInput(event.target.value);
                  setSeedError('');
                }}
                placeholder="Random"
                inputMode="numeric"
              />
              {seedError && <em>{seedError}</em>}
            </label>
          </div>
          <button type="button" disabled={!selected || launching} onClick={handleConfirm}>
            {launching ? 'LAUNCHING...' : 'FACE THE AI'}
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
