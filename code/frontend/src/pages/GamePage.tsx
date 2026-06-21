import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import { useNavigate, useParams, useSearchParams } from 'react-router-dom';
import {
  DndContext,
  PointerSensor,
  TouchSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from '@dnd-kit/core';
import { useGameStore } from '@/stores/gameStore';
import { useActionMask } from '@/hooks/useActionMask';
import { useEffectHighlight } from '@/hooks/useEffectHighlight';
import { useDropZone, type DragData } from '@/hooks/useDropZone';
import { GameBoard } from '@/components/board/GameBoard';
import { ActionBar } from '@/components/game/ActionBar';
import { PhaseIndicator } from '@/components/game/PhaseIndicator';
import { PromptBar } from '@/components/game/PromptBar';
import { SelectionPanel } from '@/components/game/SelectionPanel';
import { TrashSelectModal } from '@/components/game/TrashSelectModal';
import { TrashViewer } from '@/components/game/TrashViewer';
import { ResultOverlay } from '@/components/game/ResultOverlay';
import { AttackArrow } from '@/components/game/AttackArrow';
import { PhaseBanner } from '@/components/game/PhaseBanner';
import { DigivolveBanner } from '@/components/game/DigivolveBanner';
import { BattleEffect } from '@/components/game/BattleEffect';
import { CardOverlay } from '@/components/game/CardOverlay';
import { CardDetailOverlay } from '@/components/game/CardDetailOverlay';
import { GameLogDrawer } from '@/components/game/GameLogDrawer';
import { SecurityRevealOverlay } from '@/components/board/SecurityRevealOverlay';
import { EffectPopup } from '@/components/game/EffectPopup';
import { KeywordPromptDialog } from '@/components/game/KeywordPromptDialog';
import { PeekButton } from '@/components/game/PeekButton';
import { DragOverlayCard } from '@/components/game/DragOverlayCard';
import { ConnectionStatusScreen } from '@/components/game/ConnectionStatusScreen';
import { useWebSocketGame, type UseWebSocketGameOptions } from '@/hooks/useWebSocketGame';
import { connectionScreenState } from '@/utils/connectionScreen';
import { pickAlias, liveGameLabels } from '@/features/play/botNames';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import * as gameApi from '@/api/gameApi';
import * as deckApiMod from '@/api/deckApi';
import * as deckStore from '@/storage/deckStore';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import type { LocalModelMeta } from '@/api/desktopModelsApi';
import {
  ACTION,
  ATTACK_TARGETS_PER_SLOT,
  ATTACK_TARGET_SECURITY,
  BREEDING_SLOT,
  DIGIVOLVE_FIELDS_PER_HAND,
  FIELD_SLOTS,
  SELECTION,
} from '@/utils/constants';
import {
  anyFieldSelectionActionId,
  anyFieldSelectionHighlights,
  fieldSelectionActionId,
  fieldSelectionHighlights,
  isAnyFieldSelectionKind,
  isFieldSelectionKind,
} from '@/utils/selectionTargets';
import {
  GamePhase,
  type ActionTrace,
  type GameEvent,
  type GameState,
  type PermanentInfo,
} from '@/types/game';
import { useUiStore } from '@/stores/uiStore';
import { BotSpeedControl } from '@/components/game/BotSpeedControl';
import { usePacedAgentDriver } from '@/hooks/usePacedAgentDriver';
import { formatEvents } from '@/utils/gameLogFormat';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';
type ActionTraceResponse = { action_traces?: ActionTrace[] };
// Stable empty mask used to lock all mask-derived affordances while a paced
// agent sequence is draining (the response mask belongs to the AGENT during
// those beats — rendering it would light up wrong affordances).
const EMPTY_MASK: number[] = [];
// Desktop builds have no server-side Deck rows, so deck list/load must go
// through the Tauri-backed deckStore. parseDeck / validateDeckRaw already
// branch internally, so those stay on deckApiMod.
const decks = IS_DESKTOP ? deckStore : deckApiMod;

export function GamePage() {
  const { id: urlGameId } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const mode = searchParams.get('mode');
  const isPvpMode = mode === 'pvp';
  const isVsAiOnline = mode === 'vsai';
  const isSpectator = searchParams.get('role') === 'spectator';
  // WebSocket is used for both PvP and vs-AI-online: the server streams
  // state and runs the AI turn internally for the 'vsai' mode.
  const useWebSocket = isPvpMode || isVsAiOnline || isSpectator;


  const store = useGameStore();
  const { seed: flowSeed, clearLaunchState } = usePlayFlowStore();
  useEffectHighlight();
  // Bot action pacing (add-bot-action-pacing): non-instant speeds advance
  // local bot games one agent action per request, paced by the driver
  // effect below. WebSocket games are server-paced and never use this.
  const botSpeed = useUiStore((s) => s.botSpeed);
  const paced = !useWebSocket && botSpeed !== 'instant';
  // While the bot's beats are draining, all mask-derived affordances are
  // locked — the in-flight mask belongs to the agent, not the human.
  const parsedMask = useActionMask(store.agentPending ? EMPTY_MASK : store.actionMask);
  const actionPendingRef = useRef(false);

  // WebSocket connection (active for PvP, vs-AI-online, and spectator modes)
  const wsOptions = useMemo<UseWebSocketGameOptions | null>(() => {
    if (!urlGameId || !useWebSocket) return null;
    return {
      gameId: urlGameId,
      role: isSpectator ? 'spectator' : 'player',
      onStateUpdate: (payload) => {
        store.setGameState(payload.state);
        store.setActionMask(payload.action_mask ?? []);
        if (payload.seed !== undefined) {
          store.setGameSeed(payload.seed ?? null);
        }
        if (payload.events) store.appendEvents(payload.events);
        // Opponent shown under a cosmetic Digimon alias, seeded by game id so
        // it stays constant across state updates / reloads and is identical on
        // both clients and for spectators. The local player keeps "You";
        // spectators (no seat) see both seats aliased. (anonymize-opponent-names)
        store.setPlayerLabels(liveGameLabels(urlGameId, payload.your_player_id));
      },
      onGameOver: () => {},
      onError: (msg) => console.error('WebSocket error:', msg),
    };
  }, [urlGameId, useWebSocket, isSpectator, isVsAiOnline]); // eslint-disable-line react-hooks/exhaustive-deps

  const ws = useWebSocketGame(wsOptions);

  const appendResponseActionTraces = useCallback(
    (response: ActionTraceResponse) => {
      if (response.action_traces?.length) {
        store.appendActionTraces(response.action_traces);
      }
    },
    [store],
  );

  // Apply any step/action-shaped response to the store, including the
  // paced-mode `agent_pending` flag the pacing driver keys off.
  const applyGameResponse = useCallback(
    (res: {
      state: GameState;
      action_mask: number[];
      logs?: string[];
      events?: GameEvent[];
      agent_pending?: boolean;
      seed?: string;
    } & ActionTraceResponse) => {
      store.setGameState(res.state);
      store.setActionMask(res.action_mask);
      if (res.seed !== undefined) {
        store.setGameSeed(res.seed ?? null);
      }
      if (res.events) store.appendEvents(res.events);
      appendResponseActionTraces(res);
      store.setAgentPending(res.agent_pending ?? false);
    },
    [appendResponseActionTraces, store],
  );

  // Use WebSocket sendAction for PvP / vs-AI-online / spectator; HTTP for local games.
  const sendAction = useCallback(
    async (actionId: number) => {
      if (useWebSocket) {
        ws.sendAction(actionId);
        return;
      }
      if (!store.gameId || actionPendingRef.current) return;
      // Paced bot sequence draining — the human has no legal input until
      // the beats finish (mask is locked too; this is a belt-and-braces
      // guard against queued clicks).
      if (store.agentPending) return;
      actionPendingRef.current = true;

      try {
        const actionResult = await gameApi.sendAction(store.gameId, actionId, {
          paced,
        });
        applyGameResponse(actionResult);

        // If the human is still mid-decision — a multi-stage selection like
        // DNA digivolve's two material picks leaves a pending_selection
        // owned by the local player — do NOT run the agent-autoplay step.
        // Stepping here is a no-op server-side but keeps `actionPendingRef`
        // set across the await, which would drop the player's very next
        // click (e.g. the second DNA material). Let the player continue.
        const humanStillDeciding =
          actionResult.state.pendingSelection?.selectingPlayer === 1;

        // Paced mode: the response already executed at most one agent beat;
        // the pacing driver effect advances the rest on a timer. Only the
        // legacy instant path drains agents inline here.
        if (!paced && !actionResult.is_game_over && !humanStillDeciding) {
          const stepResult = await gameApi.stepGame(store.gameId);
          applyGameResponse(stepResult);
        }
      } finally {
        actionPendingRef.current = false;
      }
    },
    [applyGameResponse, paced, store, useWebSocket, ws],
  );

  // Pacing driver: while the bot has actions left, request the next beat
  // after the configured delay; retry-once + stall-with-Continue on
  // failure. Inert for WebSocket games (server-paced).
  const { pacingStalled, resumePacing } = usePacedAgentDriver({
    active: !useWebSocket,
    applyGameResponse,
  });
  const { savedDecks, setSavedDecks } = useDeckBuilderStore();
  const { getDropAction } = useDropZone(parsedMask);

  const [selectedDeckId, setSelectedDeckId] = useState<string>('');
  const [opponentDeckId, setOpponentDeckId] = useState<string>('');
  const [agentType, setAgentType] = useState<string>('greedy');
  const [starting, setStarting] = useState(false);
  const [startSeedInput, setStartSeedInput] = useState('');
  const [startSeedError, setStartSeedError] = useState('');
  const [localModels, setLocalModels] = useState<LocalModelMeta[]>([]);
  const [selectedModelId, setSelectedModelId] = useState<string>('');

  // Desktop only — hydrate the model list so the "Trained (ONNX)" option
  // can populate its dropdown. Silently no-ops on the web build.
  useEffect(() => {
    if (!IS_DESKTOP) return;
    let cancelled = false;
    void (async () => {
      try {
        const mod = await import('@/api/desktopModelsApi');
        const models = await mod.listLocal();
        if (!cancelled) setLocalModels(models);
      } catch {
        // Non-fatal — leave list empty; user can still pick greedy/random.
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  // Refresh the engine-decoded action list whenever the legal-action set
  // changes. The action bar renders activatable effects (card + effect name)
  // from this instead of re-deriving labels from raw mask ranges. Cleared
  // while the agent is acting or the game is over. WebSocket modes (PvP /
  // vs-AI-online / spectator) drive state through their own channel and the
  // /games decoded-actions endpoint may not back them — they fall back to the
  // action bar's mask-based rendering, so we skip the fetch there.
  useEffect(() => {
    const gid = store.gameId;
    if (!gid || useWebSocket || store.agentPending || store.isGameOver) {
      store.setDecodedActions([]);
      return;
    }
    let cancelled = false;
    void (async () => {
      try {
        const actions = await gameApi.getDecodedActions(gid);
        if (!cancelled) store.setDecodedActions(actions);
      } catch {
        if (!cancelled) store.setDecodedActions([]);
      }
    })();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    store.gameId,
    store.actionMask,
    store.agentPending,
    store.isGameOver,
    useWebSocket,
  ]);

  // Hydrate store from URL param when navigating to /game/:id
  useEffect(() => {
    if (urlGameId && !store.gameId) {
      if (useWebSocket) {
        // PvP / vs-AI-online / spectator: WebSocket hook will send initial state
        store.setGameId(urlGameId);
        store.clearActionTraces();
      } else {
        // Local mode: fetch state via HTTP
        (async () => {
          try {
            const [state, maskData] = await Promise.all([
              gameApi.getState(urlGameId),
              gameApi.getMask(urlGameId),
            ]);
            const stateSeed = (state as GameState & { seed?: string }).seed;
            store.setGameId(urlGameId);
            store.setGameState(state);
            store.setActionMask(maskData);
            store.setGameSeed(stateSeed ?? flowSeed ?? null);
            store.clearActionTraces();
            store.setPlayerLabels({
              1: 'YOU',
              2: pickAlias(`${urlGameId}:2`),
            });
            // Lock human input and drive any opening agent decision (e.g. the
            // AI's mulligan when the human goes second) before handing control
            // back. Games created via createBotGame/createAiStarterGame land
            // here in the Mulligan phase; without this step the human lands on
            // the AI's mulligan prompt and their first click is consumed
            // resolving the AI's decision — the "mulligan twice" bug. Setting
            // agentPending first blanks the mask so no click slips through the
            // load window.
            store.setAgentPending(true);
            const stepResult = await gameApi.stepGame(urlGameId, { paced });
            applyGameResponse(stepResult);
          } catch (err) {
            console.error('Failed to load game:', err);
            store.setAgentPending(false);
          }
        })();
      }
    }
  }, [urlGameId]); // eslint-disable-line react-hooks/exhaustive-deps

  // Dev-only: the desktop debug bridge (Tauri feature `debug-bridge`) can
  // stage / mutate the game out of band (driven by the scenario MCP). It
  // emits `debug:state-changed` after each external mutation; refetch so
  // the board reflects the staged state without a manual reload. Inert
  // outside the Tauri runtime (browser-dev / web) — the event API is
  // imported lazily and only when Tauri's IPC hooks are present.
  useEffect(() => {
    if (typeof window === 'undefined') return;
    const hasTauri = Boolean(
      (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__,
    );
    if (!hasTauri) return;
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    (async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const un = await listen('debug:state-changed', async () => {
          try {
            const gid = useGameStore.getState().gameId ?? 'rust-local';
            const [state, maskData] = await Promise.all([
              gameApi.getState(gid),
              gameApi.getMask(gid),
            ]);
            store.setGameState(state);
            store.setActionMask(maskData);
          } catch (err) {
            console.error('debug bridge refresh failed:', err);
          }
        });
        if (cancelled) un();
        else unlisten = un;
      } catch {
        // Tauri event API unavailable (browser-dev) — no bridge here.
      }
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const [inspectedPerm, setInspectedPerm] = useState<PermanentInfo | null>(null);
  // Right-click card inspect (DCGO CardDetail) — enlarged single-card view,
  // opened from the hand or from a card image in the stack inspector.
  const [inspectedCardId, setInspectedCardId] = useState<string | null>(null);
  const [draggedCardId, setDraggedCardId] = useState<string | null>(null);
  const [draggedHandIndex, setDraggedHandIndex] = useState<number | null>(null);
  const [isOverValid, setIsOverValid] = useState(false);
  // Action choice dialog: shown when the same hand card can be used in more
  // than one way (play / regular digivolve / DNA digivolve). Mirrors DCGO's
  // "With which method would you like to Digivolve?" modal.
  const [actionChoice, setActionChoice] = useState<{
    handIndex: number;
    canPlay: boolean;
    digivolveTargets: Set<number>; // field slot indices
    canDigivolveBreeding: boolean;
    canDnaDigivolve: boolean;
  } | null>(null);
  // When set, slot clicks are interpreted as digivolve targets
  const [digivolvingHandIndex, setDigivolvingHandIndex] = useState<number | null>(null);
  // Trash viewer state: null = closed, 1 = own trash, 2 = opponent trash
  const [trashViewerPlayer, setTrashViewerPlayer] = useState<number | null>(null);
  // Hovered hand card index for memory cost preview
  const [hoveredHandIndex, setHoveredHandIndex] = useState<number | null>(null);
  // Track which player surrendered (if any)
  const [surrenderedBy, setSurrenderedBy] = useState<number | null>(null);
  const boardContainerRef = useRef<HTMLDivElement>(null);

  // Load saved decks on first render
  useState(() => {
    decks.listDecks().then(setSavedDecks).catch(() => {});
  });

  // DnD sensors: pointer with 8px activation distance, touch with 150ms delay
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
    useSensor(TouchSensor, { activationConstraint: { delay: 150, tolerance: 5 } }),
  );

  const handleStartGame = async () => {
    if (!selectedDeckId || !opponentDeckId) return;
    if (agentType === 'trained' && !selectedModelId) return;
    setStarting(true);
    try {
      let normalizedSeed: string | null = null;
      try {
        normalizedSeed = gameApi.normalizeSeedInput(startSeedInput);
        setStartSeedError('');
      } catch (err) {
        setStartSeedError((err as Error).message);
        setStarting(false);
        return;
      }
      const [deck, oppDeck] = await Promise.all([
        decks.getDeck(selectedDeckId),
        decks.getDeck(opponentDeckId),
      ]);

      // Desktop + trained: ensure the selected model is loaded into the
      // inference cache before we ask the engine to create a game that
      // references it. `loadCached` is idempotent, so re-activating the
      // same model across games is cheap.
      if (IS_DESKTOP && agentType === 'trained' && selectedModelId) {
        const mod = await import('@/api/desktopModelsApi');
        await mod.loadCached(selectedModelId);
      }

      const result = await gameApi.createGame({
        deck1: [...deck.egg_deck, ...deck.main_deck],
        deck2: [...oppDeck.egg_deck, ...oppDeck.main_deck],
        player1_type: 'human',
        player2_type: 'agent',
        player2_policy: agentType,
        player_kinds: [
          'human',
          agentType === 'trained' ? 'trained' : 'greedy',
        ],
        player_model_ids: [
          null,
          agentType === 'trained' ? selectedModelId : null,
        ],
        // Paced mode: the browser wire's create runs an opening agent
        // prelude — cap it at one beat so the opening bot turn is paced
        // too. (Desktop create runs no prelude; the flag is inert there.)
        paced,
        seed: normalizedSeed,
      });

      // Set game state before gameId so the board has player data when it first renders
      store.setGameState(result.state);
      store.setActionMask(result.action_mask);
      store.setGameSeed(result.seed ?? normalizedSeed);
      // Client owns opponent naming: alias the AI seat (seeded by game id so it
      // is stable for the game) and keep "YOU" for the human, ignoring any
      // backend-provided opponent label. (anonymize-opponent-names)
      store.setPlayerLabels({
        1: 'YOU',
        2: pickAlias(`${result.game_id}:2`),
      });
      store.clearEvents();
      store.clearActionTraces();
      if (result.events) store.appendEvents(result.events);
      store.setAgentPending(result.agent_pending ?? false);
      store.setGameId(result.game_id);

      // Step once to handle initial agent turn if agent goes first. Paced
      // mode advances a single beat; the pacing driver takes it from there.
      const stepResult = await gameApi.stepGame(result.game_id, { paced });
      applyGameResponse(stepResult);
    } catch (err) {
      console.error('Failed to create game:', err);
      // If gameId was set but step failed, reset to avoid blank board
      if (store.gameId) store.reset();
    } finally {
      setStarting(false);
    }
  };

  const handleAction = useCallback(
    async (actionId: number) => {
      await sendAction(actionId);
    },
    [sendAction],
  );

  const handleUndo = useCallback(async () => {
    if (!store.gameId) return;
    try {
      const res = await gameApi.undoGame(store.gameId);
      if (!res) return; // Tauri path returns null — undo disabled there
      store.setGameState(res.state);
      store.setActionMask(res.action_mask);
      store.clearEvents();
      store.clearActionTraces();
      if (res.events) store.appendEvents(res.events);
      // Clear any in-flight UI selection state so we land on the
      // replayed snapshot with a clean slate.
      store.setAgentPending(res.agent_pending ?? false);
      setActionChoice(null);
      setDigivolvingHandIndex(null);
      setSurrenderedBy(null);
    } catch (err) {
      console.error('Undo failed:', err);
    }
  }, [store]);

  const handleSurrender = useCallback(async () => {
    if (!store.gameId || store.isGameOver) return;
    // PvP / vs-AI-online run over the WebSocket: concede there (the REST
    // surrenderGame path only drives local games and would never reach the
    // live game's server-side runner). The server broadcasts the resulting
    // game-over state through the normal state_update path. The viewer is
    // perspective-normalized to player 1, so they surrendered as "you" (1).
    if (useWebSocket) {
      ws.sendSurrender();
      setSurrenderedBy(1);
      return;
    }
    try {
      const res = await gameApi.surrenderGame(store.gameId, 1);
      store.setGameState(res.state);
      store.setActionMask(res.action_mask);
      if (res.events) store.appendEvents(res.events);
      appendResponseActionTraces(res as typeof res & ActionTraceResponse);
      store.setAgentPending(false);
      setSurrenderedBy(1);
    } catch {
      // Ignore errors (e.g. game already over)
    }
  }, [appendResponseActionTraces, store, useWebSocket, ws]);

  const handleReturnToLauncher = useCallback(() => {
    store.reset();
    clearLaunchState();
    navigate('/');
  }, [clearLaunchState, navigate, store]);

  // Bail out of a WebSocket game whose connection never came up. Clears the
  // game state so we don't re-enter the dead board, then returns to the play
  // screen (the natural retry point for PvP / vs-AI-online / spectator).
  const handleLeaveConnection = useCallback(() => {
    store.reset();
    clearLaunchState();
    navigate('/play');
  }, [clearLaunchState, navigate, store]);

  const gameLogLines = useMemo(() => {
    if (!store.player1 || !store.player2) return [];
    return formatEvents(store.events, {
      state: {
        turnCount: store.turnCount,
        currentPhase: store.currentPhase,
        currentPlayer: store.currentPlayer,
        firstPlayer: store.firstPlayer,
        memoryGauge: store.memoryGauge,
        isGameOver: store.isGameOver,
        winner: store.winner,
        player1: store.player1,
        player2: store.player2,
        revealedCards: store.revealedCards,
        pendingSelection: store.pendingSelection,
        pendingAttack: store.pendingAttack,
      },
      playerLabels: store.playerLabels,
    });
  }, [
    store.currentPhase,
    store.currentPlayer,
    store.events,
    store.isGameOver,
    store.memoryGauge,
    store.pendingAttack,
    store.pendingSelection,
    store.player1,
    store.player2,
    store.playerLabels,
    store.revealedCards,
    store.turnCount,
    store.winner,
  ]);

  const handlePlayCard = useCallback(
    (handIndex: number) => {
      if (store.currentPhase === GamePhase.Mulligan) {
        return;
      }
      const canPlay = parsedMask.canPlayFromHand.has(handIndex);
      const fieldTargets = parsedMask.canDigivolve.get(handIndex);
      const canBreeding = parsedMask.canDigivolveBreeding.has(handIndex);
      const canDigi = (fieldTargets && fieldTargets.size > 0) || canBreeding;
      const canDna = parsedMask.canDnaDigivolve.has(handIndex);

      // If more than one method is legal, show the method-picker modal so
      // the user (and the RL action space) chooses explicitly — DCGO does
      // exactly the same thing for the "Normal vs DNA Digivolution" prompt.
      const methodCount = (canPlay ? 1 : 0) + (canDigi ? 1 : 0) + (canDna ? 1 : 0);
      if (methodCount > 1) {
        setActionChoice({
          handIndex,
          canPlay,
          digivolveTargets: fieldTargets ?? new Set(),
          canDigivolveBreeding: canBreeding,
          canDnaDigivolve: canDna,
        });
        return;
      }

      if (canPlay) {
        handleAction(handIndex);
        return;
      }

      if (canDigi) {
        // Only digivolve available — resolve target
        const allTargets = new Set(fieldTargets ?? []);
        if (canBreeding) allTargets.add(BREEDING_SLOT);
        if (allTargets.size === 1) {
          const target = allTargets.values().next().value!;
          handleAction(ACTION.DIGIVOLVE_START + handIndex * DIGIVOLVE_FIELDS_PER_HAND + target);
        } else {
          // Multiple targets — enter digivolve target mode
          setDigivolvingHandIndex(handIndex);
        }
        return;
      }

      if (canDna) {
        // Only DNA available — dispatch the DNA action. The engine will
        // immediately push a `pending_selection` for the first material
        // ("Select a level 4 Green Digimon" in DCGO's prompt), and the
        // existing SelectionPanel + slot-click routing handles both
        // materials with no extra UI plumbing needed.
        handleAction(ACTION.DNA_START + handIndex);
      }
    },
    [parsedMask, handleAction, store.currentPhase],
  );

  const handleActionChoicePlay = useCallback(() => {
    if (actionChoice) {
      handleAction(actionChoice.handIndex);
      setActionChoice(null);
    }
  }, [actionChoice, handleAction]);

  const handleActionChoiceDigivolve = useCallback(() => {
    if (!actionChoice) return;
    const { handIndex, digivolveTargets, canDigivolveBreeding: canBreeding } = actionChoice;
    const allTargets = new Set(digivolveTargets);
    if (canBreeding) allTargets.add(BREEDING_SLOT);
    setActionChoice(null);
    if (allTargets.size === 1) {
      const target = allTargets.values().next().value!;
      handleAction(ACTION.DIGIVOLVE_START + handIndex * DIGIVOLVE_FIELDS_PER_HAND + target);
    } else {
      // Multiple targets — enter digivolve target selection mode
      setDigivolvingHandIndex(handIndex);
    }
  }, [actionChoice, handleAction]);

  const handleActionChoiceDna = useCallback(() => {
    if (!actionChoice) return;
    const { handIndex } = actionChoice;
    setActionChoice(null);
    // Fire the DNA action. Material picks (two own-field permanents) come
    // back as engine-driven `pending_selection` prompts that the existing
    // SelectionPanel + handleSlotClick already render and route.
    handleAction(ACTION.DNA_START + handIndex);
  }, [actionChoice, handleAction]);

  const handleSlotClick = useCallback(
    (isOpponent: boolean, slotIndex: number) => {
      const phase = store.currentPhase;

      // Digivolve target selection mode
      if (digivolvingHandIndex !== null && !isOpponent) {
        const fieldTargets = parsedMask.canDigivolve.get(digivolvingHandIndex);
        if (fieldTargets?.has(slotIndex)) {
          const actionId = ACTION.DIGIVOLVE_START + digivolvingHandIndex * DIGIVOLVE_FIELDS_PER_HAND + slotIndex;
          handleAction(actionId);
          setDigivolvingHandIndex(null);
          return;
        }
        // Cancel digivolve selection on invalid click
        setDigivolvingHandIndex(null);
        return;
      }

      // DNA digivolve material selection. The engine surfaces each of the
      // two material picks as a PendingSelection whose `valid_action_ids`
      // are the RAW battle_area indices of the eligible own-field
      // permanents — NOT the 100+ `SELECTION.OWN_FIELD_START` encoding the
      // other selection phases use. So clicking an own-field permanent
      // dispatches that raw field index directly. Must run before the
      // generic selection branch below (SelectMaterial is inside the
      // SelectTarget..SelectSecurity range) so it isn't mis-encoded as
      // 100+slot. See `Game::initiate_dna_digivolve` (valid_action_ids =
      // battle_area indices) + `dna_digivolve_user_action.rs`.
      if (phase === GamePhase.SelectMaterial && !isOpponent) {
        // Read the LIVE mask via getState(), not the closure's
        // `store.actionMask`: the two DNA material picks happen in quick
        // succession and the second click can fire before this callback's
        // closure re-renders with the second-stage mask. The live mask is
        // always current, so the second material's raw index dispatches
        // reliably.
        const liveMask = useGameStore.getState().actionMask;
        if (slotIndex < FIELD_SLOTS && liveMask[slotIndex] === 1) {
          handleAction(slotIndex);
        }
        // Ignore clicks on non-eligible permanents; don't fall through to
        // the 100+ mapping or attack logic.
        return;
      }

      // Map a board click to a field-target selection. The engine encodes
      // own- and opponent-field targets in the same `OWN_FIELD_START + slot`
      // range and disambiguates by `pendingSelection.kind`;
      // `fieldSelectionActionId` honours that (the old `isOpponent ?
      // ENEMY_FIELD_START` branch never matched the engine's ids, so "delete
      // an opponent's Digimon" prompts swallowed every click).
      //
      // Gate on the selection KIND, NOT a phase range: single-target field
      // prompts run in `SelectTarget`, but capped-multi-select field prompts
      // ("delete up to 2 of your opponent's Digimon") run in `SelectBudgeted`,
      // which a `SelectTarget..SelectSecurity` range excluded — leaving those
      // multi-select prompts unclickable.
      if (isFieldSelectionKind(store.pendingSelection?.kind)) {
        const selIdx = fieldSelectionActionId(
          store.pendingSelection?.kind,
          isOpponent,
          slotIndex,
          parsedMask.validSelections,
        );
        if (selIdx !== null) {
          handleAction(selIdx);
          return;
        }
      }

      // `AnyField` selections (`select_any_permanent` / `select_dna_pair`) span
      // BOTH battle areas and encode the engine player in the action id
      // (`encode_attack(player, index)`), so route by decoding the id rather
      // than the single-side `OwnField`/`OppField` kind. Without this branch,
      // "place 1 Digimon as the bottom security card"-style prompts (EX8-028
      // Skadimon) swallowed every click — the softlock.
      if (isAnyFieldSelectionKind(store.pendingSelection?.kind)) {
        const selIdx = anyFieldSelectionActionId(
          store.pendingSelection?.kind,
          isOpponent,
          slotIndex,
          parsedMask.validSelections,
          store.pendingSelection?.selectingPlayer,
        );
        if (selIdx !== null) {
          handleAction(selIdx);
          return;
        }
      }

      // During BlockTiming, slots 100-111 select a blocker
      if (phase === GamePhase.BlockTiming && !isOpponent) {
        const blockAction = SELECTION.OWN_FIELD_START + slotIndex;
        if (store.actionMask[blockAction] === 1) {
          handleAction(blockAction);
          return;
        }
      }

      // In main phase with attacker selected, compute attack action
      if (store.selectedAttacker !== null) {
        const attacker = store.selectedAttacker;
        if (isOpponent) {
          const actionId = ACTION.ATTACK_START + attacker * ATTACK_TARGETS_PER_SLOT + slotIndex;
          if (store.actionMask[actionId] === 1) {
            handleAction(actionId);
            store.selectAttacker(null);
            return;
          }
        }
        // Click security (conceptual — we'll handle via button)
        store.selectAttacker(null);
        return;
      }

      // Select attacker
      if (!isOpponent && parsedMask.canAttack.has(slotIndex)) {
        store.selectAttacker(slotIndex);
        return;
      }

      // Inspect on right-click/double-click style
      const player = isOpponent ? store.player2 : store.player1;
      if (player?.battleArea[slotIndex]) {
        setInspectedPerm(player.battleArea[slotIndex] ?? null);
      }
    },
    [store, parsedMask, handleAction, digivolvingHandIndex],
  );

  // Right-click inspect: open the stack inspector for ANY permanent (own,
  // opponent, or breeding), at any time. Read-only — submits no action and is
  // independent of pendingSelection / attack state.
  const handleSlotInspect = useCallback(
    (isOpponent: boolean, slotIndex: number) => {
      const player = isOpponent ? store.player2 : store.player1;
      const perm = player?.battleArea[slotIndex];
      if (perm) setInspectedPerm(perm);
    },
    [store],
  );

  const handleBreedingInspect = useCallback(
    (isOpponent: boolean) => {
      const player = isOpponent ? store.player2 : store.player1;
      const perm = player?.breedingArea;
      if (perm) setInspectedPerm(perm);
    },
    [store],
  );

  const handleRevealedClick = useCallback(
    (index: number) => {
      const selIdx = SELECTION.REVEALED_START + index;
      if (parsedMask.validSelections.has(selIdx)) {
        handleAction(selIdx);
      }
    },
    [parsedMask, handleAction],
  );

  // Attack security button
  const handleAttackSecurity = useCallback(() => {
    if (store.selectedAttacker === null) return;
    const actionId =
      ACTION.ATTACK_START + store.selectedAttacker * ATTACK_TARGETS_PER_SLOT + ATTACK_TARGET_SECURITY;
    if (store.actionMask[actionId] === 1) {
      handleAction(actionId);
    }
    store.selectAttacker(null);
  }, [store, handleAction]);

  // Drag-and-drop handlers
  const handleDragStart = useCallback((event: DragStartEvent) => {
    const data = event.active.data.current as DragData | undefined;
    if (data?.type === 'hand-card') {
      setDraggedHandIndex(data.handIndex ?? null);
      if (data.cardId) setDraggedCardId(data.cardId);
    } else if (data?.type === 'breeding-perm' && store.player1?.breedingArea) {
      setDraggedCardId(store.player1.breedingArea.topCardId);
      setDraggedHandIndex(null);
    }
  }, [store.player1]);

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      setDraggedCardId(null);
      setDraggedHandIndex(null);
      setIsOverValid(false);

      const { active, over } = event;
      if (!over) return;

      const dragData = active.data.current as DragData;
      const dropZone = over.data.current as { type: 'empty-field-slot' | 'occupied-field-slot' | 'breeding-slot'; slotIndex: number };

      if (!dragData || !dropZone) return;

      const actionId = getDropAction(dragData, dropZone);
      if (actionId !== null) {
        handleAction(actionId);
      }
    },
    [getDropAction, handleAction],
  );

  const handleDragOver = useCallback(
    (event: { active: { data: { current: unknown } }; over: { data: { current: unknown } } | null }) => {
      if (!event.over) {
        setIsOverValid(false);
        return;
      }
      const dragData = event.active.data.current as DragData;
      const dropZone = event.over.data.current as { type: 'empty-field-slot' | 'occupied-field-slot' | 'breeding-slot'; slotIndex: number };
      if (!dragData || !dropZone) {
        setIsOverValid(false);
        return;
      }
      const actionId = getDropAction(dragData, dropZone);
      setIsOverValid(actionId !== null);
    },
    [getDropAction],
  );

  // Auto-select opponent deck: first deck that differs from player's selection
  const autoSelectOpponentDeck = useCallback(
    (playerDeckId: string) => {
      const other = savedDecks.find((d) => d.id !== playerDeckId);
      setOpponentDeckId(other?.id ?? playerDeckId);
    },
    [savedDecks],
  );

  // No game — show setup screen
  if (!store.gameId) {
    return (
      <div className="flex flex-col items-center justify-center min-h-[calc(100vh-56px)] p-8 gap-6">
        <h1 className="text-2xl font-bold text-[var(--ink-0)]">Start a Game</h1>

        <div className="flex flex-wrap gap-3 items-end justify-center">
          <div>
            <label className="block text-sm text-[var(--ink-1)] mb-1">Your Deck</label>
            <select
              value={selectedDeckId}
              onChange={(e) => {
                setSelectedDeckId(e.target.value);
                if (!opponentDeckId || opponentDeckId === selectedDeckId) {
                  autoSelectOpponentDeck(e.target.value);
                }
              }}
              className="px-3 py-2 bg-[var(--surface-raised)] border border-[var(--line-1)] rounded text-[var(--ink-0)]"
            >
              <option value="">Select a deck...</option>
              {savedDecks.map((d) => (
                <option key={d.id} value={d.id}>{d.name}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm text-[var(--ink-1)] mb-1">Opponent Deck</label>
            <select
              value={opponentDeckId}
              onChange={(e) => setOpponentDeckId(e.target.value)}
              className="px-3 py-2 bg-[var(--surface-raised)] border border-[var(--line-1)] rounded text-[var(--ink-0)]"
            >
              <option value="">Select a deck...</option>
              {savedDecks.map((d) => (
                <option key={d.id} value={d.id}>{d.name}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm text-[var(--ink-1)] mb-1">Agent Type</label>
            <select
              value={agentType}
              onChange={(e) => setAgentType(e.target.value)}
              className="px-3 py-2 bg-[var(--surface-raised)] border border-[var(--line-1)] rounded text-[var(--ink-0)]"
            >
              <option value="greedy">Greedy Agent</option>
              <option value="random">Random Agent</option>
              {IS_DESKTOP && (
                <option value="trained" disabled={localModels.length === 0}>
                  Trained (ONNX){localModels.length === 0 ? ' — none downloaded' : ''}
                </option>
              )}
            </select>
          </div>

          {IS_DESKTOP && agentType === 'trained' && (
            <div>
              <label className="block text-sm text-[var(--ink-1)] mb-1">Model</label>
              <select
                value={selectedModelId}
                onChange={(e) => setSelectedModelId(e.target.value)}
                className="px-3 py-2 bg-[var(--surface-raised)] border border-[var(--line-1)] rounded text-[var(--ink-0)]"
              >
                <option value="">Select a model…</option>
                {localModels.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.name} ({m.model_type.toUpperCase()})
                  </option>
                ))}
              </select>
            </div>
          )}

          <div>
            <label className="block text-sm text-[var(--ink-1)] mb-1">Shuffle Seed</label>
            <input
              value={startSeedInput}
              onChange={(e) => {
                setStartSeedInput(e.target.value);
                setStartSeedError('');
              }}
              placeholder="Random"
              inputMode="numeric"
              className="px-3 py-2 bg-[var(--surface-raised)] border border-[var(--line-1)] rounded text-[var(--ink-0)]"
            />
            {startSeedError && (
              <p className="mt-1 max-w-[220px] text-xs text-[var(--warn)]">{startSeedError}</p>
            )}
          </div>

          <button
            onClick={handleStartGame}
            disabled={
              !selectedDeckId
              || !opponentDeckId
              || starting
              || (agentType === 'trained' && !selectedModelId)
            }
            className="px-6 py-2 bg-[var(--accent)] hover:opacity-90 disabled:opacity-50 text-[var(--accent-ink)] font-medium rounded"
          >
            {starting ? 'Starting...' : 'Start Game'}
          </button>
        </div>
      </div>
    );
  }

  // WebSocket connection gate (PvP / vs-AI-online / spectator). These modes
  // stream state over the socket; `store.gameId` is set immediately, so the
  // board would otherwise render and sit on "Loading game..." forever if the
  // socket never delivers state. Show a connection-status screen until the
  // first state_update hydrates the board, or surface a failure UI (with a
  // retry + a way back) if the socket errors — so a dead socket (e.g. the
  // server returning HTTP 404 on every /ws/games/... upgrade) no longer hangs
  // both players indefinitely. Local games always resolve to 'ready'.
  const connState = connectionScreenState({
    useWebSocket,
    status: ws.status,
    retryCount: ws.retryCount,
    boardHydrated: Boolean(store.player1 && store.player2),
  });
  if (connState !== 'ready') {
    return (
      <ConnectionStatusScreen
        state={connState}
        error={ws.error}
        onRetry={ws.reconnect}
        onLeave={handleLeaveConnection}
      />
    );
  }

  // Compute highlights
  const highlightedOwnSlots = new Set<number>();
  const highlightedEnemySlots = new Set<number>();

  // Highlight attackable own slots
  for (const slot of parsedMask.canAttack.keys()) {
    highlightedOwnSlots.add(slot);
  }

  // Highlight digivolve target slots when in digivolve target mode
  if (digivolvingHandIndex !== null) {
    const targets = parsedMask.canDigivolve.get(digivolvingHandIndex);
    if (targets) {
      for (const slot of targets) highlightedOwnSlots.add(slot);
    }
  }

  // Highlight valid field-permanent selections during selection phases.
  // Without this, prompts like "Choose an opponent Digimon to trash 3
  // digivolution cards from" or DNA Digivolve material picks render the
  // banner but leave the board un-affordanced — the user sees no hint
  // that opponent (or own) slots are clickable. `handleSlotClick` already
  // routes the dispatch; this purely surfaces *which* slots are valid.
  //
  // The engine encodes own- and opponent-field targets in the SAME
  // `OWN_FIELD_START + slot` id range and disambiguates by
  // `pendingSelection.kind`, so we must route by `kind`, not by id range
  // (the old `ENEMY_FIELD_START` branch never matched and left
  // opponent-target prompts un-highlighted). See `utils/selectionTargets.ts`.
  {
    const fieldHighlights = fieldSelectionHighlights(
      store.pendingSelection?.kind,
      parsedMask.validSelections,
    );
    for (const slot of fieldHighlights.own) highlightedOwnSlots.add(slot);
    for (const slot of fieldHighlights.enemy) highlightedEnemySlots.add(slot);

    // `AnyField` (both-battle-area) selections decode the side from each id.
    const anyHighlights = anyFieldSelectionHighlights(
      store.pendingSelection?.kind,
      parsedMask.validSelections,
      store.pendingSelection?.selectingPlayer,
    );
    for (const slot of anyHighlights.own) highlightedOwnSlots.add(slot);
    for (const slot of anyHighlights.enemy) highlightedEnemySlots.add(slot);
  }

  // DNA material selection highlights its own-field candidates by RAW
  // battle_area index (the engine's encoding — see handleSlotClick), which
  // falls outside the 100+ range scanned above. Light up each eligible
  // own-field slot so the player sees which Digimon to combine.
  if (store.currentPhase === GamePhase.SelectMaterial) {
    for (let slot = 0; slot < FIELD_SLOTS; slot++) {
      if (store.actionMask[slot] === 1) highlightedOwnSlots.add(slot);
    }
  }

  // If attacker selected, show valid targets
  const targetedSlots = new Set<number>();
  if (store.selectedAttacker !== null) {
    const targets = parsedMask.canAttack.get(store.selectedAttacker);
    if (targets) {
      for (const t of targets) {
        if (t < FIELD_SLOTS) targetedSlots.add(t);
      }
    }
  }

  // Valid revealed card indices
  const validRevealedIndices = new Set<number>();
  for (const idx of parsedMask.validSelections) {
    if (idx >= SELECTION.REVEALED_START && idx <= SELECTION.REVEALED_END) {
      validRevealedIndices.add(idx - SELECTION.REVEALED_START);
    }
  }

  // Digivolve: highlight hand cards that can digivolve
  const digivolveHandIndices = new Set([
    ...parsedMask.canDigivolve.keys(),
    ...parsedMask.canDigivolveBreeding,
  ]);

  // Merge playable + digivolve highlights for hand
  const highlightedHand = store.currentPhase === GamePhase.Mulligan
    ? new Set<number>()
    : new Set([...parsedMask.canPlayFromHand, ...digivolveHandIndices]);

  // Memory cost preview: look up hovered hand card's play cost
  const previewCost = (() => {
    if (hoveredHandIndex == null || !store.player1) return null;
    const cardInfo = store.player1.handCards?.[hoveredHandIndex];
    if (!cardInfo) return null;
    // Only show preview for playable cards
    if (!parsedMask.canPlayFromHand.has(hoveredHandIndex) && !digivolveHandIndices.has(hoveredHandIndex)) return null;
    return cardInfo.playCost;
  })();

  // Compute valid drop slots while dragging a hand card
  const dragValidDropSlots = new Set<number>();
  let dragCanBreeding = false;
  if (draggedHandIndex !== null) {
    // Empty slots are valid for play
    if (parsedMask.canPlayFromHand.has(draggedHandIndex)) {
      // All empty field slots are valid play targets (marked as 'empty-field-slot')
      // The actual play action just uses handIndex, so all empties work
    }
    // Occupied slots valid for digivolve
    const digiTargets = parsedMask.canDigivolve.get(draggedHandIndex);
    if (digiTargets) {
      for (const slot of digiTargets) dragValidDropSlots.add(slot);
    }
    if (parsedMask.canDigivolveBreeding.has(draggedHandIndex)) {
      dragCanBreeding = true;
    }
  }

  return (
    // `h-full` fills the parent `main` (Layout's flex-1 slot), which in
    // the desktop build is 1080 - navbar; in web mode it's the
    // viewport-equivalent. Previously this used `100vh - 56px` which
    // referenced the actual window viewport (720px in the smallest
    // preset) instead of the fixed canvas (1080px), causing ~360px of
    // dead space below the hand at desktop's minimum preset.
    <div className="h-full flex flex-col">
      {/* Full-width board area */}
      <div className="flex-1 flex flex-col min-w-0 min-h-0">
        <div className="px-3 py-1 flex items-center justify-between">
          <PhaseIndicator
            phase={store.currentPhase}
            turnCount={store.turnCount}
            currentPlayer={store.currentPlayer}
            isGameOver={store.isGameOver}
            winner={store.winner}
            playerLabels={store.playerLabels}
          />
          {store.selectedAttacker !== null && parsedMask.canAttackSecurity.get(store.selectedAttacker) && (
            <button
              onClick={handleAttackSecurity}
              className="px-3 py-1 bg-red-600 hover:bg-red-500 text-white text-sm rounded"
            >
              Attack Security
            </button>
          )}
          {/* Debug undo: rolls back one human action. Only useful in the
              browser-dev workflow (FastAPI replays the saved seed +
              decks - last action). The Tauri build's `gameApi.undoGame`
              returns null so this button silently no-ops if it ever ends
              up on the shipping desktop bundle. Wired here so QA can
              walk back into a state and inspect the action mask /
              pending_selection without restarting the match. */}
          {!store.isGameOver && (
            <button
              data-testid="undo-step-button"
              onClick={handleUndo}
              title="Undo last action (debug)"
              className="px-3 py-1 bg-amber-700 hover:bg-amber-600 text-white text-xs rounded ml-2"
            >
              ← Undo
            </button>
          )}
        </div>
        {store.currentPhase === GamePhase.Mulligan && (
          <div className="px-3 pb-1 text-xs text-amber-300" data-testid="mulligan-banner">
            Opening Mulligan —{' '}
            <span className="font-semibold">
              {store.firstPlayer === 0 ? 'you go first' : 'you go second'}
            </span>
            : choose <span className="font-semibold">Keep Hand</span> or{' '}
            <span className="font-semibold">Mulligan</span>.
          </div>
        )}

        <PromptBar
          currentPhase={store.currentPhase}
          pendingSelection={store.pendingSelection}
          localPlayer={1}
          isGameOver={store.isGameOver}
          agentActing={store.agentPending}
        />
        {pacingStalled && (
          <div className="flex items-center justify-center gap-3 px-4 py-2 bg-red-900/50 border-b border-red-700/60">
            <span className="text-sm text-red-200">
              The bot&apos;s turn could not advance (connection problem?).
            </span>
            <button
              type="button"
              className="px-3 py-0.5 text-sm rounded border border-red-400 text-red-100 hover:bg-red-800/60"
              onClick={resumePacing}
            >
              Continue
            </button>
          </div>
        )}

        <PhaseBanner phase={store.currentPhase} isGameOver={store.isGameOver} />
        <DigivolveBanner />
        <BattleEffect />

        <div className="flex-1 min-h-0 overflow-hidden relative" ref={boardContainerRef}>
          <CardOverlay
            permanent={inspectedPerm}
            onClose={() => setInspectedPerm(null)}
            onInspectCard={setInspectedCardId}
          />
          <CardDetailOverlay
            cardId={inspectedCardId}
            onClose={() => setInspectedCardId(null)}
          />
          <SecurityRevealOverlay />
          <EffectPopup />
          <GameLogDrawer logs={gameLogLines} />
          <AttackArrow
            pendingAttack={store.pendingAttack}
            selectedAttacker={store.selectedAttacker}
            containerRef={boardContainerRef}
          />
          <DndContext
            sensors={sensors}
            onDragStart={handleDragStart}
            onDragEnd={handleDragEnd}
            onDragOver={handleDragOver}
          >
            <GameBoard
              onPlayCard={handlePlayCard}
              onSlotClick={handleSlotClick}
              onSlotInspect={handleSlotInspect}
              onBreedingInspect={handleBreedingInspect}
              onHatch={() => handleAction(ACTION.HATCH)}
              onMove={() => handleAction(ACTION.MOVE)}
              onBreedingClick={
                digivolvingHandIndex !== null && parsedMask.canDigivolveBreeding.has(digivolvingHandIndex)
                  ? () => {
                      handleAction(ACTION.DIGIVOLVE_START + digivolvingHandIndex * DIGIVOLVE_FIELDS_PER_HAND + BREEDING_SLOT);
                      setDigivolvingHandIndex(null);
                    }
                  : parsedMask.canMove
                    ? () => handleAction(ACTION.MOVE)
                    : undefined
              }
              onRevealedClick={handleRevealedClick}
              onOwnTrashClick={() => setTrashViewerPlayer(1)}
              onOpponentTrashClick={() => setTrashViewerPlayer(2)}
              canHatch={parsedMask.canHatch}
              canMove={parsedMask.canMove}
              canDigivolveBreeding={parsedMask.canDigivolveBreeding.size > 0}
              highlightBreeding={
                (digivolvingHandIndex !== null && parsedMask.canDigivolveBreeding.has(digivolvingHandIndex))
                || dragCanBreeding
              }
              playableHandIndices={highlightedHand}
              highlightedOwnSlots={highlightedOwnSlots}
              highlightedEnemySlots={highlightedEnemySlots}
              targetedSlots={targetedSlots}
              validRevealedIndices={validRevealedIndices}
              dragValidDropSlots={draggedHandIndex !== null ? dragValidDropSlots : undefined}
              isDraggingHandCard={draggedHandIndex !== null}
              canPlayDragged={draggedHandIndex !== null && parsedMask.canPlayFromHand.has(draggedHandIndex)}
              previewCost={previewCost}
              onHandCardHoverIndex={setHoveredHandIndex}
              onHandCardInspect={setInspectedCardId}
              actionTraces={store.actionTraces}
              latestTensorSummary={store.latestTensorSummary}
            />
            <DragOverlayCard cardId={draggedCardId} isOverValid={isOverValid} />
          </DndContext>
        </div>

        {/* Action choice dialog (Play / Digivolve / DNA Digivolve) */}
        {actionChoice && (
          <div className="shrink-0 flex items-center justify-center gap-3 py-2 bg-[var(--ib-graphite)] border-t border-[var(--ib-line)]">
            <span className="text-sm text-[var(--ib-bone-d)]">Choose action:</span>
            {actionChoice.canPlay && (
              <button
                onClick={handleActionChoicePlay}
                className="px-4 py-1.5 bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded"
              >
                Play
              </button>
            )}
            {(actionChoice.digivolveTargets.size > 0 ||
              actionChoice.canDigivolveBreeding) && (
              <button
                onClick={handleActionChoiceDigivolve}
                className="px-4 py-1.5 bg-purple-600 hover:bg-purple-500 text-white text-sm font-medium rounded"
              >
                Digivolve
              </button>
            )}
            {actionChoice.canDnaDigivolve && (
              <button
                onClick={handleActionChoiceDna}
                className="px-4 py-1.5 bg-fuchsia-600 hover:bg-fuchsia-500 text-white text-sm font-medium rounded"
              >
                DNA Digivolve
              </button>
            )}
            <button
              onClick={() => setActionChoice(null)}
              className="px-3 py-1.5 bg-gray-600 hover:bg-gray-500 text-white text-sm rounded"
            >
              Cancel
            </button>
          </div>
        )}

        {/* Digivolve target mode indicator */}
        {digivolvingHandIndex !== null && (
          <div className="shrink-0 flex items-center justify-center gap-3 py-2 bg-purple-900/50 border-t border-purple-600">
            <span className="text-sm text-purple-200">Select a digivolve target on the field</span>
            <button
              onClick={() => setDigivolvingHandIndex(null)}
              className="px-3 py-1.5 bg-gray-600 hover:bg-gray-500 text-white text-sm rounded"
            >
              Cancel
            </button>
          </div>
        )}

        {/* Bot speed selector — only meaningful for locally-paced bot
            games; hidden for server-paced WebSocket games. */}
        {!useWebSocket && (
          <div className="shrink-0 flex justify-end px-3 py-1">
            <BotSpeedControl />
          </div>
        )}
        {(store.gameSeed ?? flowSeed) && (
          <div className="shrink-0 flex items-center justify-end gap-2 px-3 py-1 text-xs text-gray-300">
            <span className="uppercase tracking-widest text-gray-500">Seed</span>
            <code className="max-w-[260px] truncate rounded border border-white/10 bg-black/30 px-2 py-1">
              {store.gameSeed ?? flowSeed}
            </code>
            <button
              type="button"
              aria-label="Copy seed"
              onClick={() => void navigator.clipboard?.writeText((store.gameSeed ?? flowSeed) ?? '')}
              className="rounded border border-white/15 px-2 py-1 text-gray-100 hover:bg-white/10"
            >
              Copy
            </button>
          </div>
        )}
        <div className="shrink-0">
          <ActionBar
            phase={store.currentPhase}
            actionMask={store.agentPending ? EMPTY_MASK : store.actionMask}
            onAction={handleAction}
            onSurrender={handleSurrender}
            isGameOver={store.isGameOver}
            canActivateEffect={parsedMask.canActivateEffect}
            decodedActions={store.agentPending ? [] : store.decodedActions}
          />
        </div>
      </div>

      {/* Win/Loss result overlay */}
      <ResultOverlay
        isGameOver={store.isGameOver}
        winner={store.winner}
        localPlayer={1}
        playerLabels={store.playerLabels}
        surrenderedBy={surrenderedBy}
        gameSeed={store.gameSeed ?? flowSeed}
        onReturnToMenu={handleReturnToLauncher}
      />

      {/* Trash viewer modal */}
      <TrashViewer
        isOpen={trashViewerPlayer !== null}
        onClose={() => setTrashViewerPlayer(null)}
        trashIds={
          trashViewerPlayer === 2
            ? (store.player2?.trashIds ?? [])
            : (store.player1?.trashIds ?? [])
        }
        ownerLabel={
          trashViewerPlayer !== null
            ? (store.playerLabels[trashViewerPlayer] ?? `Player ${trashViewerPlayer}`)
            : ''
        }
        onInspect={setInspectedCardId}
      />

      {/* Selection panel modal for hand/security/effect-choice selections */}
      <SelectionPanel
        currentPhase={store.currentPhase}
        pendingSelection={store.pendingSelection}
        actionMask={store.actionMask}
        handIds={store.player1?.handIds ?? []}
        securityCount={store.player1?.securityCount ?? 0}
        opponentSecurityCount={store.player2?.securityCount ?? 0}
        battleArea={store.player1?.battleArea ?? []}
        onAction={handleAction}
        localPlayer={1}
        onInspectCard={setInspectedCardId}
      />

      {/* Interactive trash-selection modal — single + capped multi-select,
          either player's trash (replaces the old SelectionPanel trash path). */}
      <TrashSelectModal
        pendingSelection={store.pendingSelection}
        actionMask={store.actionMask}
        ownTrashIds={store.player1?.trashIds ?? []}
        opponentTrashIds={store.player2?.trashIds ?? []}
        localPlayer={1}
        onAction={handleAction}
        onInspectCard={setInspectedCardId}
      />

      {/* Keyword prompt dialog for optional keyword activations */}
      <KeywordPromptDialog
        currentPhase={store.currentPhase}
        pendingSelection={store.pendingSelection}
        actionMask={store.actionMask}
        onAction={handleAction}
        localPlayer={1}
      />

      {/* Peek toggle — hides the blocking decision overlays so the player can
          read the board before committing to a choice. */}
      {store.pendingSelection?.selectingPlayer === 1 && !store.isGameOver && <PeekButton />}
    </div>
  );
}
