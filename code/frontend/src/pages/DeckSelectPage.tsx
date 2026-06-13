import { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import * as library from '@/api/deckLibraryAdapter';
import { normalizeSeedInput } from '@/api/gameApi';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { canUseDeckForFormat, getPlayFormat } from '@/features/play/formatCatalog';
import { createBotGame } from '@/features/play/playApi';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import type { DeckSummary } from '@/types/deck';
import './DeckSelectPage.css';

export function DeckSelectPage() {
  const navigate = useNavigate();
  const { formatId, opponentMode, deckId, selectDeck, setQueue } = usePlayFlowStore();
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [search, setSearch] = useState('');
  const [launching, setLaunching] = useState(false);
  const [seedInput, setSeedInput] = useState('');
  const [seedError, setSeedError] = useState('');
  const format = getPlayFormat(formatId);
  const selected = decks.find((deck) => deck.id === deckId) ?? decks[0] ?? null;
  const selectedLegality = selected ? canUseDeckForFormat(selected, formatId) : null;

  useEffect(() => {
    library
      .listDecks()
      .then((items) => {
        setDecks(items);
        const firstLegal = items.find((deck) => canUseDeckForFormat(deck, formatId).ok);
        selectDeck(firstLegal?.id ?? items[0]?.id ?? null);
      })
      .catch(() => setDecks([]));
  }, [formatId, selectDeck]);

  const filtered = useMemo(() => {
    const needle = search.trim().toLowerCase();
    if (!needle) return decks;
    return decks.filter((deck) =>
      `${deck.name} ${deck.meta_archetype ?? ''} ${deck.tags.join(' ')}`.toLowerCase().includes(needle),
    );
  }, [decks, search]);

  const handleConfirm = async () => {
    if (!selected || !selectedLegality?.ok) return;
    if (opponentMode === 'quick') {
      navigate('/play/matching');
      return;
    }
    if (opponentMode === 'room') {
      navigate('/play/room/new');
      return;
    }
    setLaunching(true);
    try {
      let normalizedSeed: string | null = null;
      try {
        normalizedSeed = normalizeSeedInput(seedInput);
        setSeedError('');
      } catch (err) {
        setSeedError((err as Error).message);
        setLaunching(false);
        return;
      }
      const deck = await library.getDeck(selected.id);
      const response = await createBotGame({
        deck,
        opponentDeck: deck,
        seed: normalizedSeed,
      });
      setQueue({ gameId: response.game_id, seed: response.seed ?? normalizedSeed });
      navigate(`/game/${response.game_id}`);
    } finally {
      setLaunching(false);
    }
  };

  return (
    <InBetweenShell
      title="CHOOSE DECK"
      stepLabel="02"
      crumbs={[
        { label: 'PLAY', href: '/play' },
        { label: 'FORMAT', href: '/play' },
        { label: 'CHOOSE DECK' },
      ]}
      rightSlot={<span>{format.name} - {format.deckLabel}</span>}
    >
      <main className="deck-select-main">
        <section className="deck-select-banner">
          <div>
            <span className="label">FORMAT //</span>
            <h1>{format.name}</h1>
            <p>{format.description}</p>
          </div>
          <Link to="/play">CHANGE</Link>
        </section>

        <section className="deck-select-toolbar">
          <input
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Decks, archetypes, tags"
          />
          <Link to="/deckbuilder/new?returnTo=play">NEW DECK</Link>
        </section>

        <section className="deck-select-grid">
          {filtered.map((deck) => {
            const legality = canUseDeckForFormat(deck, formatId);
            return (
              <button
                key={deck.id}
                type="button"
                aria-label={deck.name}
                className={`deck-select-card ${deck.id === selected?.id ? 'selected' : ''} ${legality.ok ? '' : 'illegal'}`}
                onClick={() => selectDeck(deck.id)}
              >
                <span className="glyph">
                  {deck.name
                    .split(/\s+/)
                    .map((part) => part[0])
                    .slice(0, 2)
                    .join('')}
                </span>
                <span className="name">{deck.name}</span>
                <span className="meta">
                  {deck.main_count}/{deck.egg_count} - {deck.meta_archetype ?? 'Unclassified'}
                </span>
                <span className={legality.ok ? 'legal' : 'warn'}>
                  {legality.ok ? `LEGAL IN ${format.name}` : legality.reason}
                </span>
              </button>
            );
          })}
        </section>

        <div className="deck-confirm-bar">
          <div className="deck-confirm-info">
            <span>{selected ? selected.name : 'NO DECK SELECTED'}</span>
            {opponentMode === 'bot' && (
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
            )}
          </div>
          <button
            type="button"
            disabled={!selected || !selectedLegality?.ok || launching}
            onClick={handleConfirm}
          >
            {launching ? 'LAUNCHING...' : 'USE THIS DECK'}
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
