import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { InBetweenShell } from '@/features/play/InBetweenShell';
import { getPlayFormat } from '@/features/play/formatCatalog';
import { getDeck } from '@/features/play/playApi';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import { useMatchmaking } from '@/hooks/useMatchmaking';
import type { DeckResponse } from '@/types/deck';
import './MatchingPage.css';

export function MatchingPage() {
  const navigate = useNavigate();
  const { formatId, deckId, queueType, setQueue } = usePlayFlowStore();
  const matchmaking = useMatchmaking();
  const [deck, setDeck] = useState<DeckResponse | null>(null);
  const [localWait, setLocalWait] = useState(0);
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
    if (!deck || matchmaking.status !== 'idle') return;
    void matchmaking.enqueue({
      queue_type: queueType,
      main_deck: deck.main_deck,
      egg_deck: deck.egg_deck,
      game_mode: formatId,
    });
  }, [deck, formatId, matchmaking, queueType]);

  useEffect(() => {
    if (matchmaking.ticketId) setQueue({ ticketId: matchmaking.ticketId });
  }, [matchmaking.ticketId, setQueue]);

  useEffect(() => {
    if (matchmaking.status !== 'waiting' && matchmaking.status !== 'connecting') {
      setLocalWait(0);
      return;
    }
    const id = window.setInterval(() => setLocalWait((value) => value + 1), 1000);
    return () => window.clearInterval(id);
  }, [matchmaking.status]);

  useEffect(() => {
    if (matchmaking.status !== 'matched' || !matchmaking.match) return;
    setQueue({ roomCode: matchmaking.match.join_code, gameId: matchmaking.match.game_id });
    navigate(`/game/${matchmaking.match.game_id}?mode=pvp&player=1`);
  }, [matchmaking.match, matchmaking.status, navigate, setQueue]);

  const elapsed = Math.max(localWait, Math.floor(matchmaking.waitedSeconds));
  const initials = useMemo(
    () =>
      deck?.name
        .split(/\s+/)
        .map((part) => part[0])
        .slice(0, 2)
        .join('') ?? '??',
    [deck],
  );

  return (
    <InBetweenShell
      title="MATCHMAKING"
      stepLabel="03"
      crumbs={[
        { label: 'PLAY', href: '/play' },
        { label: 'DECK', href: '/play/deck' },
        { label: 'MATCHING' },
      ]}
      rightSlot={<span>{format.name} - {deck?.name ?? 'LOADING'}</span>}
    >
      <main className="matching-main">
        <header className="matching-header">
          <div className="matching-kicker">// MATCHMAKING SERVICE - NA-WEST RELAY</div>
          <h1>
            SEARCHING
            <br />
            <em>FOR AN OPPONENT.</em>
          </h1>
          <p>SCANNING THE LADDER - BALANCED PAIRING - TYPICAL WAIT 25-45 SECONDS</p>
        </header>

        <section className="matching-stage">
          <article className="match-player-card p1">
            <span className="role">YOU</span>
            <div className="deck-art">{initials}</div>
            <h2>{deck?.name ?? 'Loading deck'}</h2>
            <p>{deck ? `${deck.main_deck.length}/50 main - ${deck.egg_deck.length} eggs` : 'Resolving deck'}</p>
            <span className="ready">READY</span>
          </article>

          <div className="matching-radar">
            <div className="pulse">VS</div>
            <strong>
              {Math.floor(elapsed / 60)}:{String(elapsed % 60).padStart(2, '0')}
            </strong>
            <span>{matchmaking.ratingWindow ? `RANGE +/-${matchmaking.ratingWindow}` : 'SCANNING...'}</span>
          </div>

          <article className="match-player-card p2">
            <span className="role">OPPONENT</span>
            <div className="deck-art muted">??</div>
            <h2>SEARCHING...</h2>
            <p>{matchmaking.error ?? 'Awaiting handshake'}</p>
            <span className="ready waiting">{matchmaking.status.toUpperCase()}</span>
          </article>
        </section>

        <div className="matching-actions">
          <button type="button" onClick={() => void matchmaking.cancel()}>
            CANCEL SEARCH
          </button>
        </div>
      </main>
    </InBetweenShell>
  );
}
