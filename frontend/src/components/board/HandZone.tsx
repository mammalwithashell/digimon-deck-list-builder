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
  onHoverIndex?: (index: number | null) => void;
}

function DraggableHandCard({ cardId, index, isOpponent, highlighted, cardInfo, onClick, onHoverIndex }: DraggableHandCardProps) {
  const setHoveredCard = useGameStore((s) => s.setHoveredCard);
  const dragData: DragData = { type: 'hand-card', handIndex: index, cardId };
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({
    id: `hand-card-${index}`,
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
      className={`relative transition-transform hover:-translate-y-2 ${isDragging ? 'opacity-30' : ''}`}
      style={{ marginLeft: index > 0 ? '-12px' : 0, zIndex: isDragging ? 100 : index }}
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
            onHoverIndex?.(index);
          }
        }}
        onMouseLeave={() => {
          setHoveredCard(null);
          onHoverIndex?.(null);
        }}
      />

      {/* Stat overlays — own hand only */}
      {cardInfo && !isOpponent && (
        <>
          {/* Play cost (top-left) */}
          <div
            className="absolute top-0.5 left-0.5 flex items-center justify-center rounded-full
                        w-[18px] h-[18px] text-[10px] font-bold text-white shadow-sm
                        border border-white/30 pointer-events-none"
            style={{ backgroundColor: colorHex }}
          >
            {cardInfo.playCost}
          </div>

          {/* Level or type tag (top-right) */}
          {cardInfo.cardKind === CARD_KIND.Digimon && cardInfo.level != null && (
            <div className="absolute top-0.5 right-0.5 text-[9px] font-bold text-yellow-300
                            bg-black/60 rounded px-0.5 leading-tight pointer-events-none">
              Lv.{cardInfo.level}
            </div>
          )}
          {cardInfo.cardKind === CARD_KIND.Option && (
            <div className="absolute top-0.5 right-0.5 text-[9px] font-bold text-purple-300
                            bg-black/60 rounded px-0.5 leading-tight pointer-events-none">
              OPT
            </div>
          )}
          {cardInfo.cardKind === CARD_KIND.Tamer && (
            <div className="absolute top-0.5 right-0.5 text-[9px] font-bold text-cyan-300
                            bg-black/60 rounded px-0.5 leading-tight pointer-events-none">
              TMR
            </div>
          )}

          {/* DP (bottom-right) — Digimon only */}
          {cardInfo.cardKind === CARD_KIND.Digimon && cardInfo.dp != null && (
            <div className="absolute bottom-0.5 right-0.5 text-[9px] font-bold text-white
                            bg-black/60 rounded px-0.5 leading-tight pointer-events-none">
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
}

export function HandZone({
  cardIds,
  isOpponent,
  highlightedIndices,
  handCards,
  onCardClick,
  onCardHoverIndex,
}: HandZoneProps) {
  return (
    <div className="flex justify-center gap-[-8px] py-1">
      {cardIds.map((cardId, i) => (
        <DraggableHandCard
          key={`${cardId}-${i}`}
          cardId={cardId}
          index={i}
          isOpponent={isOpponent}
          highlighted={highlightedIndices?.has(i) ?? false}
          cardInfo={handCards?.[i]}
          onClick={() => onCardClick?.(i)}
          onHoverIndex={onCardHoverIndex}
        />
      ))}
      {cardIds.length === 0 && (
        <div className="text-xs text-gray-600 py-8">No cards in hand</div>
      )}
    </div>
  );
}
