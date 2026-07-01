import { useGameStore } from '@/stores/gameStore';
import { useEffectiveLiveBackground } from '@/stores/uiStore';
import { usePeekStore } from '@/stores/peekStore';
import { LiveAtmosphere } from '@/components/layout/LiveAtmosphere';
import { PlayerHalf } from './PlayerHalf';
import { HandZone } from './HandZone';
import { MemoryGauge } from './MemoryGauge';
import { RevealedCardsZone } from './RevealedCardsZone';
import { ActionTraceTicker } from './ActionTraceTicker';
import { TensorDebugBadge } from './TensorDebugBadge';
import { anyFieldSelectionHighlights, fieldSelectionHighlights } from '@/utils/selectionTargets';
import { GamePhase, type ActionTrace, type TensorSummary } from '@/types/game';

interface GameBoardProps {
  onPlayCard?: (handIndex: number) => void;
  onSlotClick?: (isOpponent: boolean, slotIndex: number) => void;
  /** Right-click (context-menu) a permanent to open the stack inspector. */
  onSlotInspect?: (isOpponent: boolean, slotIndex: number) => void;
  /** Right-click a breeding-area permanent to inspect it. */
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
  /** Own/enemy slots already picked during a board-driven "choose N" selection. */
  selectedOwnSlots?: Set<number>;
  selectedEnemySlots?: Set<number>;
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
  /** Memory cost preview (positive = costs memory) */
  previewCost?: number | null;
  /** Callback when hovering a hand card by index */
  onHandCardHoverIndex?: (index: number | null) => void;
  /** Right-click (context-menu) a hand card to open the enlarged card detail. */
  onHandCardInspect?: (cardId: string) => void;
  actionTraces?: ActionTrace[];
  latestTensorSummary?: TensorSummary | null;
}

