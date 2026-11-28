import type { CSSProperties } from 'react';
import { useDraggable } from '@dnd-kit/core';
import { Card } from '@/components/shared/Card';
import { useGameStore } from '@/stores/gameStore';
import { COLOR_HEX, COLOR_NAMES } from '@/utils/constants';
import type { DragData } from '@/hooks/useDropZone';
import type { HandCardInfo } from '@/types/game';

const CARD_KIND = { Digimon: 0, Tamer: 1, Option: 2, DigiEgg: 3 } as const;

interface DraggableHandCardProps {
  cardId: string;
  index: number;
  total: number;
  isOpponent: boolean;
  showFaceUp: boolean;
  interactive: boolean;
  highlighted: boolean;
  cardInfo?: HandCardInfo;
  onClick: () => void;
  onHover?: (cardId: string | null) => void;
  onHoverIndex?: (index: number | null) => void;
  onInspect?: (cardId: string) => void;
}

function DraggableHandCard({
  cardId,
  index,
  total,
  isOpponent,
  showFaceUp,
  interactive,
  highlighted,
  cardInfo,
  onClick,
  onHover,
  onHoverIndex,
  onInspect,
}: DraggableHandCardProps) {
  const setHoveredCard = useGameStore((s) => s.setHoveredCard);
  const dragData: DragData = { type: 'hand-card', handIndex: index, cardId };
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `hand-card-${isOpponent ? 'opponent' : 'player'}-${index}`,
    data: dragData,
    disabled: !interactive,
  });

  const primaryColor = cardInfo?.colors[0];
  const colorName = primaryColor != null ? COLOR_NAMES[primaryColor] : undefined;
  const colorHex = colorName ? COLOR_HEX[colorName] ?? '#374151' : '#374151';

  // Gentle Hearthstone-style fan arc for the local player's hand
  const mid = (total - 1) / 2;
  const offset = index - mid;
  const maxStep = Math.min(2.4, 14 / Math.max(total, 1));
  const fanDeg = isOpponent ? 0 : offset * maxStep;
  const fanY = isOpponent ? 0 : Math.min(14, offset * offset * 1.15);

  const fanStyle: CSSProperties = {
    marginLeft: index > 0 ? (total > 8 ? '-18px' : '-10px') : 0,
    zIndex: isDragging ? 100 : index + 1,
    ['--hand-fan-deg' as string]: `${fanDeg.toFixed(2)}deg`,
    ['--hand-fan-y' as string]: `${fanY.toFixed(1)}px`,
  };

  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      className={`ib-hand-card ${highlighted ? 'ib-hand-card--ready' : ''} ${
        isDragging ? 'ib-hand-card--dragging' : ''
      }`}
      style={fanStyle}
      onContextMenu={(e) => {
        e.preventDefault();
        if (showFaceUp && !isDragging) onInspect?.(cardId);
      }}
    >
      <div className="ib-hand-card__lift">
        <Card
          cardId={cardId}
          cardName={cardInfo?.cardName}
          size="md"
          faceDown={!showFaceUp}
          useDefaultSleeve={isOpponent}
          highlighted={showFaceUp && highlighted}
          onClick={onClick}
          onMouseEnter={() => {
            if (showFaceUp && !isDragging) {
              setHoveredCard(cardId);
              onHover?.(cardId);
              onHoverIndex?.(index);
            }
          }}
          onMouseLeave={() => {
            setHoveredCard(null);
            onHover?.(null);
            onHoverIndex?.(null);
          }}
        />

        {cardInfo && showFaceUp && (
          <>
            <div className="ib-hand-card__cost" style={{ backgroundColor: colorHex }}>
              {cardInfo.playCost}
            </div>

            {cardInfo.cardKind === CARD_KIND.Digimon && cardInfo.level != null && (
              <div className="ib-hand-card__tag">Lv.{cardInfo.level}</div>
            )}
            {cardInfo.cardKind === CARD_KIND.Option && (
              <div className="ib-hand-card__tag ib-hand-card__tag--option">OPT</div>
            )}
            {cardInfo.cardKind === CARD_KIND.Tamer && (
              <div className="ib-hand-card__tag ib-hand-card__tag--tamer">TMR</div>
            )}

            {cardInfo.cardKind === CARD_KIND.Digimon && cardInfo.dp != null && (
              <div className="ib-hand-card__dp">
                {cardInfo.dp >= 1000 ? `${Math.round(cardInfo.dp / 1000)}K` : cardInfo.dp}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

interface HandZoneProps {
  cardIds: string[];
  isOpponent: boolean;
  /** Override to render opponent hand face-up (e.g. Studio dual-seat mode). */
  showFaceUp?: boolean;
  /** Override to allow clicking/dragging cards on this hand. */
  interactive?: boolean;
  highlightedIndices?: Set<number>;
  handCards?: HandCardInfo[];
  onCardClick?: (handIndex: number) => void;
  onCardHover?: (cardId: string | null) => void;
  onCardHoverIndex?: (index: number | null) => void;
  onCardInspect?: (cardId: string) => void;
}

export function HandZone({
  cardIds,
  isOpponent,
  showFaceUp,
  interactive,
  highlightedIndices,
  handCards,
  onCardClick,
  onCardHover,
  onCardHoverIndex,
  onCardInspect,
}: HandZoneProps) {
  const faceUp = showFaceUp ?? !isOpponent;
  const canInteract = interactive ?? !isOpponent;
  return (
    <div className={`ib-hand-zone ${isOpponent ? 'ib-hand-zone--opp' : 'ib-hand-zone--you'}`}>
      {cardIds.map((cardId, i) => (
        <DraggableHandCard
          key={`${cardId}-${i}`}
          cardId={cardId}
          index={i}
          total={cardIds.length}
          isOpponent={isOpponent}
          showFaceUp={faceUp}
          interactive={canInteract}
          highlighted={highlightedIndices?.has(i) ?? false}
          cardInfo={handCards?.[i]}
          onClick={() => onCardClick?.(i)}
          onHover={onCardHover}
          onHoverIndex={onCardHoverIndex}
          onInspect={onCardInspect}
        />
      ))}
      {cardIds.length === 0 && (
        <div className="ib-hand-zone__empty">No cards in hand</div>
      )}
    </div>
  );
}
