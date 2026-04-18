import { useState, useEffect, useCallback, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import { useWebSocketGame } from '@/hooks/useWebSocketGame';
import { useMatchmaking } from '@/hooks/useMatchmaking';
import * as lobbyApi from '@/api/lobbyApi';
import * as deckApiMod from '@/api/deckApi';
import type { LobbyGame } from '@/api/lobbyApi';
import type { QueueType, TierFilter } from '@/api/matchmaking';
import type { DeckSummary } from '@/types/deck';

export function LobbyPage() {
  const navigate = useNavigate();
  const { savedDecks, setSavedDecks } = useDeckBuilderStore();

  // Load decks
  useEffect(() => {
    deckApiMod.listDecks().then(setSavedDecks).catch(() => {});
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Tab state
  const [tab, setTab] = useState<'play' | 'create' | 'join' | 'browse'>('play');

  // Play (matchmaking queue) tab state
  const [playQueueType, setPlayQueueType] = useState<QueueType>('casual');
  const [playDeckId, setPlayDeckId] = useState('');
  const [playTierFilter, setPlayTierFilter] = useState<TierFilter>('any');
  const matchmaking = useMatchmaking();

  // When a match is found, join the synthesized lobby by code and navigate
  // to the game — same handoff as the manual "Join by code" flow.
  useEffect(() => {
    if (matchmaking.status !== 'matched' || !matchmaking.match || !playDeckId) return;
    let cancelled = false;
    (async () => {
      try {
        const deck = await deckApiMod.getDeck(playDeckId);
        const result = await lobbyApi.joinLobby(matchmaking.match!.join_code, {
          deck: [...deck.egg_deck, ...deck.main_deck],
        });
        if (!cancelled) {
          navigate(`/game/${result.game_id}?mode=pvp&player=${result.player_id}`);
        }
      } catch (err) {
        console.error('Failed to join matched game:', err);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [matchmaking.status, matchmaking.match, playDeckId, navigate]);

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

  const handleQueue = useCallback(() => {
    if (!playDeckId) return;
    void matchmaking.enqueue({
      queue_type: playQueueType,
      deck_id: playDeckId,
      opponent_tier_filter: playQueueType === 'casual' ? playTierFilter : undefined,
    });
  }, [matchmaking, playDeckId, playQueueType, playTierFilter]);

  // Create tab state
  const [selectedDeckId, setSelectedDeckId] = useState('');
  const [isPublic, setIsPublic] = useState(true);
  const [allowSpectators, setAllowSpectators] = useState(true);
  const [spectatorMode, setSpectatorMode] = useState<'hidden' | 'open'>('hidden');
  const [createdCode, setCreatedCode] = useState<string | null>(null);
  const [createdGameId, setCreatedGameId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [opponentJoined, setOpponentJoined] = useState(false);

  // WebSocket connection to detect when opponent joins
  const wsOptions = useMemo(
    () =>
      createdGameId
        ? {
            gameId: createdGameId,
            role: 'player' as const,
            onPlayerJoined: () => {
              setOpponentJoined(true);
              // Auto-navigate to game when opponent connects
              navigate(`/game/${createdGameId}?mode=pvp&player=1`);
            },
          }
        : null,
    [createdGameId, navigate],
  );
  useWebSocketGame(wsOptions);

  // Join tab state
  const [joinCode, setJoinCode] = useState('');
  const [joinDeckId, setJoinDeckId] = useState('');
  const [joining, setJoining] = useState(false);

  // Browse tab state
  const [games, setGames] = useState<LobbyGame[]>([]);
  const [loadingGames, setLoadingGames] = useState(false);

  const refreshGames = useCallback(async () => {
    setLoadingGames(true);
    try {
      const list = await lobbyApi.listLobbyGames();
      setGames(list);
    } catch {
      // ignore
    } finally {
      setLoadingGames(false);
    }
  }, []);

  // Auto-refresh browse tab
  useEffect(() => {
    if (tab === 'browse') {
      refreshGames();
      const interval = setInterval(refreshGames, 5000);
      return () => clearInterval(interval);
    }
  }, [tab, refreshGames]);

  const handleCreate = async () => {
    if (!selectedDeckId) return;
    setCreating(true);
    try {
      const deck = await deckApiMod.getDeck(selectedDeckId);
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

  const handleJoin = async (code: string, deckId: string) => {
    if (!deckId || !code) return;
    setJoining(true);
    try {
      const deck = await deckApiMod.getDeck(deckId);
      const result = await lobbyApi.joinLobby(code, {
        deck: [...deck.egg_deck, ...deck.main_deck],
      });
      navigate(`/game/${result.game_id}?mode=pvp&player=${result.player_id}`);
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
      className="w-full rounded border border-gray-600 bg-gray-700 px-3 py-2 text-white"
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
      <h1 className="mb-6 text-2xl font-bold text-white">Multiplayer Lobby</h1>

      {/* Tabs */}
      <div className="mb-6 flex gap-2">
        {(['play', 'create', 'join', 'browse'] as const).map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`rounded px-4 py-2 text-sm font-medium ${
              tab === t
                ? 'bg-blue-600 text-white'
                : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
            }`}
          >
            {t === 'play'
              ? 'Play'
              : t === 'create'
                ? 'Create Game'
                : t === 'join'
                  ? 'Join Game'
                  : 'Browse Games'}
          </button>
        ))}
      </div>

      {/* Play (matchmaking queue) Tab */}
      {tab === 'play' && (
        <PlayTab
          decks={savedDecks}
          queueType={playQueueType}
          onQueueType={setPlayQueueType}
          deckId={playDeckId}
          onDeckId={setPlayDeckId}
          tierFilter={playTierFilter}
          onTierFilter={setPlayTierFilter}
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
            <div className="rounded border border-green-600 bg-green-900/30 p-6 text-center">
              <p className="mb-2 text-sm text-gray-300">Share this code with your opponent:</p>
              <p className="mb-4 font-mono text-4xl font-bold tracking-widest text-green-400">
                {createdCode}
              </p>
              <p className="mb-4 text-sm text-gray-400">
                {opponentJoined
                  ? 'Opponent joined! Redirecting to game...'
                  : 'Waiting for opponent to join...'}
              </p>
              <div className="flex justify-center gap-3">
                <button
                  onClick={handleCancel}
                  disabled={opponentJoined}
                  className="rounded bg-red-700 px-4 py-2 text-white hover:bg-red-600 disabled:opacity-50"
                >
                  Cancel
                </button>
              </div>
            </div>
          ) : (
            <>
              <div>
                <label className="mb-1 block text-sm text-gray-300">Your Deck</label>
                <DeckSelect value={selectedDeckId} onChange={setSelectedDeckId} />
              </div>
              <div className="flex items-center gap-4">
                <label className="flex items-center gap-2 text-sm text-gray-300">
                  <input
                    type="checkbox"
                    checked={isPublic}
                    onChange={(e) => setIsPublic(e.target.checked)}
                  />
                  List in public lobby
                </label>
                <label className="flex items-center gap-2 text-sm text-gray-300">
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
                  <label className="mb-1 block text-sm text-gray-300">Spectator Mode</label>
                  <select
                    value={spectatorMode}
                    onChange={(e) => setSpectatorMode(e.target.value as 'hidden' | 'open')}
                    className="rounded border border-gray-600 bg-gray-700 px-3 py-2 text-white"
                  >
                    <option value="hidden">Hidden (hands/security hidden)</option>
                    <option value="open">Open (full visibility)</option>
                  </select>
                </div>
              )}
              <button
                onClick={handleCreate}
                disabled={!selectedDeckId || creating}
                className="rounded bg-blue-600 px-6 py-2 text-white hover:bg-blue-500 disabled:opacity-50"
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
            <label className="mb-1 block text-sm text-gray-300">Join Code</label>
            <input
              type="text"
              value={joinCode}
              onChange={(e) => setJoinCode(e.target.value.toUpperCase())}
              placeholder="ABC123"
              maxLength={6}
              className="w-full rounded border border-gray-600 bg-gray-700 px-3 py-2 font-mono text-lg tracking-widest text-white placeholder-gray-500"
            />
          </div>
          <div>
            <label className="mb-1 block text-sm text-gray-300">Your Deck</label>
            <DeckSelect value={joinDeckId} onChange={setJoinDeckId} />
          </div>
          <button
            onClick={() => handleJoin(joinCode, joinDeckId)}
            disabled={!joinCode || !joinDeckId || joining}
            className="rounded bg-green-600 px-6 py-2 text-white hover:bg-green-500 disabled:opacity-50"
          >
            {joining ? 'Joining...' : 'Join Game'}
          </button>
        </div>
      )}

      {/* Browse Tab */}
      {tab === 'browse' && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <p className="text-sm text-gray-400">
              {games.length} game{games.length !== 1 ? 's' : ''} available
            </p>
            <button
              onClick={refreshGames}
              disabled={loadingGames}
              className="text-sm text-blue-400 hover:text-blue-300"
            >
              {loadingGames ? 'Refreshing...' : 'Refresh'}
            </button>
          </div>
          {games.length === 0 ? (
            <p className="py-8 text-center text-gray-500">
              No public games available. Create one or join with a code.
            </p>
          ) : (
            <div className="space-y-2">
              {games.map((game) => (
                <div
                  key={game.game_id}
                  className="flex items-center justify-between rounded border border-gray-700 bg-gray-800 p-3"
                >
                  <div>
                    <span className="font-medium text-white">{game.host_display_name}</span>
                    <span className="ml-3 font-mono text-sm text-gray-400">{game.join_code}</span>
                  </div>
                  <div className="flex items-center gap-3">
                    <DeckSelect value={joinDeckId} onChange={setJoinDeckId} />
                    <button
                      onClick={() => handleJoin(game.join_code, joinDeckId)}
                      disabled={!joinDeckId || joining}
                      className="whitespace-nowrap rounded bg-green-600 px-3 py-1 text-sm text-white hover:bg-green-500 disabled:opacity-50"
                    >
                      Join
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}


function tierBadge(tier: string | null | undefined): { label: string; className: string } {
  if (tier === 'meta') return { label: 'meta', className: 'bg-red-700 text-red-100' };
  if (tier === 'rogue') return { label: 'rogue', className: 'bg-amber-700 text-amber-100' };
  if (tier === 'jank') return { label: 'jank', className: 'bg-emerald-700 text-emerald-100' };
  return { label: 'unclassified', className: 'bg-gray-700 text-gray-300' };
}


interface PlayTabProps {
  decks: DeckSummary[];
  queueType: QueueType;
  onQueueType: (q: QueueType) => void;
  deckId: string;
  onDeckId: (id: string) => void;
  tierFilter: TierFilter;
  onTierFilter: (f: TierFilter) => void;
  onQueue: () => void;
  onCancel: () => void;
  status: ReturnType<typeof useMatchmaking>['status'];
  match: ReturnType<typeof useMatchmaking>['match'];
  waitedSeconds: number;
  ratingWindow: number | null;
  errorMsg: string | null;
}

function PlayTab(props: PlayTabProps) {
  const {
    decks, queueType, onQueueType, deckId, onDeckId, tierFilter, onTierFilter,
    onQueue, onCancel, status, match, waitedSeconds, ratingWindow, errorMsg,
  } = props;
  const selectedDeck = decks.find((d) => d.id === deckId);
  const badge = selectedDeck ? tierBadge(selectedDeck.meta_tier) : null;
  const isQueued = status === 'waiting' || status === 'connecting';
  const isTerminal = status === 'matched' || status === 'error' || status === 'cancelled';

  if (isQueued) {
    const label = queueType === 'ranked'
      ? `Searching ranked — window ±${ratingWindow ?? 50}`
      : 'Searching casual...';
    return (
      <div className="rounded border border-blue-600 bg-blue-900/20 p-8 text-center">
        <div className="mb-2 text-lg font-medium text-white">{label}</div>
        <div className="mb-4 font-mono text-3xl text-blue-300">
          {Math.floor(waitedSeconds / 60)}:{String(waitedSeconds % 60).padStart(2, '0')}
        </div>
        <button
          onClick={onCancel}
          className="rounded bg-red-700 px-4 py-2 text-white hover:bg-red-600"
        >
          Cancel
        </button>
      </div>
    );
  }

  if (status === 'matched' && match) {
    return (
      <div className="rounded border border-green-600 bg-green-900/20 p-8 text-center">
        <div className="mb-2 text-lg font-medium text-white">Match found!</div>
        {match.opponent.display_name ? (
          <p className="mb-2 text-gray-300">vs {match.opponent.display_name}</p>
        ) : null}
        <p className="text-sm text-gray-400">Connecting to game…</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {isTerminal && errorMsg ? (
        <div className="rounded border border-red-600 bg-red-900/20 p-3 text-sm text-red-200">
          {errorMsg}
        </div>
      ) : null}

      <div>
        <label className="mb-1 block text-sm text-gray-300">Queue</label>
        <div className="flex gap-2">
          {(['casual', 'ranked'] as const).map((q) => (
            <button
              key={q}
              onClick={() => onQueueType(q)}
              className={`flex-1 rounded px-4 py-2 text-sm font-medium ${
                queueType === q
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
              }`}
            >
              {q === 'casual' ? 'Casual' : 'Ranked'}
            </button>
          ))}
        </div>
        <p className="mt-1 text-xs text-gray-500">
          {queueType === 'casual'
            ? 'Filtered by opponent tier; no rating changes.'
            : 'Matched by skill rating; rating updates after the game.'}
        </p>
      </div>

      <div>
        <label className="mb-1 block text-sm text-gray-300">Your Deck</label>
        <select
          value={deckId}
          onChange={(e) => onDeckId(e.target.value)}
          className="w-full rounded border border-gray-600 bg-gray-700 px-3 py-2 text-white"
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
            <span className={`rounded px-2 py-0.5 font-medium ${badge.className}`}>
              {badge.label}
            </span>
            {selectedDeck.meta_archetype ? (
              <span className="text-gray-400">→ {selectedDeck.meta_archetype}</span>
            ) : null}
            <span className="text-gray-500">· format: {selectedDeck.game_mode}</span>
          </div>
        ) : null}
      </div>

      {queueType === 'casual' && (
        <div>
          <label className="mb-1 block text-sm text-gray-300">Opponent</label>
          <select
            value={tierFilter}
            onChange={(e) => onTierFilter(e.target.value as TierFilter)}
            className="w-full rounded border border-gray-600 bg-gray-700 px-3 py-2 text-white"
          >
            <option value="any">Any — face anyone</option>
            <option value="same">Same tier as my deck</option>
            <option value="meta_only">Meta only — try to beat the best</option>
            <option value="jank_only">Jank only — off-meta safe space</option>
          </select>
        </div>
      )}

      <button
        onClick={onQueue}
        disabled={!deckId}
        className="w-full rounded bg-blue-600 px-6 py-3 text-base font-medium text-white hover:bg-blue-500 disabled:opacity-50"
      >
        Find Match
      </button>
    </div>
  );
}
