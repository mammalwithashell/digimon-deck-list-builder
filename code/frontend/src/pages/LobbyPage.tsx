import { useState, useEffect, useCallback } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import { useMatchmaking } from '@/hooks/useMatchmaking';
import * as lobbyApi from '@/api/lobbyApi';
import * as deckApiMod from '@/api/deckApi';
import * as deckStore from '@/storage/deckStore';
import { getConfig as getMatchmakingConfig } from '@/api/matchmaking';
import type { QueueType } from '@/api/matchmaking';
import type { DeckSummary } from '@/types/deck';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';
const decks = IS_DESKTOP ? deckStore : deckApiMod;

// Shared token-based control styles (Adventure '99 / Digi-OS theming).
const FIELD =
  'border border-[var(--line-1)] bg-[var(--surface-raised)] px-3 py-2 text-[var(--ink-0)] shadow-[inset_1.5px_1.5px_0_var(--bevel-lo),inset_-1px_-1px_0_var(--bevel-hi)]';
const PRIMARY_BTN =
  'bg-[var(--accent)] text-[var(--accent-ink)] shadow-[var(--bevel-shadow)] hover:opacity-90 disabled:opacity-50';
const DANGER_BTN =
  'bg-[var(--danger)] text-white shadow-[var(--bevel-shadow)] hover:opacity-90 disabled:opacity-50';

