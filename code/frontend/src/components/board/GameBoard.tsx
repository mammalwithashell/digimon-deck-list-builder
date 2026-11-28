import './arena.css';
import { useGameStore } from '@/stores/gameStore';
import { usePeekStore } from '@/stores/peekStore';
import { ArenaSurface } from './ArenaSurface';
import { ArenaHud } from './ArenaHud';
import { PlayerHalf } from './PlayerHalf';
import { HandZone } from './HandZone';
import { MemoryBand } from './MemoryBand';
import { RevealedCardsZone } from './RevealedCardsZone';
import { TensorDebugBadge } from './TensorDebugBadge';
import { anyFieldSelectionHighlights, fieldSelectionHighlights } from '@/utils/selectionTargets';
import { GamePhase, type ActionTrace, type GameState, type TensorSummary } from '@/types/game';

interface GameBoardProps {
  /** Optional presented snapshot from the presentation queue (defaults to live store state). */
  presentedState?: GameState | null;
  actionMask?: number[];
  isBusy?: boolean;
  isPresenting?: boolean;
  memoryDelta?: number | null;
  gameSeed?: string | null;
  canAttackSecurity?: boolean;
  onAttackSecurity?: () => void;
  onAction?: (actionId: number) => void;
  onUndo?: () => void;
  onSurrender?: () => void;
  onSkipPresentation?: () => void;
  onPlayCard?: (handIndex: number) => void;
  onSlotClick?: (isOpponent: boolean, slotIndex: number) => void;
  onSlotInspect?: (isOpponent: boolean, slotIndex: number) => void;
  onBreedingInspect?: (isOpponent: boolean) => void;
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
  selectedOwnSlots?: Set<number>;
  selectedEnemySlots?: Set<number>;
  targetedSlots?: Set<number>;
  attackableSlots?: Set<number>;
  validRevealedIndices?: Set<number>;
  dragValidDropSlots?: Set<number>;
  isDraggingHandCard?: boolean;
  canPlayDragged?: boolean;
  isDraggingAttacker?: boolean;
  onOwnTrashClick?: () => void;
  onOpponentTrashClick?: () => void;
  previewCost?: number | null;
  onHandCardHoverIndex?: (index: number | null) => void;
  onHandCardInspect?: (cardId: string) => void;
  actionTraces?: ActionTrace[];
  latestTensorSummary?: TensorSummary | null;
  /** Studio dual-seat mode: render both hands face-up and playable. */
  dualSeat?: boolean;
  /** Studio manual perspective flip: swap visual top/bottom seats without mutating player IDs. */
  flipped?: boolean;
  playableOpponentHandIndices?: Set<number>;
  onOpponentPlayCard?: (handIndex: number) => void;
}

