import { useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import * as library from '@/api/deckLibraryAdapter';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { canUseDeckForFormat, getPlayFormat } from '@/features/play/formatCatalog';
import {
  cancelRoom,
  createRoom,
  getDeck,
  getRoomState,
  leaveRoom,
  setRoomDeck,
  setRoomFirstPlayer,
  startRoom,
} from '@/features/play/playApi';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import type { FirstPlayerChoice, LobbyState } from '@/api/lobbyApi';
import type { DeckResponse, DeckSummary } from '@/types/deck';
import './RoomLobbyPage.css';

const POLL_INTERVAL_MS = 2000;

const FIRST_PLAYER_OPTIONS: Array<{ value: FirstPlayerChoice; label: string }> = [
  { value: '1', label: '1' },
  { value: 'random', label: 'RANDOM' },
  { value: '2', label: '2' },
];

export function RoomLobbyPage() {
  const { gameId: routeGameId } = useParams();
  const navigate = useNavigate();
  const { deckId, formatId, roomCode, gameId, seat, selectDeck, setQueue } = usePlayFlowStore();
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [deck, setDeck] = useState<DeckResponse | null>(null);
  const [roomState, setRoomState] = useState<LobbyState | null>(null);
  const [creating, setCreating] = useState(routeGameId === 'new');
  const [lockingDeck, setLockingDeck] = useState(false);
  const [lockedDeckId, setLockedDeckId] = useState<string | null>(null);
  const [starting, setStarting] = useState(false);
  const [notice, setNotice] = useState('');
  const createRequest = useRef<Promise<{ game_id: string; join_code: string }> | null>(null);
  const navigatedRef = useRef(false);
  const format = getPlayFormat(formatId);

  const visibleGameId = gameId ?? (routeGameId === 'new' ? null : routeGameId ?? null);
  const visibleCode = roomState?.join_code ?? roomCode ?? '-----';
  // Seat: prefer live server state; fall back to the flow store; creating a
  // room always means hosting.
  const mySeat = roomState?.your_seat ?? (routeGameId === 'new' ? 1 : seat ?? 1);
  const isHost = mySeat === 1;
  const selectedSummary = useMemo(
    () => decks.find((item) => item.id === deckId) ?? null,
    [deckId, decks],
  );
  const selectedLegality = selectedSummary ? canUseDeckForFormat(selectedSummary, formatId) : null;
  const canLockDeck = Boolean(visibleGameId && deck && selectedLegality?.ok && !roomState?.started);
  const myDeckReady = Boolean(
    (isHost ? roomState?.host_deck_ready : roomState?.joiner_deck_ready) &&
      lockedDeckId &&
      lockedDeckId === deckId,
  );
  const opponentSeated = isHost ? Boolean(roomState?.joiner_display_name) : true;
  const opponentName = isHost
    ? roomState?.joiner_display_name ?? null
    : roomState?.host_display_name ?? null;
  const opponentDeckReady = Boolean(
    isHost ? roomState?.joiner_deck_ready : roomState?.host_deck_ready,
  );
  const bothReady = Boolean(roomState?.host_deck_ready && roomState?.joiner_deck_ready);

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

  // Host: create the room on /play/room/new
  useEffect(() => {
    if (routeGameId !== 'new') return;
    let cancelled = false;
    setCreating(true);
    createRequest.current ??= createRoom({ formatId });
    createRequest.current
      .then((room) => {
        if (cancelled) return;
        setQueue({ gameId: room.game_id, roomCode: room.join_code, seat: 1 });
        navigate(`/play/room/${room.game_id}`, { replace: true });
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
  }, [formatId, navigate, routeGameId, setQueue]);

  // Both seats: poll room state to see the opponent arrive, decks lock,
  // and the game start.
  useEffect(() => {
    if (!routeGameId || routeGameId === 'new') return;
    let cancelled = false;
    setQueue({ gameId: routeGameId });

    const poll = () => {
      getRoomState(routeGameId)
        .then((state) => {
          if (cancelled) return;
          setRoomState(state);
          setQueue({
            gameId: state.game_id,
            roomCode: state.join_code ?? undefined,
            seat: state.your_seat ?? undefined,
          });
        })
        .catch((err: unknown) => {
          if (cancelled) return;
          const status = (err as { response?: { status?: number } })?.response?.status;
          if (status === 404) {
            // Room cancelled/expired — back to the chooser.
            setNotice('Room closed');
            navigate('/play/room', { replace: true });
          } else {
            setNotice('Unable to load room state');
          }
        });
    };

    poll();
    const id = window.setInterval(poll, POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [navigate, routeGameId, setQueue]);

  // Auto-enter the game for both seats once started.
  useEffect(() => {
    if (!roomState?.started || !visibleGameId || navigatedRef.current) return;
    navigatedRef.current = true;
    const playerSeat = roomState.your_seat ?? mySeat;
    navigate(`/game/${visibleGameId}?mode=pvp&player=${playerSeat}`);
  }, [mySeat, navigate, roomState, visibleGameId]);

  // Lock the selected deck for my seat whenever the selection changes.
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

  const handleFirstPlayer = (choice: FirstPlayerChoice) => {
    if (!visibleGameId || !isHost || roomState?.started) return;
    setRoomFirstPlayer({ gameId: visibleGameId, firstPlayer: choice })
      .then(setRoomState)
      .catch(() => setNotice('Unable to set first player'));
  };

  const handleStart = () => {
    if (!visibleGameId || starting) return;
    setStarting(true);
    startRoom(visibleGameId)
      .then((state) => {
        setRoomState(state);
      })
      .catch(() => {
        setNotice('Unable to start — both decks must be locked');
        setStarting(false);
      });
  };

  const handleBack = () => {
    if (visibleGameId && !roomState?.started) {
      // Best-effort cleanup; the TTL sweep covers failures.
      void (isHost ? cancelRoom(visibleGameId) : leaveRoom(visibleGameId)).catch(() => undefined);
    }
    navigate('/play/room');
  };

  const myPanel = (
    <article className={`room-player ${isHost ? 'p1' : 'p2'}`}>
      <span className="role">{isHost ? 'HOST' : 'GUEST'}</span>
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
      <span className={`ready ${myDeckReady ? 'on' : 'waiting'}`}>
        {lockingDeck ? 'LOCKING' : myDeckReady ? 'READY' : 'CHOOSE DECK'}
      </span>
    </article>
  );

  const opponentPanel = (
    <article className={`room-player ${isHost ? 'p2' : 'p1'} ${opponentSeated ? '' : 'empty'}`}>
      <span className="role">{isHost ? 'OPPONENT' : 'HOST'}</span>
      <h2>{opponentSeated ? opponentName ?? 'OPPONENT' : 'WAITING...'}</h2>
      <div className="deck-slot">
        <strong>{opponentDeckReady ? 'DECK LOCKED' : 'NO DECK LOCKED'}</strong>
        <span>
          {opponentSeated
            ? opponentDeckReady
              ? 'Ready to battle'
              : 'Choosing a deck'
            : `Share code ${visibleCode}`}
        </span>
      </div>
      <span className={`ready ${opponentDeckReady ? 'on' : 'waiting'}`}>
        {opponentDeckReady ? 'READY' : 'WAITING'}
      </span>
    </article>
  );

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
            <span>
              {isHost ? 'SHARE THE ROOM CODE BELOW - PRIVATE' : `ROOM HOSTED BY ${opponentName ?? '...'}`}
            </span>
            <h1>ROOM LOBBY</h1>
            {notice && <p className="room-notice">{notice}</p>}
          </div>
          <div className="room-code-card">
            <span>// ROOM CODE</span>
            <strong>{creating ? '-----' : visibleCode}</strong>
            <button
              type="button"
              disabled={visibleCode === '-----'}
              onClick={() => void navigator.clipboard.writeText(visibleCode)}
            >
              COPY
            </button>
          </div>
        </header>

        <section className="room-grid">
          {myPanel}
          {opponentPanel}
        </section>

        {isHost && (
          <section className="room-first-player" aria-label="First player">
            <span className="room-first-player-label">// FIRST PLAYER</span>
            {FIRST_PLAYER_OPTIONS.map((option) => (
              <button
                key={option.value}
                type="button"
                className={(roomState?.first_player ?? 'random') === option.value ? 'on' : ''}
                disabled={roomState?.started}
                onClick={() => handleFirstPlayer(option.value)}
              >
                {option.label}
              </button>
            ))}
          </section>
        )}

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
          {isHost ? (
            <button type="button" disabled={!bothReady || starting} onClick={handleStart}>
              {starting ? 'STARTING' : 'START GAME'}
            </button>
          ) : (
            <span className={`ready ${myDeckReady ? 'on' : 'waiting'}`}>
              {myDeckReady ? 'READY - WAITING FOR HOST TO START' : 'LOCK A DECK TO READY UP'}
            </span>
          )}
          <button type="button" onClick={handleBack}>
            {isHost ? 'CANCEL ROOM' : 'LEAVE ROOM'}
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
