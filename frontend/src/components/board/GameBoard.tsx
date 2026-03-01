import { useGameStore } from '@/stores/gameStore';
import { PlayerHalf } from './PlayerHalf';
import { HandZone } from './HandZone';
import { MemoryGauge } from './MemoryGauge';
import { RevealedCardsZone } from './RevealedCardsZone';
import { SELECTION } from '@/utils/constants';

interface GameBoardProps {
  onPlayCard?: (handIndex: number) => void;
  onSlotClick?: (isOpponent: boolean, slotIndex: number) => void;
  onHatch?: () => void;
  onMove?: () => void;
  onBreedingClick?: () => void;
  onRevealedClick?: (index: number) => void;
  canHatch?: boolean;
  canMove?: boolean;
  canDigivolveBreeding?: boolean;
  highlightBreeding?: boolean;
  playableHandIndices?: Set<number>;
  highlightedOwnSlots?: Set<number>;
  highlightedEnemySlots?: Set<number>;
  targetedSlots?: Set<number>;
  validRevealedIndices?: Set<number>;
  /** Field slot indices where dragged hand card can digivolve */
  dragValidDropSlots?: Set<number>;
  /** Whether a hand card is currently being dragged */
  isDraggingHandCard?: boolean;
  /** Whether the dragged hand card can be played (to empty slots) */
  canPlayDragged?: boolean;
  onOwnTrashClick?: () => void;
  onOpponentTrashClick?: () => void;
}

export function GameBoard({
  onPlayCard,
  onSlotClick,
  onHatch,
  onMove,
  onBreedingClick,
  onRevealedClick,
  canHatch = false,
  canMove = false,
  canDigivolveBreeding = false,
  highlightBreeding = false,
  playableHandIndices,
  highlightedOwnSlots,
  highlightedEnemySlots,
  targetedSlots,
  validRevealedIndices,
  dragValidDropSlots,
  isDraggingHandCard = false,
  canPlayDragged = false,
  onOwnTrashClick,
  onOpponentTrashClick,
}: GameBoardProps) {
  const {
    player1,
    player2,
    memoryGauge,
    currentPhase,
    revealedCards,
    pendingSelection,
  } = useGameStore();

  if (!player1 || !player2) return null;

  // During selection phases, highlight valid targets
  const selectionHighlights = new Set<number>();
  if (pendingSelection) {
    for (const idx of pendingSelection.validIndices) {
      if (idx >= SELECTION.OWN_FIELD_START && idx <= SELECTION.OWN_FIELD_END) {
        selectionHighlights.add(idx - SELECTION.OWN_FIELD_START);
      }
    }
  }

  const ownSlots = new Set([...(highlightedOwnSlots ?? []), ...selectionHighlights]);

  return (
    <div data-testid="game-board" className="flex flex-col gap-1 h-full">
      {/* Opponent hand (top edge) */}
      <HandZone cardIds={player2.handIds} isOpponent />

      {/* Opponent half */}
      <div className="flex-[2]">
        <PlayerHalf
          player={player2}
          isOpponent
          highlightedSlots={highlightedEnemySlots}
          onTrashClick={onOpponentTrashClick}
          targetedSlots={targetedSlots}
          onSlotClick={(i) => onSlotClick?.(true, i)}
        />
      </div>

      {/* Memory gauge */}
      <div className="flex items-center justify-center gap-4 py-1 border-y border-gray-700/50">
        <MemoryGauge value={memoryGauge} localPlayer={1} currentPhase={currentPhase} />
      </div>

      {/* Revealed cards */}
      {revealedCards.length > 0 && (
        <RevealedCardsZone
          cards={revealedCards}
          validIndices={validRevealedIndices}
          onCardClick={onRevealedClick}
        />
      )}

      {/* Player half */}
      <div className="flex-[2]">
        <PlayerHalf
          player={player1}
          isOpponent={false}
          highlightedSlots={ownSlots}
          canHatch={canHatch}
          canMove={canMove}
          canDigivolveBreeding={canDigivolveBreeding}
          highlightBreeding={highlightBreeding}
          onSlotClick={(i) => onSlotClick?.(false, i)}
          onHatch={onHatch}
          onMove={onMove}
          onBreedingClick={onBreedingClick}
          onTrashClick={onOwnTrashClick}
          dragValidDropSlots={dragValidDropSlots}
          isDraggingHandCard={isDraggingHandCard}
          canPlayDragged={canPlayDragged}
        />
      </div>

      {/* Player hand (bottom edge) */}
      <HandZone
        cardIds={player1.handIds}
        isOpponent={false}
        highlightedIndices={playableHandIndices}
        onCardClick={onPlayCard}
      />
    </div>
  );
}
