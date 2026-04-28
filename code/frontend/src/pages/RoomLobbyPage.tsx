import { useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import * as library from '@/api/deckLibraryAdapter';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { canUseDeckForFormat, getPlayFormat } from '@/features/play/formatCatalog';
import { createRoom, getDeck, getRoomState, setRoomDeck } from '@/features/play/playApi';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import type { LobbyState } from '@/api/lobbyApi';
import type { DeckResponse, DeckSummary } from '@/types/deck';
import './RoomLobbyPage.css';

export function RoomLobbyPage() {
  const { gameId: routeGameId } = useParams();
  const navigate = useNavigate();
  const { deckId, formatId, roomCode, gameId, selectDeck, setQueue } = usePlayFlowStore();
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [deck, setDeck] = useState<DeckResponse | null>(null);
  const [roomState, setRoomState] = useState<LobbyState | null>(null);
  const [creating, setCreating] = useState(routeGameId === 'new');
  const [lockingDeck, setLockingDeck] = useState(false);
  const [lockedDeckId, setLockedDeckId] = useState<string | null>(null);
  const [notice, setNotice] = useState('');
  const createRequest = useRef<Promise<{ game_id: string; join_code: string }> | null>(null);
  const format = getPlayFormat(formatId);

  const visibleGameId = gameId ?? (routeGameId === 'new' ? null : routeGameId ?? null);
  const visibleCode = roomState?.join_code ?? roomCode ?? '------';
  const selectedSummary = useMemo(
    () => decks.find((item) => item.id === deckId) ?? null,
    [deckId, decks],
  );
  const selectedLegality = selectedSummary ? canUseDeckForFormat(selectedSummary, formatId) : null;
  const canLockDeck = Boolean(visibleGameId && deck && selectedLegality?.ok);
  const deckReady = Boolean(roomState?.host_deck_ready && lockedDeckId && lockedDeckId === deckId);

  useEffect(() => {
    let cancelled = false;
    library
      .listDecks()
      .then((items) => {
        if (cancelled) return;
        setDecks(items);
        if (!deckId) {
          const firstLegal = items.find((item) => canUseDeckForFormat(item, formatId).ok);
          selectDeck(firstLegal?.id ?? items[0]?.id ?? null);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setDecks([]);
          setNotice('Deck library unavailable');
        }
      });
    return () => {
      cancelled = true;
    };
  }, [deckId, formatId, selectDeck]);

  useEffect(() => {
    if (!deckId) {
      setDeck(null);
      return;
    }
    let cancelled = false;
    getDeck(deckId)
      .then((loaded) => {
        if (!cancelled) setDeck(loaded);
      })
      .catch(() => {
        if (!cancelled) {
          setDeck(null);
          setNotice('Unable to load selected deck');
        }
      });
    return () => {
      cancelled = true;
    };
  }, [deckId]);

  useEffect(() => {
    if (routeGameId !== 'new') return;
    let cancelled = false;
    setCreating(true);
    createRequest.current ??= createRoom({ formatId });
    createRequest.current
      .then((room) => {
        if (cancelled) return;
        setQueue({ gameId: room.game_id, roomCode: room.join_code });
        setRoomState({
          game_id: room.game_id,
          join_code: room.join_code,
          host_display_name: null,
          host_deck_ready: false,
          joiner_deck_ready: false,
          started: false,
        });
      })
      .catch(() => {
        if (!cancelled) setNotice('Unable to create room');
      })
      .finally(() => {
        if (!cancelled) setCreating(false);
      });
    return () => {
      cancelled = true;
    };
  }, [formatId, routeGameId, setQueue]);

  useEffect(() => {
    if (!routeGameId || routeGameId === 'new') return;
    let cancelled = false;
    setQueue({ gameId: routeGameId });
    getRoomState(routeGameId)
      .then((state) => {
        if (cancelled) return;
        setRoomState(state);
        setQueue({ gameId: state.game_id, roomCode: state.join_code ?? undefined });
      })
      .catch(() => {
        if (!cancelled) setNotice('Unable to load room state');
      });
    return () => {
      cancelled = true;
    };
  }, [routeGameId, setQueue]);

  useEffect(() => {
    if (!canLockDeck || !visibleGameId || !deckId || lockedDeckId === deckId) return;
    let cancelled = false;
    setLockingDeck(true);
    setRoomDeck({ gameId: visibleGameId, deck: deck! })
      .then((state) => {
        if (cancelled) return;
        setRoomState(state);
        setLockedDeckId(deckId);
        setNotice('Deck locked in room');
      })
      .catch(() => {
        if (!cancelled) setNotice('Unable to lock deck');
      })
      .finally(() => {
        if (!cancelled) setLockingDeck(false);
      });
    return () => {
      cancelled = true;
    };
  }, [canLockDeck, deck, deckId, lockedDeckId, visibleGameId]);

  return (
    <InBetweenShell
      title="ROOM LOBBY"
      stepLabel="02"
      crumbs={[{ label: 'PLAY', href: '/play' }, { label: 'ROOM MATCH' }, { label: 'LOBBY' }]}
      rightSlot={<span>{format.name}</span>}
    >
      <main className="room-main">
        <header className="room-header">
          <div>
            <span>SHARE THE ROOM CODE BELOW - PRIVATE</span>
            <h1>ROOM LOBBY</h1>
            {notice && <p className="room-notice">{notice}</p>}
          </div>
          <div className="room-code-card">
            <span>// ROOM CODE</span>
            <strong>{creating ? '------' : visibleCode}</strong>
            <button
              type="button"
              disabled={visibleCode === '------'}
              onClick={() => void navigator.clipboard.writeText(visibleCode)}
            >
              COPY
            </button>
          </div>
        </header>

        <section className="room-grid">
          <article className="room-player p1">
            <span className="role">HOST</span>
            <h2>YOU</h2>
            <div className="deck-slot">
              <strong>{deck?.name ?? 'NO DECK LOCKED'}</strong>
              <span>
                {deck
                  ? `${deck.main_deck.length}/50 main - ${deck.egg_deck.length} eggs`
                  : 'Choose a deck in this room'}
              </span>
              {selectedLegality && !selectedLegality.ok && <em>{selectedLegality.reason}</em>}
            </div>
            <span className={`ready ${deckReady ? 'on' : 'waiting'}`}>
              {lockingDeck ? 'LOCKING' : deckReady ? 'READY' : 'CHOOSE DECK'}
            </span>
          </article>
          <article className="room-player p2 empty">
            <span className="role">OPPONENT</span>
            <h2>WAITING...</h2>
            <div className="deck-slot">
              <strong>NO DECK LOCKED</strong>
              <span>Share code {visibleCode}</span>
            </div>
            <span className="ready waiting">WAITING</span>
          </article>
        </section>

        <section className="room-deck-picker" aria-label="Room deck selection">
          <div className="room-deck-picker-head">
            <div>
              <span>DECK SELECTION</span>
              <strong>{selectedSummary?.name ?? 'Select a saved deck'}</strong>
            </div>
            <Link to="/deckbuilder/new?returnTo=play">NEW DECK</Link>
          </div>
          <div className="room-deck-list">
            {decks.map((item) => {
              const legality = canUseDeckForFormat(item, formatId);
              return (
                <button
                  type="button"
                  key={item.id}
                  className={`room-deck-option ${item.id === deckId ? 'selected' : ''} ${legality.ok ? '' : 'illegal'}`}
                  onClick={() => {
                    selectDeck(item.id);
                    setLockedDeckId(null);
                  }}
                >
                  <span className="name">{item.name}</span>
                  <span>{item.main_count}/{item.egg_count} - {item.meta_archetype ?? 'Unclassified'}</span>
                  <b>{legality.ok ? `LEGAL IN ${format.name}` : legality.reason}</b>
                </button>
              );
            })}
            {decks.length === 0 && <div className="room-empty">NO SAVED DECKS</div>}
          </div>
        </section>

        <div className="room-actions">
          <button
            type="button"
            disabled={!visibleGameId || !deckReady}
            onClick={() => navigate(`/game/${visibleGameId}?mode=pvp&player=1`)}
          >
            ENTER GAME
          </button>
          <button type="button" onClick={() => navigate('/play')}>
            BACK TO FORMAT
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
