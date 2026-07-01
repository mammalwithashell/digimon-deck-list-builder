import { BattleArea } from './BattleArea';
import { EggDeck } from './EggDeck';
import { BreedingArea } from './BreedingArea';
import { DeckPile } from './DeckPile';
import { SecurityStack } from './SecurityStack';
import { TrashPile } from './TrashPile';
import type { PlayerState } from '@/types/game';

interface PlayerHalfProps {
  player: PlayerState;
  isOpponent: boolean;
  highlightedSlots?: Set<number>;
  /** Slots already picked during a "choose N" selection (marked as selected). */
  selectedSlots?: Set<number>;
  targetedSlots?: Set<number>;
  canHatch?: boolean;
  canMove?: boolean;
  canDigivolveBreeding?: boolean;
  highlightBreeding?: boolean;
  onSlotClick?: (slotIndex: number) => void;
  onSlotHover?: (slotIndex: number | null) => void;
  onSlotInspect?: (slotIndex: number) => void;
  onBreedingInspect?: () => void;
  onHatch?: () => void;
  onMove?: () => void;
  onBreedingClick?: () => void;
  onTrashClick?: () => void;
  dragValidDropSlots?: Set<number>;
  isDraggingHandCard?: boolean;
  canPlayDragged?: boolean;
}

export function PlayerHalf({
  player,
  isOpponent,
  highlightedSlots,
  selectedSlots,
  targetedSlots,
  canHatch = false,
  canMove = false,
  canDigivolveBreeding = false,
  highlightBreeding = false,
  onSlotClick,
  onSlotHover,
  onSlotInspect,
  onBreedingInspect,
  onHatch,
  onMove,
  onBreedingClick,
  onTrashClick,
  dragValidDropSlots,
  isDraggingHandCard = false,
  canPlayDragged = false,
}: PlayerHalfProps) {
  const breedingClick = !isOpponent
    ? onBreedingClick ?? (canMove ? onMove : undefined)
    : undefined;

  return (
    <div className={`ib-player-half ${isOpponent ? 'ib-player-half--opp' : 'ib-player-half--you'}`}>
      <div className="ib-player-half__raise">
        <EggDeck
          count={player.eggDeckCount}
          canHatch={canHatch && !isOpponent}
          onClick={onHatch}
        />
        <BreedingArea
          permanent={player.breedingArea}
          canMove={canMove && !isOpponent}
          canDigivolveDrop={canDigivolveBreeding && !isOpponent}
          highlighted={highlightBreeding && !isOpponent}
          dropId={isOpponent ? 'breeding-slot-opponent' : 'breeding-slot'}
          onClick={breedingClick}
          onInspect={onBreedingInspect}
        />
      </div>

      <div className="ib-player-half__battle">
        <BattleArea
          permanents={player.battleArea}
          isOpponent={isOpponent}
          highlightedSlots={highlightedSlots}
          selectedSlots={selectedSlots}
          targetedSlots={targetedSlots}
          onSlotClick={onSlotClick}
          onSlotHover={onSlotHover}
          onSlotInspect={onSlotInspect}
          dragValidDropSlots={dragValidDropSlots}
          isDraggingHandCard={isDraggingHandCard}
          canPlayDragged={canPlayDragged}
        />
      </div>

      <div className="ib-player-half__resources">
        <DeckPile count={player.deckCount} />
        <SecurityStack count={player.securityCount} isOpponent={isOpponent} />
        <TrashPile count={player.trashIds.length} onClick={onTrashClick} />
      </div>
    </div>
  );
}