export function GameBoard({
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
  validRevealedIndices,
  dragValidDropSlots,
  isDraggingHandCard = false,
  canPlayDragged = false,
  onOwnTrashClick,
  onOpponentTrashClick,
  previewCost,
  onHandCardHoverIndex,
  onHandCardInspect,
  actionTraces = [],
  latestTensorSummary = null,
}: GameBoardProps) {
  const {
    player1,
    player2,
    memoryGauge,
    currentPhase,
    currentPlayer,
    turnCount,
    revealedCards,
    pendingSelection,
    playerLabels,
  } = useGameStore();

  // Board atmosphere animates only when effective live-background is on
  // (motion full + toggle). Drives the `is-live` gate on the board's
  // scanline-roll / grid-drift CSS (animate-board-atmosphere).
  const liveBg = useEffectiveLiveBackground();

  // "Peek the board" also fades the centered revealed-cards strip (a reveal
  // selection is one of the modal surfaces the PeekButton shows for), so the
  // player can read the board behind it before picking.
  const peeking = usePeekStore((s) => s.peeking);

  if (!player1 || !player2) {
    return (
      <div data-testid="game-board" className="flex items-center justify-center h-full text-gray-400">
        Loading game...
      </div>
    );
  }

  // During selection phases, highlight valid field targets. The engine
  // encodes own- and opponent-field targets in the same
  // `OWN_FIELD_START + slot` range and disambiguates by
  // `pendingSelection.kind`, so route by `kind` (not id range) — otherwise an
  // opponent-target prompt lights up the player's own slots. See
  // `utils/selectionTargets.ts`.
  const fieldHighlights = fieldSelectionHighlights(
    pendingSelection?.kind,
    pendingSelection?.validIndices ?? [],
  );
  // `AnyField` selections (`select_any_permanent`) span both battle areas and
  // decode the side from each action id — see `utils/selectionTargets.ts`.
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
  const latestActionLabel = (actionTraces as unknown as { at(index: number): ActionTrace | undefined }).at(-1)?.decoded.label ?? null;

  return (
    <div data-testid="game-board" className={`ib-board${liveBg ? ' is-live' : ''}`}>
      <div className="ib-board__mat" />
      <LiveAtmosphere surface="board" />
      <div className="ib-board__horizon" />
      <div className="ib-board__scanlines" />
      <div className="ib-board__vignette" />

      <div className="ib-board__top-chrome">
        <div className="ib-chrome-tag"><span className="ib-dot" />MATCH LIVE</div>
        <div className="ib-chrome-tag">TURN {String(turnCount).padStart(2, '0')}</div>
        <div className="ib-chrome-tag">{currentPlayer === 1 ? 'YOUR PRIORITY' : 'OPP PRIORITY'}</div>
        <ActionTraceTicker traces={actionTraces} />
      </div>

      <PlayerTag
        className="ib-player-tag--opp"
        accent="opp"
        label={playerLabels[2] ?? 'Opponent'}
        sublabel={`${player2.handCount} hand · ${player2.deckCount} deck`}
      />
      <PlayerTag
        className="ib-player-tag--you"
        accent="you"
        label={playerLabels[1] ?? 'You'}
        sublabel={`${player1.trashIds.length} trash · ${player1.eggDeckCount} eggs`}
      />

      <div className="ib-board__opponent-hand">
        <HandZone cardIds={player2.handIds} isOpponent />
      </div>

      {/* Flow stage: opponent field / memory gauge / player field share the
          vertical space as a flex column (see .ib-board__stage). The two
          fields are flex:1 (equal), so neither half is smushed. */}
      <div className="ib-board__stage">
        <div className="ib-board__side ib-board__side--opp">
          <PlayerHalf
            player={player2}
            isOpponent
            highlightedSlots={enemySlots}
            selectedSlots={selectedEnemySlots}
            onTrashClick={onOpponentTrashClick}
            targetedSlots={targetedSlots}
            onSlotClick={(i) => onSlotClick?.(true, i)}
            onSlotInspect={(i) => onSlotInspect?.(true, i)}
            onBreedingInspect={() => onBreedingInspect?.(true)}
          />
        </div>

        <div className="ib-board__gauge">
          <MemoryGauge
            value={memoryGauge}
            localPlayer={1}
            currentPhase={currentPhase}
            previewCost={previewCost}
            latestActionLabel={latestActionLabel}
          />
        </div>

        <div className="ib-board__side ib-board__side--player">
          <PlayerHalf
            player={player1}
            isOpponent={false}
            highlightedSlots={ownSlots}
            selectedSlots={selectedOwnSlots}
            canHatch={canHatch}
            canMove={canMove}
            canDigivolveBreeding={canDigivolveBreeding}
            highlightBreeding={highlightBreeding}
            onSlotClick={(i) => onSlotClick?.(false, i)}
            onSlotInspect={(i) => onSlotInspect?.(false, i)}
            onBreedingInspect={() => onBreedingInspect?.(false)}
            onHatch={onHatch}
            onMove={onMove}
            onBreedingClick={onBreedingClick}
            onTrashClick={onOwnTrashClick}
            dragValidDropSlots={dragValidDropSlots}
            isDraggingHandCard={isDraggingHandCard}
            canPlayDragged={canPlayDragged}
          />
        </div>
      </div>

      {/* Revealed-cards zone overlays the gauge area (absolute), so it lives
          outside the flow stage. */}
      {revealedCards.length > 0 && (
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

      <div className="ib-board__hand">
        <HandZone
          cardIds={player1.handIds}
          isOpponent={false}
          highlightedIndices={playableHandIndices}
          handCards={player1.handCards}
          onCardClick={onPlayCard}
          onCardHoverIndex={onHandCardHoverIndex}
          onCardInspect={onHandCardInspect}
        />
      </div>

      {/* Dev-only tensor telemetry — gated to dev builds so it never ships,
          and anchored bottom-left so it can't overlap the hand-count chip. */}
      {import.meta.env.DEV && <TensorDebugBadge summary={latestTensorSummary} />}
      <div className="ib-hand-count-chip" aria-label={`${player1.handCount} cards in your hand`}>
        <span className="ib-hand-count-chip__label">YOUR HAND</span>
        <span className="ib-hand-count-chip__value">{player1.handCount}</span>
      </div>
    </div>
  );
}

function PlayerTag({
  className,
  accent,
  label,
  sublabel,
}: {
  className: string;
  accent: 'opp' | 'you';
  label: string;
  sublabel: string;
}) {
  return (
    <div className={`ib-player-tag ${className} ib-player-tag--${accent}`}>
      <div className="ib-player-tag__avatar">{accent === 'you' ? 'Y' : 'O'}</div>
      <div>
        <div className="ib-player-tag__name">{label}</div>
        <div className="ib-player-tag__sub">{sublabel}</div>
      </div>
    </div>
  );
}

