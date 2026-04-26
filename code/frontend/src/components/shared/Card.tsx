import { useCardImage } from '@/hooks/useCardImage';
import { COLOR_HEX, COLOR_NAMES } from '@/utils/constants';

type CardSize = 'sm' | 'md' | 'lg' | 'xl' | 'inspector';

const SIZE_MAP: Record<CardSize, { width: number; height: number }> = {
  sm: { width: 60, height: 84 },
  md: { width: 96, height: 134 },
  lg: { width: 100, height: 140 },
  xl: { width: 200, height: 280 },
  inspector: { width: 160, height: 224 },
};

interface CardOverlay {
  dp?: number;
  level?: number | null;
  keywords?: string[];
  saModifier?: number;
}

interface CardProps {
  cardId: string;
  cardName?: string;
  cardColor?: number;
  size?: CardSize;
  faceDown?: boolean;
  suspended?: boolean;
  dimmed?: boolean;
  highlighted?: boolean;
  targeted?: boolean;
  isAltArt?: boolean;
  overlay?: CardOverlay;
  onClick?: () => void;
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
  className?: string;
}

export function Card({
  cardId,
  cardName,
  cardColor,
  size = 'md',
  faceDown = false,
  suspended = false,
  dimmed = false,
  highlighted = false,
  targeted = false,
  isAltArt = false,
  overlay,
  onClick,
  onMouseEnter,
  onMouseLeave,
  className = '',
}: CardProps) {
  const { src, isLoading, hasError } = useCardImage(faceDown ? null : cardId, isAltArt);
  const { width, height } = SIZE_MAP[size];

  const colorName = cardColor !== undefined ? COLOR_NAMES[cardColor] : undefined;
  const bgColor = colorName ? COLOR_HEX[colorName] ?? '#374151' : '#374151';

  const borderClass = highlighted
    ? 'ring-2 ring-cyan-400 shadow-[0_0_10px_rgba(34,211,238,0.4)]'
    : targeted
      ? 'ring-2 ring-red-500 shadow-[0_0_10px_rgba(239,68,68,0.4)]'
      : '';

  return (
    <div
      className={`relative inline-block rounded overflow-hidden select-none cursor-pointer transition-shadow duration-200
        ${suspended ? 'rotate-90' : ''}
        ${dimmed ? 'opacity-30' : ''}
        ${!dimmed && !highlighted && !targeted ? 'hover:shadow-[0_0_8px_rgba(34,211,238,0.3)] hover:ring-1 hover:ring-cyan-500/40' : ''}
        ${borderClass} ${className}`}
      style={{ width, height }}
      title={faceDown ? undefined : cardId}
      onClick={onClick}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      {faceDown ? (
        <div
          className="w-full h-full bg-gradient-to-br from-blue-900 to-blue-700 border border-blue-600 rounded flex items-center justify-center"
        >
          <span className="text-blue-300 text-xs font-bold">DCG</span>
        </div>
      ) : isLoading ? (
        <div
          className="w-full h-full animate-pulse rounded"
          style={{ backgroundColor: bgColor + '40' }}
        />
      ) : hasError || !src ? (
        <div
          className="w-full h-full rounded border border-gray-600 flex flex-col items-center justify-center p-1 text-center"
          style={{ backgroundColor: bgColor + '60' }}
        >
          {cardName && (
            <span className="text-[8px] text-gray-200 leading-tight">{cardName}</span>
          )}
          {overlay?.level != null && (
            <span className="text-[10px] text-yellow-300 font-bold">Lv.{overlay.level}</span>
          )}
          {overlay?.dp != null && (
            <span className="text-[10px] text-white font-bold">{overlay.dp} DP</span>
          )}
        </div>
      ) : (
        <img
          src={src}
          alt={cardName ?? cardId}
          className="w-full h-full object-cover rounded"
          draggable={false}
        />
      )}

      {/* DP badge */}
      {!faceDown && overlay?.dp != null && src && (
        <div className="absolute bottom-0 left-0 bg-black/80 text-white text-[9px] font-bold px-1 rounded-tr">
          {overlay.dp}
        </div>
      )}

      {/* Level badge */}
      {!faceDown && overlay?.level != null && src && (
        <div className="absolute top-0 left-0 bg-yellow-600/90 text-white text-[9px] font-bold px-1 rounded-br">
          Lv.{overlay.level}
        </div>
      )}

      {/* SA modifier */}
      {!faceDown && overlay?.saModifier != null && overlay.saModifier !== 0 && src && (
        <div className="absolute top-0 right-0 bg-red-600/90 text-white text-[9px] font-bold px-1 rounded-bl">
          SA{overlay.saModifier > 0 ? '+' : ''}{overlay.saModifier}
        </div>
      )}

      {/* Alt art badge */}
      {!faceDown && isAltArt && (
        <div className="absolute top-0 right-0 bg-purple-600/90 text-white text-[8px] font-bold px-1 rounded-bl">
          ALT
        </div>
      )}
    </div>
  );
}
