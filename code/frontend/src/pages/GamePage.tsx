import { useState, useCallback, useRef, useEffect, useMemo } from 'react';
import { useParams, useSearchParams } from 'react-router-dom';
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
import { DragOverlayCard } from '@/components/game/DragOverlayCard';
import { useWebSocketGame, type UseWebSocketGameOptions } from '@/hooks/useWebSocketGame';
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
  fieldSelectionActionId,
  fieldSelectionHighlights,
  isFieldSelectionKind,
} from '@/utils/selectionTargets';
import { GamePhase, type ActionTrace, type PermanentInfo } from '@/types/game';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';
type ActionTraceResponse = { action_traces?: ActionTrace[] };
// Desktop builds have no server-side Deck rows, so deck list/load must go
// through the Tauri-backed deckStore. parseDeck / validateDeckRaw already
// branch internally, so those stay on deckApiMod.
const decks = IS_DESKTOP ? deckStore : deckApiMod;

export function GamePage() {
  const { id: urlGameId } = useParams<{ id: string }>();
  const [searchParams] = useSearchParams();
  const mode = searchParams.get('mode');
  const isPvpMode = mode === 'pvp';
  const isVsAiOnline = mode === 'vsai';
  const isSpectator = searchParams.get('role') === 'spectator';
  // WebSocket is used for both PvP and vs-AI-online: the server streams
  // state and runs the AI turn internally for the 'vsai' mode.
  const useWebSocket = isPvpMode || isVsAiOnline || isSpectator;


  const store = useGameStore();
  const { opponentMode } = usePlayFlowStore();
  useEffectHighlight();
  const parsedMask = useActionMask(store.actionMask);
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
        if (payload.logs) store.appendLogs(payload.logs);
        if (payload.events) store.appendEvents(payload.events);
        if (payload.your_player_id != null) {
          store.setPlayerLabels({
            [payload.your_player_id]: 'You',
            [payload.your_player_id === 1 ? 2 : 1]: isVsAiOnline ? 'AI' : 'Opponent',
          });
        }
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

  // Use WebSocket sendAction for PvP / vs-AI-online / spectator; HTTP for local games.
  const sendAction = useCallback(
    async (actionId: number) => {
      if (useWebSocket) {
        ws.sendAction(actionId);
        return;
      }
      if (!store.gameId || actionPendingRef.current) return;
      actionPendingRef.current = true;

      try {
        const actionResult = await gameApi.sendAction(store.gameId, actionId);
        store.setGameState(actionResult.state);
        store.setActionMask(actionResult.action_mask);
        if (actionResult.logs) store.appendLogs(actionResult.logs);
        if (actionResult.events) store.appendEvents(actionResult.events);
        appendResponseActionTraces(actionResult);

        // If the human is still mid-decision — a multi-stage selection like
        // DNA digivolve's two material picks leaves a pending_selection
        // owned by the local player — do NOT run the agent-autoplay step.
        // Stepping here is a no-op server-side but keeps `actionPendingRef`
        // set across the await, which would drop the player's very next
        // click (e.g. the second DNA material). Let the player continue.
        const humanStillDeciding =
          actionResult.state.pendingSelection?.selectingPlayer === 1;

        if (!actionResult.is_game_over && !humanStillDeciding) {
          const stepResult = await gameApi.stepGame(store.gameId);
          store.setGameState(stepResult.state);
          store.setActionMask(stepResult.action_mask);
          if (stepResult.logs) store.appendLogs(stepResult.logs);
          if (stepResult.events) store.appendEvents(stepResult.events);
          appendResponseActionTraces(stepResult);
        }
      } finally {
        actionPendingRef.current = false;
      }
    },
    [appendResponseActionTraces, store, useWebSocket, ws],
  );
  const { savedDecks, setSavedDecks } = useDeckBuilderStore();
  const { getDropAction } = useDropZone(parsedMask);

  const [selectedDeckId, setSelectedDeckId] = useState<string>('');
  const [opponentDeckId, setOpponentDeckId] = useState<string>('');
  const [agentType, setAgentType] = useState<string>('greedy');
  const [starting, setStarting] = useState(false);
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

  // Hydrate store from URL param when navigating to /game/:id
  useEffect(() => {
    if (urlGameId && !store.gameId) {
      if (useWebSocket) {
        // PvP / vs-AI-online / spectator: WebSocket hook will send initial state
        store.setGameId(urlGameId);
        store.clearLogs();
        store.clearActionTraces();
      } else {
        // Local mode: fetch state via HTTP
        (async () => {
          try {
            const [state, maskData] = await Promise.all([
              gameApi.getState(urlGameId),
              gameApi.getMask(urlGameId),
            ]);
            store.setGameId(urlGameId);
            store.setGameState(state);
            store.setActionMask(maskData);
            store.clearLogs();
            store.clearActionTraces();
            store.setPlayerLabels({
              1: 'YOU',
              2: opponentMode === 'bot' ? 'GREEDY BOT' : 'OPPONENT',
            });
          } catch (err) {
            console.error('Failed to load game:', err);
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
      });

      // Set game state before gameId so the board has player data when it first renders
      store.setGameState(result.state);
      store.setActionMask(result.action_mask);
      if (result.player_labels) {
        store.setPlayerLabels(result.player_labels);
      } else {
        store.setPlayerLabels({
          1: 'YOU',
          2: agentType === 'greedy' ? 'GREEDY BOT' : 'OPPONENT',
        });
      }
      store.clearLogs();
      store.clearEvents();
      store.clearActionTraces();
      store.setGameId(result.game_id);

      // Step once to handle initial agent turn if agent goes first
      const stepResult = await gameApi.stepGame(result.game_id);
      store.setGameState(stepResult.state);
      store.setActionMask(stepResult.action_mask);
      if (stepResult.logs) store.appendLogs(stepResult.logs);
      if (stepResult.events) store.appendEvents(stepResult.events);
      appendResponseActionTraces(stepResult);
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
      store.clearLogs();
      store.clearEvents();
      store.clearActionTraces();
      if (res.logs) store.appendLogs(res.logs);
      if (res.events) store.appendEvents(res.events);
      // Clear any in-flight UI selection state so we land on the
      // replayed snapshot with a clean slate.
      setActionChoice(null);
      setDigivolvingHandIndex(null);
      setSurrenderedBy(null);
    } catch (err) {
      console.error('Undo failed:', err);
    }
  }, [store]);

  const handleSurrender = useCallback(async () => {
    if (!store.gameId || store.isGameOver) return;
    try {
      const res = await gameApi.surrenderGame(store.gameId, 1);
      store.setGameState(res.state);
      store.setActionMask(res.action_mask);
      if (res.logs) store.appendLogs(res.logs);
      if (res.events) store.appendEvents(res.events);
      appendResponseActionTraces(res as typeof res & ActionTraceResponse);
      setSurrenderedBy(1);
    } catch {
      // Ignore errors (e.g. game already over)
    }
  }, [appendResponseActionTraces, store]);

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
        <h1 className="text-2xl font-bold text-gray-100">Start a Game</h1>

        <div className="flex flex-wrap gap-3 items-end justify-center">
          <div>
            <label className="block text-sm text-gray-400 mb-1">Your Deck</label>
            <select
              value={selectedDeckId}
              onChange={(e) => {
                setSelectedDeckId(e.target.value);
                if (!opponentDeckId || opponentDeckId === selectedDeckId) {
                  autoSelectOpponentDeck(e.target.value);
                }
              }}
              className="px-3 py-2 bg-gray-700 border border-gray-600 rounded text-gray-200"
            >
              <option value="">Select a deck...</option>
              {savedDecks.map((d) => (
                <option key={d.id} value={d.id}>{d.name}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm text-gray-400 mb-1">Opponent Deck</label>
            <select
              value={opponentDeckId}
              onChange={(e) => setOpponentDeckId(e.target.value)}
              className="px-3 py-2 bg-gray-700 border border-gray-600 rounded text-gray-200"
            >
              <option value="">Select a deck...</option>
              {savedDecks.map((d) => (
                <option key={d.id} value={d.id}>{d.name}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm text-gray-400 mb-1">Agent Type</label>
            <select
              value={agentType}
              onChange={(e) => setAgentType(e.target.value)}
              className="px-3 py-2 bg-gray-700 border border-gray-600 rounded text-gray-200"
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
              <label className="block text-sm text-gray-400 mb-1">Model</label>
              <select
                value={selectedModelId}
                onChange={(e) => setSelectedModelId(e.target.value)}
                className="px-3 py-2 bg-gray-700 border border-gray-600 rounded text-gray-200"
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

          <button
            onClick={handleStartGame}
            disabled={
              !selectedDeckId
              || !opponentDeckId
              || starting
              || (agentType === 'trained' && !selectedModelId)
            }
            className="px-6 py-2 bg-blue-600 hover:bg-blue-500 disabled:bg-blue-800 disabled:opacity-50 text-white font-medium rounded"
          >
            {starting ? 'Starting...' : 'Start Game'}
          </button>
        </div>
      </div>
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
            Opening Mulligan: choose <span className="font-semibold">Keep Hand</span> or <span className="font-semibold">Mulligan</span>.
          </div>
        )}

        <PromptBar
          currentPhase={store.currentPhase}
          pendingSelection={store.pendingSelection}
          localPlayer={1}
          isGameOver={store.isGameOver}
        />

        <PhaseBanner phase={store.currentPhase} isGameOver={store.isGameOver} />
        <DigivolveBanner />
        <BattleEffect />

        <div className="flex-1 overflow-hidden relative" ref={boardContainerRef}>
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
          <GameLogDrawer logs={store.logs} />
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
          <div className="flex items-center justify-center gap-3 py-2 bg-gray-800 border-t border-gray-600">
            <span className="text-sm text-gray-300">Choose action:</span>
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
          <div className="flex items-center justify-center gap-3 py-2 bg-purple-900/50 border-t border-purple-600">
            <span className="text-sm text-purple-200">Select a digivolve target on the field</span>
            <button
              onClick={() => setDigivolvingHandIndex(null)}
              className="px-3 py-1.5 bg-gray-600 hover:bg-gray-500 text-white text-sm rounded"
            >
              Cancel
            </button>
          </div>
        )}

        <ActionBar
          phase={store.currentPhase}
          actionMask={store.actionMask}
          onAction={handleAction}
          onSurrender={handleSurrender}
          isGameOver={store.isGameOver}
          canActivateEffect={parsedMask.canActivateEffect}
        />
      </div>

      {/* Win/Loss result overlay */}
      <ResultOverlay
        isGameOver={store.isGameOver}
        winner={store.winner}
        localPlayer={1}
        playerLabels={store.playerLabels}
        surrenderedBy={surrenderedBy}
        onReturnToMenu={() => store.reset()}
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
      />

      {/* Selection panel modal for hand/trash/security/effect-choice selections */}
      <SelectionPanel
        currentPhase={store.currentPhase}
        pendingSelection={store.pendingSelection}
        actionMask={store.actionMask}
        handIds={store.player1?.handIds ?? []}
        trashIds={store.player1?.trashIds ?? []}
        securityIds={store.player1?.securityIds ?? []}
        battleArea={store.player1?.battleArea ?? []}
        onAction={handleAction}
        localPlayer={1}
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
    </div>
  );
}
