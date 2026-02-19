import { useState, useCallback } from 'react';
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
import { useDropZone, type DragData } from '@/hooks/useDropZone';
import { GameBoard } from '@/components/board/GameBoard';
import { ActionBar } from '@/components/game/ActionBar';
import { PhaseIndicator } from '@/components/game/PhaseIndicator';
import { GameLog } from '@/components/game/GameLog';
import { StackInspector } from '@/components/game/StackInspector';
import { DragOverlayCard } from '@/components/game/DragOverlayCard';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import * as gameApi from '@/api/gameApi';
import * as deckApiMod from '@/api/deckApi';
import {
  ACTION,
  ATTACK_TARGETS_PER_SLOT,
  SELECTION,
} from '@/utils/constants';
import { GamePhase, type PermanentInfo } from '@/types/game';

export function GamePage() {
  const store = useGameStore();
  const { sendAction } = useGameActions();
  const parsedMask = useActionMask(store.actionMask);
  const { savedDecks, setSavedDecks } = useDeckBuilderStore();
  const { getDropAction } = useDropZone(parsedMask);

  const [selectedDeckId, setSelectedDeckId] = useState<string>('');
  const [agentType, setAgentType] = useState<string>('greedy');
  const [starting, setStarting] = useState(false);
  const [inspectedPerm, setInspectedPerm] = useState<PermanentInfo | null>(null);
  const [draggedCardId, setDraggedCardId] = useState<string | null>(null);
  const [isOverValid, setIsOverValid] = useState(false);

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
    if (!selectedDeckId) return;
    setStarting(true);
    try {
      const deck = await deckApiMod.getDeck(selectedDeckId);
      const deckIds = deck.main_deck;
      const eggIds = deck.egg_deck;

      const result = await gameApi.createGame({
        deck1: [...eggIds, ...deckIds],
        deck2: [...eggIds, ...deckIds], // Agent uses same deck for now
        player1_type: 'human',
        player2_type: 'agent',
      });

      store.setGameId(result.game_id);
      store.setGameState(result.state);
      store.setActionMask(result.action_mask);
      if (result.player_labels) store.setPlayerLabels(result.player_labels);
      store.clearLogs();

      // Step once to handle initial agent turn if agent goes first
      const stepResult = await gameApi.stepGame(result.game_id);
      store.setGameState(stepResult.state);
      store.setActionMask(stepResult.action_mask);
      if (stepResult.logs) store.appendLogs(stepResult.logs);
    } catch (err) {
      console.error('Failed to create game:', err);
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

  const handlePlayCard = useCallback(
    (handIndex: number) => {
      if (parsedMask.canPlayFromHand.has(handIndex)) {
        handleAction(handIndex);
      }
    },
    [parsedMask, handleAction],
  );

  const handleSlotClick = useCallback(
    (isOpponent: boolean, slotIndex: number) => {
      const phase = store.currentPhase;

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
        const blockAction = 100 + slotIndex;
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
    [store, parsedMask, handleAction],
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
      ACTION.ATTACK_START + store.selectedAttacker * ATTACK_TARGETS_PER_SLOT + 12;
    if (store.actionMask[actionId] === 1) {
      handleAction(actionId);
    }
    store.selectAttacker(null);
  }, [store, handleAction]);

  // Drag-and-drop handlers
  const handleDragStart = useCallback((event: DragStartEvent) => {
    const data = event.active.data.current as DragData | undefined;
    if (data?.cardId) {
      setDraggedCardId(data.cardId);
    } else if (data?.type === 'breeding-perm' && store.player1?.breedingArea) {
      setDraggedCardId(store.player1.breedingArea.topCardId);
    }
  }, [store.player1]);

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      setDraggedCardId(null);
      setIsOverValid(false);

      const { active, over } = event;
      if (!over) return;

      const dragData = active.data.current as DragData;
      const dropZone = over.data.current as { type: 'empty-field-slot' | 'occupied-field-slot'; slotIndex: number };

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
      const dropZone = event.over.data.current as { type: 'empty-field-slot' | 'occupied-field-slot'; slotIndex: number };
      if (!dragData || !dropZone) {
        setIsOverValid(false);
        return;
      }
      const actionId = getDropAction(dragData, dropZone);
      setIsOverValid(actionId !== null);
    },
    [getDropAction],
  );

  // No game — show setup screen
  if (!store.gameId) {
    return (
      <div className="flex flex-col items-center justify-center min-h-[calc(100vh-56px)] p-8 gap-4">
        <h1 className="text-2xl font-bold text-gray-100">Start a Game</h1>

        <div className="flex gap-3 items-end">
          <div>
            <label className="block text-sm text-gray-400 mb-1">Your Deck</label>
            <select
              value={selectedDeckId}
              onChange={(e) => setSelectedDeckId(e.target.value)}
              className="px-3 py-2 bg-gray-700 border border-gray-600 rounded text-gray-200"
            >
              <option value="">Select a deck...</option>
              {savedDecks.map((d) => (
                <option key={d.id} value={d.id}>{d.name}</option>
              ))}
            </select>
          </div>

          <div>
            <label className="block text-sm text-gray-400 mb-1">Opponent</label>
            <select
              value={agentType}
              onChange={(e) => setAgentType(e.target.value)}
              className="px-3 py-2 bg-gray-700 border border-gray-600 rounded text-gray-200"
            >
              <option value="greedy">Greedy Agent</option>
              <option value="random">Random Agent</option>
            </select>
          </div>

          <button
            onClick={handleStartGame}
            disabled={!selectedDeckId || starting}
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

  // If attacker selected, show valid targets
  const targetedSlots = new Set<number>();
  if (store.selectedAttacker !== null) {
    const targets = parsedMask.canAttack.get(store.selectedAttacker);
    if (targets) {
      for (const t of targets) {
        if (t < 12) targetedSlots.add(t);
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
  const digivolveHandIndices = new Set(parsedMask.canDigivolve.keys());

  // Merge playable + digivolve highlights for hand
  const highlightedHand = new Set([...parsedMask.canPlayFromHand, ...digivolveHandIndices]);

  return (
    <div className="h-[calc(100vh-56px)] flex">
      {/* Main board area */}
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

        <div className="flex-1 overflow-hidden">
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
              onRevealedClick={handleRevealedClick}
              canHatch={parsedMask.canHatch}
              canMove={parsedMask.canMove}
              playableHandIndices={highlightedHand}
              highlightedOwnSlots={highlightedOwnSlots}
              highlightedEnemySlots={highlightedEnemySlots}
              targetedSlots={targetedSlots}
              validRevealedIndices={validRevealedIndices}
            />
            <DragOverlayCard cardId={draggedCardId} isOverValid={isOverValid} />
          </DndContext>
        </div>

        <ActionBar
          phase={store.currentPhase}
          actionMask={store.actionMask}
          onAction={handleAction}
          isGameOver={store.isGameOver}
        />
      </div>

      {/* Right sidebar */}
      <div className="w-[240px] flex flex-col border-l border-gray-700">
        {inspectedPerm ? (
          <StackInspector
            permanent={inspectedPerm}
            onClose={() => setInspectedPerm(null)}
          />
        ) : (
          <GameLog logs={store.logs} />
        )}
      </div>
    </div>
  );
}
