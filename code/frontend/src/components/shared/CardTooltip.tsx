import { useCardImage } from '@/hooks/useCardImage';

interface CardTooltipProps {
  cardId: string;
  x: number;
  y: number;
}

export function CardTooltip({ cardId, x, y }: CardTooltipProps) {
  const { src, hasError } = useCardImage(cardId);

  if (hasError || !src) return null;

  // Offset so tooltip doesn't overlap cursor
  const left = x + 20;
  const top = Math.max(10, y - 140);

  return (
    <div
      className="fixed z-50 pointer-events-none shadow-xl rounded-lg overflow-hidden"
      style={{ left, top }}
    >
      <img
        src={src}
        alt={cardId}
        className="w-[200px] h-auto rounded-lg"
        draggable={false}
      />
    </div>
  );
}