export function GameBoard({
  presentedState,
  actionMask: propActionMask,
  isBusy = false,
  isPresenting = false,
  memoryDelta = null,
  gameSeed = null,
  canAttackSecurity = false,
  onAttackSecurity,
  onAction,
  onUndo,
  onSurrender,
  onSkipPresentation,
  onPlayCard,
  onSlotClick,
  onSlotInspect,
  onBreedingInspect,
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
  selectedOwnSlots,
  selectedEnemySlots,
  targetedSlots,
  attackableSlots,
  validRevealedIndices,
  dragValidDropSlots,
  isDraggingHandCard = false,
  canPlayDragged = false,
  isDraggingAttacker = false,
  onOwnTrashClick,
  onOpponentTrashClick,
  previewCost,
  onHandCardHoverIndex,
  onHandCardInspect,
  latestTensorSummary = null,
  dualSeat = false,
  flipped = false,
  playableOpponentHandIndices,
  onOpponentPlayCard,
}: GameBoardProps) {
  const store = useGameStore();
  const peeking = usePeekStore((s) => s.peeking);

  const player1 = presentedState?.player1 ?? store.player1;
  const player2 = presentedState?.player2 ?? store.player2;
  const memoryGauge = presentedState?.memoryGauge ?? store.memoryGauge;
  const currentPhase = presentedState?.currentPhase ?? store.currentPhase;
  const currentPlayer = presentedState?.currentPlayer ?? store.currentPlayer;
  const turnCount = presentedState?.turnCount ?? store.turnCount;
  const isGameOver = presentedState?.isGameOver ?? store.isGameOver;
  const revealedCards = presentedState?.revealedCards ?? store.revealedCards;
  const pendingSelection = store.pendingSelection;
  const playerLabels = store.playerLabels;
  const effectiveMask = propActionMask ?? store.actionMask;

  if (!player1 || !player2) {
    return (
      <div
        data-testid="game-board"
        className="flex items-center justify-center h-full text-[var(--ink-2)]"
      >
        Loading game...
      </div>
    );
  }

  const fieldHighlights = fieldSelectionHighlights(
    pendingSelection?.kind,
    pendingSelection?.validIndices ?? [],
  );
  const anyFieldHighlights = anyFieldSelectionHighlights(
    pendingSelection?.kind,
    pendingSelection?.validIndices ?? [],
    pendingSelection?.selectingPlayer,
  );

  const ownSlots = new Set([
    ...(highlightedOwnSlots ?? []),
    ...fieldHighlights.own,
    ...anyFieldHighlights.own,
  ]);
  const enemySlots = new Set([
    ...(highlightedEnemySlots ?? []),
    ...fieldHighlights.enemy,
    ...anyFieldHighlights.enemy,
  ]);

  const topPlayer = flipped ? player1 : player2;
  const bottomPlayer = flipped ? player2 : player1;
  const topLabels: Record<number, string> = flipped
    ? { 1: playerLabels[2] ?? 'Player 2', 2: playerLabels[1] ?? 'Player 1' }
    : playerLabels;

  return (
    <div data-testid="game-board" className="hs-board">
      <ArenaSurface />

      <ArenaHud
        turnCount={turnCount}
        player1={bottomPlayer}
        player2={topPlayer}
        playerLabels={topLabels}
        isGameOver={isGameOver}
        gameSeed={gameSeed}
        onUndo={onUndo}
        onSurrender={onSurrender}
        onSkipPresentation={onSkipPresentation}
        isPresenting={isPresenting}
      />

      <div className="hs-board__opp-hand">
        <HandZone
          cardIds={topPlayer.handIds}
          isOpponent
          showFaceUp={dualSeat}
          interactive={dualSeat}
          highlightedIndices={flipped ? playableHandIndices : playableOpponentHandIndices}
          handCards={topPlayer.handCards}
          onCardClick={flipped ? onPlayCard : onOpponentPlayCard}
          onCardInspect={onHandCardInspect}
        />
      </div>

      <div className="hs-board__stage">
        <div className="hs-board__half">
          <PlayerHalf
            player={topPlayer}
            isOpponent
            highlightedSlots={flipped ? ownSlots : enemySlots}
            selectedSlots={flipped ? selectedOwnSlots : selectedEnemySlots}
            canAttackSecurity={canAttackSecurity}
            onAttackSecurity={onAttackSecurity}
            onTrashClick={flipped ? onOwnTrashClick : onOpponentTrashClick}
            targetedSlots={targetedSlots}
            isDraggingAttacker={isDraggingAttacker}
            onSlotClick={(i) => onSlotClick?.(!flipped, i)}
            onSlotInspect={(i) => onSlotInspect?.(!flipped, i)}
            onInspectCard={onHandCardInspect}
            onBreedingInspect={() => onBreedingInspect?.(!flipped)}
          />
        </div>

        <MemoryBand
          value={memoryGauge}
          localPlayer={flipped ? 2 : 1}
          currentPhase={currentPhase}
          currentPlayer={currentPlayer}
          actionMask={effectiveMask}
          isGameOver={isGameOver}
          isBusy={isBusy}
          previewCost={previewCost}
          memoryDelta={memoryDelta}
          onAction={(id) => onAction?.(id)}
        />

        <div className="hs-board__half">
          <PlayerHalf
            player={bottomPlayer}
            isOpponent={false}
            highlightedSlots={flipped ? enemySlots : ownSlots}
            selectedSlots={flipped ? selectedEnemySlots : selectedOwnSlots}
            attackableSlots={attackableSlots}
            canHatch={canHatch}
            canMove={canMove}
            canDigivolveBreeding={canDigivolveBreeding}
            highlightBreeding={highlightBreeding}
            onSlotClick={(i) => onSlotClick?.(flipped, i)}
            onSlotInspect={(i) => onSlotInspect?.(flipped, i)}
            onInspectCard={onHandCardInspect}
            onBreedingInspect={() => onBreedingInspect?.(flipped)}
            onHatch={onHatch}
            onMove={onMove}
            onBreedingClick={onBreedingClick}
            onTrashClick={flipped ? onOpponentTrashClick : onOwnTrashClick}
            dragValidDropSlots={dragValidDropSlots}
            isDraggingHandCard={isDraggingHandCard}
            canPlayDragged={canPlayDragged}
          />
        </div>
      </div>

      {revealedCards.length > 0 &&
        !(
          pendingSelection?.selectingPlayer === 1 &&
          (currentPhase === GamePhase.SelectReveal ||
            currentPhase === GamePhase.SelectPermutation)
        ) && (
        <div
          className={`ib-board__revealed transition-opacity duration-150 ${
            peeking ? 'pointer-events-none opacity-0' : ''
          }`}
        >
          <RevealedCardsZone
            cards={revealedCards}
            validIndices={validRevealedIndices}
            onCardClick={onRevealedClick}
            title={
              currentPhase === GamePhase.SelectReveal && pendingSelection?.prompt
                ? pendingSelection.prompt
                : 'Revealed Cards'
            }
          />
        </div>
      )}

      <div className="hs-board__hand">
        <HandZone
          cardIds={bottomPlayer.handIds}
          isOpponent={false}
          highlightedIndices={flipped ? playableOpponentHandIndices : playableHandIndices}
          handCards={bottomPlayer.handCards}
          onCardClick={flipped ? onOpponentPlayCard : onPlayCard}
          onCardHoverIndex={onHandCardHoverIndex}
          onCardInspect={onHandCardInspect}
        />
      </div>

      {import.meta.env.DEV && <TensorDebugBadge summary={latestTensorSummary} />}
    </div>
  );
}
