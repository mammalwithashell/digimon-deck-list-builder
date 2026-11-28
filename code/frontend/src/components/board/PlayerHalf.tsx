import { BattleArea } from './BattleArea';
import { EggDeck } from './EggDeck';
import { BreedingArea } from './BreedingArea';
import { DeckPile } from './DeckPile';
import { SecurityShieldRow } from './SecurityShieldRow';
import { TrashPile } from './TrashPile';
import type { PlayerState } from '@/types/game';

interface PlayerHalfProps {
  player: PlayerState;
  isOpponent: boolean;
  highlightedSlots?: Set<number>;
  selectedSlots?: Set<number>;
  targetedSlots?: Set<number>;
  attackableSlots?: Set<number>;
  canHatch?: boolean;
  canMove?: boolean;
  canDigivolveBreeding?: boolean;
  highlightBreeding?: boolean;
  canAttackSecurity?: boolean;
  onAttackSecurity?: () => void;
  onSlotClick?: (slotIndex: number) => void;
  onSlotHover?: (slotIndex: number | null) => void;
  onSlotInspect?: (slotIndex: number) => void;
  onInspectCard?: (cardId: string) => void;
  onBreedingInspect?: () => void;
  onHatch?: () => void;
  onMove?: () => void;
  onBreedingClick?: () => void;
  onTrashClick?: () => void;
  dragValidDropSlots?: Set<number>;
  isDraggingHandCard?: boolean;
  canPlayDragged?: boolean;
  isDraggingAttacker?: boolean;
}

/**
 * Hearthstone/Aegis-aligned half-board:
 * - Far left: Eggs + Raising side by side
 * - Center column: Open Battle Area (unobstructed full height)
 * - Far right: Security Shield Dock + Deck & Trash (100% clear of the fanned hand at every aspect ratio)
 */
export function PlayerHalf({
  player,
  isOpponent,
  highlightedSlots,
  selectedSlots,
  targetedSlots,
  attackableSlots,
  canHatch = false,
  canMove = false,
  canDigivolveBreeding = false,
  highlightBreeding = false,
  canAttackSecurity = false,
  onAttackSecurity,
  onSlotClick,
  onSlotHover,
  onSlotInspect,
  onInspectCard,
  onBreedingInspect,
  onHatch,
  onMove,
  onBreedingClick,
  onTrashClick,
  dragValidDropSlots,
  isDraggingHandCard = false,
  canPlayDragged = false,
  isDraggingAttacker = false,
}: PlayerHalfProps) {
  const breedingClick = !isOpponent
    ? onBreedingClick ?? (canMove ? onMove : undefined)
    : undefined;

  return (
    <div className={`hs-player-half ${isOpponent ? 'hs-player-half--opp' : 'hs-player-half--you'}`}>
      <div className="hs-player-half__raise">
        <EggDeck
          count={player.eggDeckCount}
          canHatch={canHatch && !isOpponent}
          isOpponent={isOpponent}
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

      <div className="hs-player-half__center">
        <BattleArea
          permanents={player.battleArea}
          isOpponent={isOpponent}
          highlightedSlots={highlightedSlots}
          selectedSlots={selectedSlots}
          targetedSlots={targetedSlots}
          attackableSlots={attackableSlots}
          onSlotClick={onSlotClick}
          onSlotHover={onSlotHover}
          onSlotInspect={onSlotInspect}
          dragValidDropSlots={dragValidDropSlots}
          isDraggingHandCard={isDraggingHandCard}
          canPlayDragged={canPlayDragged}
          isDraggingAttacker={isDraggingAttacker}
        />
      </div>

      <div className="hs-player-half__resources">
        <SecurityShieldRow
          count={player.securityCount}
          securityIds={player.securityIds}
          securityFaceUp={player.securityFaceUp}
          isOpponent={isOpponent}
          isAttackTarget={isOpponent && canAttackSecurity}
          onAttackSecurity={onAttackSecurity}
          onInspectCard={onInspectCard}
        />
        <div className="hs-player-half__piles">
          <DeckPile count={player.deckCount} isOpponent={isOpponent} />
          <TrashPile count={player.trashIds.length} onClick={onTrashClick} />
        </div>
      </div>
    </div>
  );
}
