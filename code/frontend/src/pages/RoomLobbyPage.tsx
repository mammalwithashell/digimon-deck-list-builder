import { useEffect, useRef, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { getPlayFormat } from '@/features/play/formatCatalog';
import { createRoom, getDeck } from '@/features/play/playApi';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import type { DeckResponse } from '@/types/deck';
import './RoomLobbyPage.css';

export function RoomLobbyPage() {
  const { gameId: routeGameId } = useParams();
  const navigate = useNavigate();
  const { deckId, formatId, roomCode, gameId, setQueue } = usePlayFlowStore();
  const [deck, setDeck] = useState<DeckResponse | null>(null);
  const [creating, setCreating] = useState(routeGameId === 'new');
  const createStarted = useRef(false);
  const format = getPlayFormat(formatId);

  useEffect(() => {
    if (!deckId) {
      navigate('/play/deck', { replace: true });
      return;
    }
    let cancelled = false;
    getDeck(deckId)
      .then((loaded) => {
        if (!cancelled) setDeck(loaded);
      })
      .catch(() => navigate('/play/deck', { replace: true }));
    return () => {
      cancelled = true;
    };
  }, [deckId, navigate]);

  useEffect(() => {
    if (!deck || routeGameId !== 'new' || createStarted.current) return;
    let cancelled = false;
    createStarted.current = true;
    setCreating(true);
    createRoom({ deck, formatId })
      .then((room) => {
        if (cancelled) return;
        setQueue({ gameId: room.game_id, roomCode: room.join_code });
      })
      .finally(() => {
        if (!cancelled) setCreating(false);
      });
    return () => {
      cancelled = true;
    };
  }, [deck, formatId, routeGameId, setQueue]);

  const visibleCode = roomCode ?? '------';
  const visibleGameId = gameId ?? (routeGameId === 'new' ? null : routeGameId);

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
          </div>
          <div className="room-code-card">
            <span>// ROOM CODE</span>
            <strong>{creating ? '------' : visibleCode}</strong>
            <button type="button" onClick={() => void navigator.clipboard.writeText(visibleCode)}>
              COPY
            </button>
          </div>
        </header>

        <section className="room-grid">
          <article className="room-player p1">
            <span className="role">HOST</span>
            <h2>YOU</h2>
            <div className="deck-slot">
              <strong>{deck?.name ?? 'Loading deck'}</strong>
              <span>{deck ? `${deck.main_deck.length}/50 main - ${deck.egg_deck.length} eggs` : 'Resolving deck'}</span>
            </div>
            <span className="ready on">READY</span>
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

        <div className="room-actions">
          <button
            type="button"
            disabled={!visibleGameId}
            onClick={() => navigate(`/game/${visibleGameId}?mode=pvp&player=1`)}
          >
            ENTER GAME
          </button>
          <button type="button" onClick={() => navigate('/play/deck')}>
            CHANGE DECK
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