export function LobbyPage() {
  const navigate = useNavigate();
  const { savedDecks, setSavedDecks } = useDeckBuilderStore();

  // Load decks
  useEffect(() => {
    decks.listDecks().then(setSavedDecks).catch(() => {});
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Tab state
  const [tab, setTab] = useState<'play' | 'create' | 'join'>('play');

  // Play (matchmaking queue) tab state
  const [playQueueType, setPlayQueueType] = useState<QueueType>('casual');
  const [playDeckId, setPlayDeckId] = useState('');
  const [availableQueues, setAvailableQueues] = useState<QueueType[]>(['jank', 'casual', 'sweat']);
  const matchmaking = useMatchmaking();

  // Discover which queues the server currently accepts. `ranked` is gated
  // behind a server-side feature flag, so fetch the config rather than
  // hard-coding the visible set.
  useEffect(() => {
    getMatchmakingConfig()
      .then((cfg) => setAvailableQueues(cfg.queues))
      .catch(() => {/* fall back to the default alpha set */});
  }, []);

  // When a match is found, the game is already live server-side — navigate
  // straight in as the assigned seat.
  useEffect(() => {
    if (matchmaking.status !== 'matched' || !matchmaking.match) return;
    const { game_id, your_seat } = matchmaking.match;
    navigate(`/game/${game_id}?mode=pvp&player=${your_seat}`);
  }, [matchmaking.status, matchmaking.match, navigate]);

  // Drive a local waited-seconds counter between WS heartbeats so the
  // "Searching..." display updates every second without server pings.
  const [localWaited, setLocalWaited] = useState(0);
  useEffect(() => {
    if (matchmaking.status !== 'waiting' && matchmaking.status !== 'connecting') {
      setLocalWaited(0);
      return;
    }
    const interval = setInterval(() => setLocalWaited((s) => s + 1), 1000);
    return () => clearInterval(interval);
  }, [matchmaking.status]);
  useEffect(() => {
    if (matchmaking.waitedSeconds > 0) {
      setLocalWaited(Math.floor(matchmaking.waitedSeconds));
    }
  }, [matchmaking.waitedSeconds]);

  const handleQueue = useCallback(async () => {
    if (!playDeckId) return;
    // Always send the inline deck shape in desktop — the guest user has no
    // server-side Deck row to reference. Queue type IS the tier filter:
    // jank/sweat gate on classifier output, casual is open, ranked (when
    // enabled) matches by rating.
    const deck = await decks.getDeck(playDeckId);
    void matchmaking.enqueue({
      queue_type: playQueueType,
      main_deck: deck.main_deck,
      egg_deck: deck.egg_deck,
      game_mode: deck.game_mode,
    });
  }, [matchmaking, playDeckId, playQueueType]);

  // Create tab state
  const [selectedDeckId, setSelectedDeckId] = useState('');
  const [isPublic, setIsPublic] = useState(true);
  const [allowSpectators, setAllowSpectators] = useState(true);
  const [spectatorMode, setSpectatorMode] = useState<'hidden' | 'open'>('hidden');
  const [createdCode, setCreatedCode] = useState<string | null>(null);
  const [createdGameId, setCreatedGameId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [opponentJoined, setOpponentJoined] = useState(false);

  // Two-phase rooms: there is no live game to watch over WS until the host
  // starts it. Created rooms hand off to the room screen, which polls
  // lobby state for the opponent's arrival and the start signal.
  useEffect(() => {
    if (createdGameId) {
      navigate(`/play/room/${createdGameId}`);
    }
  }, [createdGameId, navigate]);

  // Join tab state
  const [joinCode, setJoinCode] = useState('');
  const [joinDeckId, setJoinDeckId] = useState('');
  const [joining, setJoining] = useState(false);

  const handleCreate = async () => {
    if (!selectedDeckId) return;
    setCreating(true);
    try {
      const deck = await decks.getDeck(selectedDeckId);
      const result = await lobbyApi.createLobby({
        deck: [...deck.egg_deck, ...deck.main_deck],
        is_public: isPublic,
        allow_spectators: allowSpectators,
        spectator_mode: spectatorMode,
      });
      setCreatedCode(result.join_code);
      setCreatedGameId(result.game_id);
    } catch (err) {
      console.error('Failed to create lobby:', err);
    } finally {
      setCreating(false);
    }
  };

  const handleJoin = async (code: string, _deckId: string) => {
    if (!code) return;
    setJoining(true);
    try {
      // Two-phase rooms: joining reserves the seat; deck locking and the
      // ready/start dance happen on the room screen.
      const result = await lobbyApi.joinLobby(code);
      navigate(`/play/room/${result.game_id}`);
    } catch (err) {
      console.error('Failed to join lobby:', err);
    } finally {
      setJoining(false);
    }
  };

  const handleCancel = async () => {
    if (!createdGameId) return;
    try {
      await lobbyApi.cancelLobby(createdGameId);
    } catch {
      // ignore
    }
    setCreatedCode(null);
    setCreatedGameId(null);
    setOpponentJoined(false);
  };

  // Deck selector helper
  const DeckSelect = ({ value, onChange }: { value: string; onChange: (v: string) => void }) => (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className={`w-full ${FIELD}`}
    >
      <option value="">Select a deck...</option>
      {savedDecks.map((d) => (
        <option key={d.id} value={d.id}>
          {d.name}
        </option>
      ))}
    </select>
  );

  return (
    <div className="mx-auto max-w-2xl px-4 py-8">
      <Link
        to="/"
        className="mb-4 inline-flex items-center gap-2 border border-[var(--line-1)] bg-[var(--surface)] px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-[var(--ink-1)] shadow-[var(--bevel-shadow)] transition-colors hover:text-[var(--ink-0)]"
      >
        ← Back to Launcher
      </Link>

      <div className="border border-[var(--line-1)] bg-[var(--surface)] p-6 shadow-[var(--bevel-shadow)]">
        <h1 className="mb-6 text-2xl font-bold text-[var(--ink-0)]">Multiplayer Lobby</h1>

        {/* Tabs */}
        <div className="mb-6 flex gap-2">
          {(['play', 'create', 'join'] as const).map((t) => (
            <button
              key={t}
              onClick={() => setTab(t)}
              className={`px-4 py-2 text-sm font-medium shadow-[var(--bevel-shadow)] ${
                tab === t
                  ? 'bg-[var(--accent)] text-[var(--accent-ink)]'
                  : 'bg-[var(--surface-raised)] text-[var(--ink-1)] hover:text-[var(--ink-0)]'
              }`}
            >
              {t === 'play' ? 'Play' : t === 'create' ? 'Create Game' : 'Join Game'}
            </button>
          ))}
        </div>

        {/* Play (matchmaking queue) Tab */}
        {tab === 'play' && (
          <PlayTab
            decks={savedDecks}
            queueType={playQueueType}
            onQueueType={setPlayQueueType}
            availableQueues={availableQueues}
            deckId={playDeckId}
            onDeckId={setPlayDeckId}
            onQueue={handleQueue}
            onCancel={() => void matchmaking.cancel()}
            status={matchmaking.status}
            match={matchmaking.match}
            waitedSeconds={localWaited}
            ratingWindow={matchmaking.ratingWindow}
            errorMsg={matchmaking.error}
          />
        )}

        {/* Create Tab */}
        {tab === 'create' && (
          <div className="space-y-4">
            {createdCode ? (
              <div className="border border-[var(--good)] bg-[var(--surface-raised)] p-6 text-center">
                <p className="mb-2 text-sm text-[var(--ink-1)]">Share this code with your opponent:</p>
                <p className="mb-4 font-mono text-4xl font-bold tracking-widest text-[var(--good)]">
                  {createdCode}
                </p>
                <p className="mb-4 text-sm text-[var(--ink-2)]">
                  {opponentJoined
                    ? 'Opponent joined! Redirecting to game...'
                    : 'Waiting for opponent to join...'}
                </p>
                <div className="flex justify-center gap-3">
                  <button
                    onClick={handleCancel}
                    disabled={opponentJoined}
                    className={`px-4 py-2 ${DANGER_BTN}`}
                  >
                    Cancel
                  </button>
                </div>
              </div>
            ) : (
              <>
                <div>
                  <label className="mb-1 block text-sm text-[var(--ink-1)]">Your Deck</label>
                  <DeckSelect value={selectedDeckId} onChange={setSelectedDeckId} />
                </div>
                <div className="flex items-center gap-4">
                  <label className="flex items-center gap-2 text-sm text-[var(--ink-1)]">
                    <input
                      type="checkbox"
                      checked={isPublic}
                      onChange={(e) => setIsPublic(e.target.checked)}
                    />
                    List in public lobby
                  </label>
                  <label className="flex items-center gap-2 text-sm text-[var(--ink-1)]">
                    <input
                      type="checkbox"
                      checked={allowSpectators}
                      onChange={(e) => setAllowSpectators(e.target.checked)}
                    />
                    Allow spectators
                  </label>
                </div>
                {allowSpectators && (
                  <div>
                    <label className="mb-1 block text-sm text-[var(--ink-1)]">Spectator Mode</label>
                    <select
                      value={spectatorMode}
                      onChange={(e) => setSpectatorMode(e.target.value as 'hidden' | 'open')}
                      className={FIELD}
                    >
                      <option value="hidden">Hidden (hands/security hidden)</option>
                      <option value="open">Open (full visibility)</option>
                    </select>
                  </div>
                )}
                <button
                  onClick={handleCreate}
                  disabled={!selectedDeckId || creating}
                  className={`px-6 py-2 ${PRIMARY_BTN}`}
                >
                  {creating ? 'Creating...' : 'Create Game'}
                </button>
              </>
            )}
          </div>
        )}

        {/* Join Tab */}
        {tab === 'join' && (
          <div className="space-y-4">
            <div>
              <label className="mb-1 block text-sm text-[var(--ink-1)]">Join Code</label>
              <input
                type="text"
                value={joinCode}
                onChange={(e) => setJoinCode(e.target.value.toUpperCase())}
                placeholder="ABC123"
                maxLength={6}
                className={`w-full font-mono text-lg tracking-widest placeholder-[var(--ink-3)] ${FIELD}`}
              />
            </div>
            <div>
              <label className="mb-1 block text-sm text-[var(--ink-1)]">Your Deck</label>
              <DeckSelect value={joinDeckId} onChange={setJoinDeckId} />
            </div>
            <button
              onClick={() => handleJoin(joinCode, joinDeckId)}
              disabled={!joinCode || !joinDeckId || joining}
              className={`px-6 py-2 ${PRIMARY_BTN}`}
            >
              {joining ? 'Joining...' : 'Join Game'}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}


function tierBadge(tier: string | null | undefined): { label: string; className: string } {
  if (tier === 'meta') return { label: 'meta', className: 'border border-[var(--danger)] text-[var(--danger)]' };
  if (tier === 'rogue') return { label: 'rogue', className: 'border border-[var(--warn)] text-[var(--warn)]' };
  if (tier === 'jank') return { label: 'jank', className: 'border border-[var(--good)] text-[var(--good)]' };
  return { label: 'unclassified', className: 'border border-[var(--line-1)] text-[var(--ink-2)]' };
}


interface PlayTabProps {
  decks: DeckSummary[];
  queueType: QueueType;
  onQueueType: (q: QueueType) => void;
  availableQueues: QueueType[];
  deckId: string;
  onDeckId: (id: string) => void;
  onQueue: () => void;
  onCancel: () => void;
  status: ReturnType<typeof useMatchmaking>['status'];
  match: ReturnType<typeof useMatchmaking>['match'];
  waitedSeconds: number;
  ratingWindow: number | null;
  errorMsg: string | null;
}

const QUEUE_DESCRIPTIONS: Record<QueueType, string> = {
  jank: 'Off-meta safe space — jank-vs-jank only.',
  casual: 'Any deck, any tier. No rating changes.',
  sweat: 'Tournament-shape — meta + rogue only.',
  ranked: 'Matched by skill rating; rating updates after the game.',
};

const QUEUE_LABELS: Record<QueueType, string> = {
  jank: 'Jank',
  casual: 'Casual',
  sweat: 'Sweat',
  ranked: 'Ranked',
};

function PlayTab(props: PlayTabProps) {
  const {
    decks, queueType, onQueueType, availableQueues, deckId, onDeckId,
    onQueue, onCancel, status, match, waitedSeconds, ratingWindow, errorMsg,
  } = props;
  const selectedDeck = decks.find((d) => d.id === deckId);
  const badge = selectedDeck ? tierBadge(selectedDeck.meta_tier) : null;
  const isQueued = status === 'waiting' || status === 'connecting';
  const isTerminal = status === 'matched' || status === 'error' || status === 'cancelled';

  if (isQueued) {
    const label = queueType === 'ranked'
      ? `Searching ranked — window ±${ratingWindow ?? 50}`
      : `Searching ${QUEUE_LABELS[queueType].toLowerCase()}...`;
    return (
      <div className="border border-[var(--accent)] bg-[var(--surface-raised)] p-8 text-center">
        <div className="mb-2 text-lg font-medium text-[var(--ink-0)]">{label}</div>
        <div className="mb-4 font-mono text-3xl text-[var(--accent)]">
          {Math.floor(waitedSeconds / 60)}:{String(waitedSeconds % 60).padStart(2, '0')}
        </div>
        <button
          onClick={onCancel}
          className={`px-4 py-2 ${DANGER_BTN}`}
        >
          Cancel
        </button>
      </div>
    );
  }

  if (status === 'matched' && match) {
    return (
      <div className="border border-[var(--good)] bg-[var(--surface-raised)] p-8 text-center">
        <div className="mb-2 text-lg font-medium text-[var(--ink-0)]">Match found!</div>
        {match.opponent.display_name ? (
          <p className="mb-2 text-[var(--ink-1)]">vs {match.opponent.display_name}</p>
        ) : null}
        <p className="text-sm text-[var(--ink-2)]">Connecting to game…</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {isTerminal && errorMsg ? (
        <div className="border border-[var(--danger)] bg-[var(--surface-raised)] p-3 text-sm text-[var(--danger)]">
          {errorMsg}
        </div>
      ) : null}

      <div>
        <label className="mb-1 block text-sm text-[var(--ink-1)]">Queue</label>
        <div className="flex gap-2">
          {availableQueues.map((q) => (
            <button
              key={q}
              onClick={() => onQueueType(q)}
              className={`flex-1 px-4 py-2 text-sm font-medium shadow-[var(--bevel-shadow)] ${
                queueType === q
                  ? 'bg-[var(--accent)] text-[var(--accent-ink)]'
                  : 'bg-[var(--surface-raised)] text-[var(--ink-1)] hover:text-[var(--ink-0)]'
              }`}
            >
              {QUEUE_LABELS[q]}
            </button>
          ))}
        </div>
        <p className="mt-1 text-xs text-[var(--ink-3)]">
          {QUEUE_DESCRIPTIONS[queueType]}
        </p>
      </div>

      <div>
        <label className="mb-1 block text-sm text-[var(--ink-1)]">Your Deck</label>
        <select
          value={deckId}
          onChange={(e) => onDeckId(e.target.value)}
          className={`w-full ${FIELD}`}
        >
          <option value="">Select a deck...</option>
          {decks.map((d) => (
            <option key={d.id} value={d.id}>
              {d.name} {d.meta_tier ? `— ${d.meta_tier}` : ''}
            </option>
          ))}
        </select>
        {selectedDeck && badge ? (
          <div className="mt-2 flex items-center gap-2 text-xs">
            <span className={`px-2 py-0.5 font-medium ${badge.className}`}>
              {badge.label}
            </span>
            {selectedDeck.meta_archetype ? (
              <span className="text-[var(--ink-2)]">→ {selectedDeck.meta_archetype}</span>
            ) : null}
            <span className="text-[var(--ink-3)]">· format: {selectedDeck.game_mode}</span>
          </div>
        ) : null}
      </div>

      <button
        onClick={onQueue}
        disabled={!deckId}
        className={`w-full px-6 py-3 text-base font-medium ${PRIMARY_BTN}`}
      >
        Find Match
      </button>
    </div>
  );
}
