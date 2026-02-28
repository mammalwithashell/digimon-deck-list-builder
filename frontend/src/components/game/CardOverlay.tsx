import { useEffect } from 'react';
import { Card } from '@/components/shared/Card';
import { KeywordBadges } from '@/components/board/KeywordBadges';
import type { PermanentInfo } from '@/types/game';
import { useGameStore } from '@/stores/gameStore';

interface CardOverlayProps {
  permanent: PermanentInfo | null;
  onClose: () => void;
}

/**
 * Floating card detail overlay anchored to the bottom-left of the board.
 * Shows on permanent click (inspectedPerm) or card hover (hoveredCard).
 */
export function CardOverlay({ permanent, onClose }: CardOverlayProps) {
  const hoveredCard = useGameStore((s) => s.hoveredCard);

  // Dismiss on Escape
  useEffect(() => {
    if (!permanent) return;
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [permanent, onClose]);

  // If only hovering (no permanent clicked), show a simple card preview
  if (!permanent && hoveredCard) {
    return (
      <div className="absolute bottom-4 left-4 z-30 pointer-events-none">
        <Card cardId={hoveredCard} size="inspector" />
      </div>
    );
  }

  if (!permanent) return null;

  const nonZeroSourceDp = permanent.dpBreakdown.sources.filter((s) => s.value !== 0);

  return (
    <div className="absolute bottom-4 left-4 z-30 w-[320px] max-h-[calc(100%-32px)] overflow-y-auto bg-gray-900/95 border border-gray-600 rounded-lg shadow-2xl backdrop-blur-sm">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-gray-700">
        <span className="text-sm font-semibold text-gray-100 truncate">
          {permanent.topCardName ?? 'Unknown'}
        </span>
        <button
          data-testid="card-overlay-close"
          onClick={onClose}
          className="text-gray-500 hover:text-gray-300 text-lg leading-none"
        >
          &times;
        </button>
      </div>

      <div className="p-3 flex gap-3">
        {/* Large card image */}
        {permanent.topCardId && (
          <div className="shrink-0">
            <Card cardId={permanent.topCardId} size="inspector" />
          </div>
        )}

        {/* Stats column */}
        <div className="flex-1 min-w-0 space-y-2">
          {/* Stat badges */}
          <div className="flex flex-wrap gap-1.5">
            {permanent.level != null && (
              <span className="px-2 py-0.5 bg-yellow-600/80 text-white text-xs font-bold rounded">
                Lv.{permanent.level}
              </span>
            )}
            {permanent.dp != null && (
              <span className="px-2 py-0.5 bg-gray-700 text-white text-xs font-bold rounded">
                {permanent.dp} DP
              </span>
            )}
            {permanent.securityAttackModifier !== 0 && (
              <span className="px-2 py-0.5 bg-red-700/80 text-white text-xs font-bold rounded">
                SA{permanent.securityAttackModifier > 0 ? '+' : ''}{permanent.securityAttackModifier}
              </span>
            )}
          </div>

          {/* Keywords */}
          {permanent.keywords.length > 0 && (
            <KeywordBadges keywords={permanent.keywords} />
          )}

          {/* Source count */}
          {permanent.sources.length > 1 && (
            <div className="text-[10px] text-gray-500">
              {permanent.sources.length} cards in stack
            </div>
          )}
        </div>
      </div>

      {/* Main effect text */}
      {permanent.mainEffectText && (
        <div className="px-3 py-2 border-t border-gray-700/50">
          <div className="text-[10px] text-gray-500 mb-1">Main Effect</div>
          <div className="text-[11px] text-gray-300 whitespace-pre-wrap leading-relaxed">
            {permanent.mainEffectText}
          </div>
        </div>
      )}

      {/* DP breakdown */}
      {permanent.dpBreakdown.total != null && (nonZeroSourceDp.length > 0 || permanent.dpBreakdown.temporary !== 0) && (
        <div className="px-3 py-2 border-t border-gray-700/50">
          <div className="text-[10px] text-gray-500 mb-1">DP Breakdown</div>
          <div className="space-y-0.5 text-[11px]">
            <div className="text-gray-400">
              Base: <span className="text-gray-200">{permanent.dpBreakdown.base ?? 0}</span>
            </div>
            {nonZeroSourceDp.map((src, i) => (
              <div key={`${src.cardId}-${i}`} className="text-gray-400">
                {src.cardName ?? src.cardId}: <span className="text-cyan-300">{src.value > 0 ? `+${src.value}` : src.value}</span>
              </div>
            ))}
            {permanent.dpBreakdown.temporary !== 0 && (
              <div className="text-gray-400">
                Temp: <span className="text-amber-300">
                  {permanent.dpBreakdown.temporary > 0 ? `+${permanent.dpBreakdown.temporary}` : permanent.dpBreakdown.temporary}
                </span>
              </div>
            )}
            <div className="text-gray-300 font-medium">
              Total: {permanent.dpBreakdown.total}
            </div>
          </div>
        </div>
      )}

      {/* Inherited effects */}
      {permanent.inheritedEffects.length > 0 && (
        <div className="px-3 py-2 border-t border-gray-700/50">
          <div className="text-[10px] text-gray-500 mb-1">Inherited Effects</div>
          <div className="space-y-1.5">
            {permanent.inheritedEffects.map((eff, i) => (
              <div key={`${eff.cardId}-${i}`} className="bg-gray-800/60 border border-gray-700 rounded p-1.5">
                <div className="flex items-start gap-1.5">
                  <Card cardId={eff.cardId} size="sm" />
                  <div className="min-w-0">
                    <div className="text-[10px] text-gray-400 truncate">{eff.cardName ?? eff.cardId}</div>
                    <div className="text-[11px] text-gray-200 whitespace-pre-wrap leading-relaxed">{eff.text}</div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Digivolution stack */}
      {permanent.sources.length > 1 && (
        <div className="px-3 py-2 border-t border-gray-700/50">
          <div className="text-[10px] text-gray-500 mb-1">
            Digivolution Stack ({permanent.sources.length})
          </div>
          <div className="flex gap-1 flex-wrap">
            {permanent.sources.map((src, i) => (
              <Card key={i} cardId={src.cardId} size="sm" />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
