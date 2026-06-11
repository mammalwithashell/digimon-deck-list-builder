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
  isOpponent: boolean;
  highlighted: boolean;
  cardInfo?: HandCardInfo;
  onClick: () => void;
  onHover?: (cardId: string | null) => void;
  onHoverIndex?: (index: number | null) => void;
  onInspect?: (cardId: string) => void;
}

function DraggableHandCard({ cardId, index, isOpponent, highlighted, cardInfo, onClick, onHover, onHoverIndex, onInspect }: DraggableHandCardProps) {
  const setHoveredCard = useGameStore((s) => s.setHoveredCard);
  const dragData: DragData = { type: 'hand-card', handIndex: index, cardId };
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `hand-card-${isOpponent ? 'opponent' : 'player'}-${index}`,
    data: dragData,
    disabled: isOpponent,
  });

  const primaryColor = cardInfo?.colors[0];
  const colorName = primaryColor != null ? COLOR_NAMES[primaryColor] : undefined;
  const colorHex = colorName ? COLOR_HEX[colorName] ?? '#374151' : '#374151';

  return (
    <div
      ref={setNodeRef}
      {...listeners}
      {...attributes}
      className={`ib-hand-card ${highlighted ? 'ib-hand-card--ready' : ''} ${isDragging ? 'ib-hand-card--dragging' : ''}`}
      style={{ marginLeft: index > 0 ? '-12px' : 0, zIndex: isDragging ? 100 : index }}
      onContextMenu={(e) => {
        // DCGO parity: right-click inspects your own (visible) hand cards;
        // the opponent's face-down hand stays private. Always suppress the
        // webview's native context menu over the hand.
        e.preventDefault();
        if (!isOpponent && !isDragging) onInspect?.(cardId);
      }}
    >
      <Card
        cardId={cardId}
        size="md"
        faceDown={isOpponent}
        highlighted={highlighted}
        onClick={onClick}
        onMouseEnter={() => {
          if (!isOpponent) {
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

      {/* Stat overlays — own hand only */}
      {cardInfo && !isOpponent && (
        <>
          {/* Play cost (top-left) */}
          <div
            className="ib-hand-card__cost"
            style={{ backgroundColor: colorHex }}
          >
            {cardInfo.playCost}
          </div>

          {/* Level or type tag (top-right) */}
          {cardInfo.cardKind === CARD_KIND.Digimon && cardInfo.level != null && (
            <div className="ib-hand-card__tag">
              Lv.{cardInfo.level}
            </div>
          )}
          {cardInfo.cardKind === CARD_KIND.Option && (
            <div className="ib-hand-card__tag ib-hand-card__tag--option">
              OPT
            </div>
          )}
          {cardInfo.cardKind === CARD_KIND.Tamer && (
            <div className="ib-hand-card__tag ib-hand-card__tag--tamer">
              TMR
            </div>
          )}

          {/* DP (bottom-right) — Digimon only */}
          {cardInfo.cardKind === CARD_KIND.Digimon && cardInfo.dp != null && (
            <div className="ib-hand-card__dp">
              {cardInfo.dp >= 1000 ? `${Math.round(cardInfo.dp / 1000)}K` : cardInfo.dp}
            </div>
          )}
        </>
      )}
    </div>
  );
}

interface HandZoneProps {
  cardIds: string[];
  isOpponent: boolean;
  highlightedIndices?: Set<number>;
  handCards?: HandCardInfo[];
  onCardClick?: (handIndex: number) => void;
  onCardHover?: (cardId: string | null) => void;
  onCardHoverIndex?: (index: number | null) => void;
  /** Right-click (context-menu) a hand card to open the enlarged card detail. */
  onCardInspect?: (cardId: string) => void;
}

export function HandZone({
  cardIds,
  isOpponent,
  highlightedIndices,
  handCards,
  onCardClick,
  onCardHover,
  onCardHoverIndex,
  onCardInspect,
}: HandZoneProps) {
  return (
    <div className={`ib-hand-zone ${isOpponent ? 'ib-hand-zone--opp' : 'ib-hand-zone--you'}`}>
      {cardIds.map((cardId, i) => (
        <DraggableHandCard
          key={`${cardId}-${i}`}
          cardId={cardId}
          index={i}
          isOpponent={isOpponent}
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
