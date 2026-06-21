import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import * as library from '@/api/deckLibraryAdapter';
import { normalizeSeedInput } from '@/api/gameApi';
import { DeckEditOverlay } from '@/components/deckbuilder/DeckEditOverlay';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { canUseDeckForFormat, getPlayFormat } from '@/features/play/formatCatalog';
import { createBotGame } from '@/features/play/playApi';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import type { DeckSummary } from '@/types/deck';
import { getCardImageUrl } from '@/utils/cardImages';
import './DeckSelectPage.css';

function deckInitials(name: string): string {
  return name
    .split(/\s+/)
    .map((part) => part[0])
    .slice(0, 2)
    .join('');
}

export function DeckSelectPage() {
  const navigate = useNavigate();
  const { formatId, opponentMode, deckId, selectDeck, setQueue } = usePlayFlowStore();
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [search, setSearch] = useState('');
  const [showAllFormats, setShowAllFormats] = useState(false);
  const [launching, setLaunching] = useState(false);
  const [seedInput, setSeedInput] = useState('');
  const [seedError, setSeedError] = useState('');
  const [editingDeckId, setEditingDeckId] = useState<string | null>(null);
  const format = getPlayFormat(formatId);

  // Decks visible in this queue: by default only those built for the current
  // format (a deck belongs to one format), with an opt-in to see the rest.
  const formatDecks = useMemo(
    () => (showAllFormats ? decks : decks.filter((deck) => deck.game_mode === formatId)),
    [decks, formatId, showAllFormats],
  );
  const otherFormatCount = decks.length - decks.filter((deck) => deck.game_mode === formatId).length;

  const selected = formatDecks.find((deck) => deck.id === deckId) ?? formatDecks[0] ?? null;
  const selectedLegality = selected ? canUseDeckForFormat(selected, formatId) : null;

  useEffect(() => {
    library
      .listDecks()
      .then((items) => {
        setDecks(items);
        // Prefer a legal deck for THIS format; fall back to any deck for it.
        const forFormat = items.filter((deck) => deck.game_mode === formatId);
        const firstLegal = forFormat.find((deck) => canUseDeckForFormat(deck, formatId).ok);
        selectDeck(firstLegal?.id ?? forFormat[0]?.id ?? null);
      })
      .catch(() => setDecks([]));
  }, [formatId, selectDeck]);

  const filtered = useMemo(() => {
    const needle = search.trim().toLowerCase();
    if (!needle) return formatDecks;
    return formatDecks.filter((deck) =>
      `${deck.name} ${deck.meta_archetype ?? ''} ${deck.tags.join(' ')}`.toLowerCase().includes(needle),
    );
  }, [formatDecks, search]);

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

  // After an in-overlay save: refresh the deck list so counts/legality update,
  // keep the edited deck selected, and close the overlay.
  const handleDeckSaved = useCallback(
    (savedId: string) => {
      setEditingDeckId(null);
      library
        .listDecks()
        .then((items) => {
          setDecks(items);
          selectDeck(savedId);
        })
        .catch(() => {
          /* keep the prior list on a refresh failure */
        });
    },
    [selectDeck],
  );

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
          {otherFormatCount > 0 && (
            <button
              type="button"
              className="deck-select-toggle"
              aria-pressed={showAllFormats}
              onClick={() => setShowAllFormats((value) => !value)}
            >
              {showAllFormats
                ? `SHOW ONLY ${format.name}`
                : `SHOW ALL FORMATS (+${otherFormatCount})`}
            </button>
          )}
          <Link to={`/deckbuilder/new?returnTo=play&format=${formatId}`}>NEW DECK</Link>
        </section>

        <section className="deck-select-grid">
          {filtered.length === 0 && (
            <div className="deck-select-empty">
              {search.trim() ? (
                <p>No decks match “{search.trim()}”.</p>
              ) : (
                <>
                  <p>No {format.name} decks yet.</p>
                  <Link to={`/deckbuilder/new?returnTo=play&format=${formatId}`}>
                    Build a {format.name} deck
                  </Link>
                  {otherFormatCount > 0 && !showAllFormats && (
                    <button type="button" onClick={() => setShowAllFormats(true)}>
                      or show your other decks
                    </button>
                  )}
                </>
              )}
            </div>
          )}
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
                  {deck.deck_icon_card_id ? (
                    <img
                      src={getCardImageUrl(deck.deck_icon_card_id)}
                      alt=""
                      loading="lazy"
                      draggable={false}
                      onError={(event) => {
                        event.currentTarget.style.display = 'none';
                      }}
                    />
                  ) : null}
                  <span>{deckInitials(deck.name)}</span>
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
          <div className="deck-confirm-actions">
            <button
              type="button"
              className="deck-edit-button"
              disabled={!selected}
              onClick={() => selected && setEditingDeckId(selected.id)}
            >
              EDIT
            </button>
            <button
              type="button"
              disabled={!selected || !selectedLegality?.ok || launching}
              onClick={handleConfirm}
            >
              {launching ? 'LAUNCHING...' : 'USE THIS DECK'}
            </button>
          </div>
        </div>

        <DeckEditOverlay
          isOpen={editingDeckId !== null}
          deckId={editingDeckId}
          onClose={() => setEditingDeckId(null)}
          onSaved={handleDeckSaved}
        />
      </main>
    </InBetweenShell>
  );
}
