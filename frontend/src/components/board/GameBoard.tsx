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
  onRevealedClick?: (index: number) => void;
  canHatch?: boolean;
  canMove?: boolean;
  playableHandIndices?: Set<number>;
  highlightedOwnSlots?: Set<number>;
  highlightedEnemySlots?: Set<number>;
  targetedSlots?: Set<number>;
  validRevealedIndices?: Set<number>;
}

export function GameBoard({
  onPlayCard,
  onSlotClick,
  onHatch,
  onMove,
  onRevealedClick,
  canHatch = false,
  canMove = false,
  playableHandIndices,
  highlightedOwnSlots,
  highlightedEnemySlots,
  targetedSlots,
  validRevealedIndices,
}: GameBoardProps) {
  const {
    player1,
    player2,
    memoryGauge,
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
    <div className="flex flex-col gap-1 h-full">
      {/* Opponent half */}
      <div className="flex-[2]">
        <PlayerHalf
          player={player2}
          isOpponent
          highlightedSlots={highlightedEnemySlots}
          targetedSlots={targetedSlots}
          onSlotClick={(i) => onSlotClick?.(true, i)}
        />
      </div>

      {/* Opponent hand */}
      <HandZone cardIds={player2.handIds} isOpponent />

      {/* Memory gauge + revealed cards */}
      <div className="flex items-center justify-center gap-4 py-1 border-y border-gray-700/50">
        <MemoryGauge value={memoryGauge} />
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
          onSlotClick={(i) => onSlotClick?.(false, i)}
          onHatch={onHatch}
          onMove={onMove}
        />
      </div>

      {/* Player hand */}
      <HandZone
        cardIds={player1.handIds}
        isOpponent={false}
        highlightedIndices={playableHandIndices}
        onCardClick={onPlayCard}
      />
    </div>
  );
}
