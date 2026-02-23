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
  targetedSlots?: Set<number>;
  canHatch?: boolean;
  canMove?: boolean;
  canDigivolveBreeding?: boolean;
  onSlotClick?: (slotIndex: number) => void;
  onSlotHover?: (slotIndex: number | null) => void;
  onHatch?: () => void;
  onMove?: () => void;
  onTrashClick?: () => void;
}

export function PlayerHalf({
  player,
  isOpponent,
  highlightedSlots,
  targetedSlots,
  canHatch = false,
  canMove = false,
  canDigivolveBreeding = false,
  onSlotClick,
  onSlotHover,
  onHatch,
  onMove,
  onTrashClick,
}: PlayerHalfProps) {
  return (
    <div className={`flex items-center gap-3 px-2 ${isOpponent ? 'flex-row-reverse' : ''}`}>
      {/* Left side zones */}
      <div className="flex flex-col items-center gap-1">
        <EggDeck
          count={player.eggDeckCount}
          canHatch={canHatch && !isOpponent}
          onClick={onHatch}
        />
        <BreedingArea
          permanent={player.breedingArea}
          canMove={canMove && !isOpponent}
          canDigivolveDrop={canDigivolveBreeding && !isOpponent}
          dropId={isOpponent ? 'breeding-slot-opponent' : 'breeding-slot'}
          onClick={onMove}
        />
      </div>

      {/* Battle area */}
      <div className="flex-1">
        <BattleArea
          permanents={player.battleArea}
          isOpponent={isOpponent}
          highlightedSlots={highlightedSlots}
          targetedSlots={targetedSlots}
          onSlotClick={onSlotClick}
          onSlotHover={onSlotHover}
        />
      </div>

      {/* Right side zones */}
      <div className="flex flex-col items-center gap-1">
        <DeckPile count={player.deckCount} />
        <SecurityStack count={player.securityCount} isOpponent={isOpponent} />
        <TrashPile count={player.trashIds.length} onClick={onTrashClick} />
      </div>
    </div>
  );
}
