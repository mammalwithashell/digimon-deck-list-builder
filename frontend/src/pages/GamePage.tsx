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
import { useGameActions } from '@/hooks/useGameActions';
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
import { GameLogDrawer } from '@/components/game/GameLogDrawer';
import { SecurityRevealOverlay } from '@/components/board/SecurityRevealOverlay';
import { EffectPopup } from '@/components/game/EffectPopup';
import { KeywordPromptDialog } from '@/components/game/KeywordPromptDialog';
import { DragOverlayCard } from '@/components/game/DragOverlayCard';
import { useWebSocketGame, type UseWebSocketGameOptions } from '@/hooks/useWebSocketGame';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import { useModelStore } from '@/stores/modelStore';
import { modelReferenceForEntry, isEntryAvailable } from '@/api/modelsApi';
import * as gameApi from '@/api/gameApi';
import * as deckApiMod from '@/api/deckApi';
import {
  ACTION,
  ATTACK_TARGETS_PER_SLOT,
  ATTACK_TARGET_SECURITY,
  BREEDING_SLOT,
  DIGIVOLVE_FIELDS_PER_HAND,
  FIELD_SLOTS,
  SELECTION,
} from '@/utils/constants';
import { GamePhase, type PermanentInfo } from '@/types/game';

export function GamePage() {
  const { id: urlGameId } = useParams<{ id: string }>();
  const [searchParams] = useSearchParams();
  const isPvpMode = searchParams.get('mode') === 'pvp';
  const isSpectator = searchParams.get('role') === 'spectator';


  const store = useGameStore();
  const { sendAction: httpSendAction } = useGameActions();
  useEffectHighlight();
  const parsedMask = useActionMask(store.actionMask);

  // WebSocket connection (only active in PvP/spectator mode)
  const wsOptions = useMemo<UseWebSocketGameOptions | null>(() => {
    if (!urlGameId || (!isPvpMode && !isSpectator)) return null;
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
            [payload.your_player_id === 1 ? 2 : 1]: 'Opponent',
          });
        }
      },
      onGameOver: () => {},
      onError: (msg) => console.error('WebSocket error:', msg),
    };
  }, [urlGameId, isPvpMode, isSpectator]); // eslint-disable-line react-hooks/exhaustive-deps

  const ws = useWebSocketGame(wsOptions);

  // Use WebSocket sendAction for PvP, HTTP for local games
  const sendAction = isPvpMode || isSpectator ? ws.sendAction : httpSendAction;
  const { savedDecks, setSavedDecks } = useDeckBuilderStore();
  const { getDropAction } = useDropZone(parsedMask);

  const [selectedDeckId, setSelectedDeckId] = useState<string>('');
  const [opponentDeckId, setOpponentDeckId] = useState<string>('');
  const [agentType, setAgentType] = useState<string>('greedy');
  const [agentModelSlug, setAgentModelSlug] = useState<string>('');
  const [starting, setStarting] = useState(false);
  const modelStore = useModelStore();
  useEffect(() => {
    // Lazy-load the model catalog on first render so the picker has
    // something to show; refresh()'s own internal cache keeps this cheap.
    if (modelStore.manifest === null) void modelStore.refresh();
  }, [modelStore]);
  const availableModels = modelStore.listModels().filter(isEntryAvailable);

  // Hydrate store from URL param when navigating to /game/:id
  useEffect(() => {
    if (urlGameId && !store.gameId) {
      if (isPvpMode || isSpectator) {
        // PvP/spectator mode: WebSocket hook will send initial state
        store.setGameId(urlGameId);
        store.clearLogs();
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
          } catch (err) {
            console.error('Failed to load game:', err);
          }
        })();
      }
    }
  }, [urlGameId]); // eslint-disable-line react-hooks/exhaustive-deps
  const [inspectedPerm, setInspectedPerm] = useState<PermanentInfo | null>(null);
  const [draggedCardId, setDraggedCardId] = useState<string | null>(null);
  const [draggedHandIndex, setDraggedHandIndex] = useState<number | null>(null);
  const [isOverValid, setIsOverValid] = useState(false);
  // Action choice dialog: shown when hand card can both play and digivolve
  const [actionChoice, setActionChoice] = useState<{
    handIndex: number;
    canPlay: boolean;
    digivolveTargets: Set<number>; // field slot indices
    canDigivolveBreeding: boolean;
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
    deckApiMod.listDecks().then(setSavedDecks).catch(() => {});
  });

  // DnD sensors: pointer with 8px activation distance, touch with 150ms delay
  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 8 } }),
    useSensor(TouchSensor, { activationConstraint: { delay: 150, tolerance: 5 } }),
  );

  const handleStartGame = async () => {
    if (!selectedDeckId || !opponentDeckId) return;
    setStarting(true);
    try {
      const [deck, oppDeck] = await Promise.all([
        deckApiMod.getDeck(selectedDeckId),
        deckApiMod.getDeck(opponentDeckId),
      ]);

      const result = await gameApi.createGame({
        deck1: [...deck.egg_deck, ...deck.main_deck],
        deck2: [...oppDeck.egg_deck, ...oppDeck.main_deck],
        player1_type: 'human',
        player2_type: 'agent',
        player2_policy: agentModelSlug ? 'trained' : agentType,
        player2_model: agentModelSlug || undefined,
      });

      // Set game state before gameId so the board has player data when it first renders
      store.setGameState(result.state);
      store.setActionMask(result.action_mask);
      if (result.player_labels) store.setPlayerLabels(result.player_labels);
      store.clearLogs();
      store.clearEvents();
      store.setGameId(result.game_id);

      // Step once to handle initial agent turn if agent goes first
      const stepResult = await gameApi.stepGame(result.game_id);
      store.setGameState(stepResult.state);
      store.setActionMask(stepResult.action_mask);
      if (stepResult.logs) store.appendLogs(stepResult.logs);
      if (stepResult.events) store.appendEvents(stepResult.events);
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

  const handleSurrender = useCallback(async () => {
    if (!store.gameId || store.isGameOver) return;
    try {
      const res = await gameApi.surrenderGame(store.gameId, 1);
      store.setGameState(res.state);
      store.setActionMask(res.action_mask);
      if (res.logs) store.appendLogs(res.logs);
      if (res.events) store.appendEvents(res.events);
      setSurrenderedBy(1);
    } catch {
      // Ignore errors (e.g. game already over)
    }
  }, [store]);

  const handlePlayCard = useCallback(
    (handIndex: number) => {
      if (store.currentPhase === GamePhase.Mulligan) {
        return;
      }
      const canPlay = parsedMask.canPlayFromHand.has(handIndex);
      const fieldTargets = parsedMask.canDigivolve.get(handIndex);
      const canBreeding = parsedMask.canDigivolveBreeding.has(handIndex);
      const canDigi = (fieldTargets && fieldTargets.size > 0) || canBreeding;

      if (canPlay && canDigi) {
        // Show choice dialog
        setActionChoice({
          handIndex,
          canPlay: true,
          digivolveTargets: fieldTargets ?? new Set(),
          canDigivolveBreeding: canBreeding,
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

      // During selection phases, map to selection actions
      if (phase >= GamePhase.SelectTarget && phase <= GamePhase.SelectSecurity) {
        const selIdx = isOpponent
          ? SELECTION.ENEMY_FIELD_START + slotIndex
          : SELECTION.OWN_FIELD_START + slotIndex;
        if (parsedMask.validSelections.has(selIdx)) {
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
              value={agentModelSlug ? 'trained' : agentType}
              onChange={(e) => {
                if (e.target.value !== 'trained') {
                  setAgentModelSlug('');
                  setAgentType(e.target.value);
                }
              }}
              className="px-3 py-2 bg-gray-700 border border-gray-600 rounded text-gray-200"
            >
              <option value="greedy">Greedy Agent</option>
              <option value="random">Random Agent</option>
              {availableModels.length > 0 && (
                <option value="trained" disabled>
                  ── Trained model: ──
                </option>
              )}
            </select>
          </div>

          {availableModels.length > 0 && (
            <div>
              <label className="block text-sm text-gray-400 mb-1">Trained Model</label>
              <select
                value={agentModelSlug}
                onChange={(e) => setAgentModelSlug(e.target.value)}
                className="px-3 py-2 bg-gray-700 border border-gray-600 rounded text-gray-200"
              >
                <option value="">(none — use agent above)</option>
                {availableModels.map((m) => {
                  const ref = modelReferenceForEntry(m);
                  return ref ? (
                    <option key={m.slug} value={ref}>
                      {m.name} ({m.arch})
                    </option>
                  ) : null;
                })}
              </select>
              <div className="text-xs text-gray-500 mt-1">
                Manage downloads in{' '}
                <a href="/settings/models" className="underline">
                  Settings → Models
                </a>
              </div>
            </div>
          )}

          <button
            onClick={handleStartGame}
            disabled={!selectedDeckId || !opponentDeckId || starting}
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
    <div className="h-[calc(100vh-56px)] flex flex-col">
      {/* Full-width board area */}
      <div className="flex-1 flex flex-col min-w-0">
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
            />
            <DragOverlayCard cardId={draggedCardId} isOverValid={isOverValid} />
          </DndContext>
        </div>

        {/* Action choice dialog (Play vs Digivolve) */}
        {actionChoice && (
          <div className="flex items-center justify-center gap-3 py-2 bg-gray-800 border-t border-gray-600">
            <span className="text-sm text-gray-300">Choose action:</span>
            <button
              onClick={handleActionChoicePlay}
              className="px-4 py-1.5 bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium rounded"
            >
              Play
            </button>
            <button
              onClick={handleActionChoiceDigivolve}
              className="px-4 py-1.5 bg-purple-600 hover:bg-purple-500 text-white text-sm font-medium rounded"
            >
              Digivolve
            </button>
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
        onAction={handleAction}
        localPlayer={1}
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
