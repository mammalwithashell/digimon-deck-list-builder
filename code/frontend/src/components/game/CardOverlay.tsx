import { useEffect } from 'react';
import { Card } from '@/components/shared/Card';
import { KeywordBadges } from '@/components/board/KeywordBadges';
import type { PermanentInfo, SourceInfo } from '@/types/game';
import { useGameStore } from '@/stores/gameStore';
import { COLOR_HEX, COLOR_NAMES } from '@/utils/constants';
import { groupModifiers } from '@/utils/modifierDisplay';
import { CardEffectText } from '@/utils/formatCardText';

interface CardOverlayProps {
  permanent: PermanentInfo | null;
  onClose: () => void;
  /**
   * Right-click (context-menu) a card image in the stack to open it in the
   * enlarged CardDetailOverlay — without closing this panel (DCGO's
   * CardInfo → OpenCardDetail flow).
   */
  onInspectCard?: (cardId: string) => void;
}

/**
 * DCGO-style PermanentDetail panel — floating card detail overlay anchored
 * to the bottom-left. Shows a vertical scrollable card stack with
 * color-coded borders and inline inherited effect text.
 */
export function CardOverlay({ permanent, onClose, onInspectCard }: CardOverlayProps) {
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

  // Hover-only preview (no permanent clicked)
  if (!permanent && hoveredCard) {
    return (
      <div className="absolute bottom-4 left-4 z-30 pointer-events-none">
        <Card cardId={hoveredCard} size="inspector" />
      </div>
    );
  }

  if (!permanent) return null;

  const saValue = 1 + permanent.securityAttackModifier;
  const nonZeroSourceDp = permanent.dpBreakdown.sources.filter((s) => s.value !== 0);
  const hasDpBreakdown = permanent.dpBreakdown.total != null &&
    (nonZeroSourceDp.length > 0 || permanent.dpBreakdown.temporary !== 0);

  // Reverse sources so top card (last in array) comes first visually
  const stackSources = [...permanent.sources].reverse();
  // Linked cards (from linkedCardIds)
  const linkedIds = permanent.linkedCardIds;
  // Active modifiers, grouped (immunities / restrictions / stat changes / other)
  const modifierGroups = groupModifiers(permanent.modifiers ?? []);

  const inspectOnRightClick = (cardId: string) => (e: React.MouseEvent) => {
    e.preventDefault();
    onInspectCard?.(cardId);
  };

  return (
    <div className="absolute bottom-4 left-4 z-30 w-[340px] max-h-[calc(100%-32px)] overflow-hidden bg-gray-900/95 border border-gray-600 rounded-lg shadow-2xl backdrop-blur-sm animate-overlay-open flex flex-col">
      {/* ── Header ── */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-gray-700 shrink-0">
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

      {/* ── Stats & Effects Summary ── */}
      <div className="px-3 py-2 border-b border-gray-700/50 shrink-0 space-y-1.5">
        {/* Stat badges */}
        <div className="flex flex-wrap gap-1.5">
          {permanent.level != null && (
            <span className="px-2 py-0.5 bg-yellow-600/80 text-white text-xs font-bold rounded">
              Lv.{permanent.level}
            </span>
          )}
          {permanent.dp != null && (
            <span className="px-2 py-0.5 bg-gray-700 text-white text-xs font-bold rounded">
              {permanent.dp.toLocaleString()} DP
            </span>
          )}
          {permanent.securityAttackModifier !== 0 && (
            <span className="px-2 py-0.5 bg-red-700/80 text-white text-xs font-bold rounded">
              SA{permanent.securityAttackModifier > 0 ? '+' : ''}{permanent.securityAttackModifier}
            </span>
          )}
        </div>

        {/* Security Attack line (DCGO style) */}
        <div className="text-[11px] text-gray-400">
          Security Attack: <span className="text-gray-200 font-medium">{saValue}</span>
        </div>

        {/* Keywords */}
        {permanent.keywords.length > 0 && (
          <KeywordBadges keywords={permanent.keywords} />
        )}

        {/* Active effects summary (DCGO bulleted list) */}
        {permanent.keywords.length > 0 && (
          <div className="text-[11px] text-gray-400 space-y-0.5">
            {permanent.keywords.map((kw) => {
              const isGained = permanent.keywordBreakdown.gained.includes(kw);
              return (
                <div key={kw}>
                  <span className="text-gray-500">-</span>{' '}
                  <span className={isGained ? 'text-cyan-300' : 'text-gray-300'}>{kw}</span>
                  {isGained && <span className="text-gray-600 text-[9px] ml-1">(granted)</span>}
                </div>
              );
            })}
          </div>
        )}

        {/* Active modifiers (immunities, restrictions, stat changes) — DCGO-style */}
        {modifierGroups.length > 0 && (
          <div data-testid="active-modifiers" className="pt-1 mt-1 border-t border-gray-700/40 space-y-1">
            <div className="text-[9px] text-gray-500 font-bold uppercase tracking-wide">
              Active Modifiers
            </div>
            {modifierGroups.map((grp) => (
              <div key={grp.group} className="space-y-0.5">
                {grp.items.map((item, i) => (
                  <div key={`${grp.group}-${i}`} className="text-[11px] flex items-baseline gap-1">
                    <span style={{ color: grp.color }}>•</span>
                    <span className="text-gray-200">{item.label}</span>
                    {item.expiryHint && (
                      <span className="text-gray-500 text-[9px]">({item.expiryHint})</span>
                    )}
                  </div>
                ))}
              </div>
            ))}
          </div>
        )}
      </div>

      {/* ── Scrollable card stack ── */}
      <div className="flex-1 overflow-y-auto px-3 py-2 space-y-2">
        {/* Top card (large) + main effect */}
        {permanent.topCardId && (
          <div
            className="rounded-lg overflow-hidden"
            style={{ borderLeft: `3px solid ${colorBorder(permanent.colors)}` }}
          >
            <div className="p-2 bg-gray-800/40">
              <Card
                cardId={permanent.topCardId}
                size="inspector"
                onContextMenu={inspectOnRightClick(permanent.topCardId)}
              />
              {permanent.mainEffectText && (
                <CardEffectText text={permanent.mainEffectText} className="mt-2" />
              )}
            </div>
          </div>
        )}

        {/* Source cards (non-top, in stack order) */}
        {stackSources
          .filter((src) => !src.isTop)
          .map((src, i) => (
            <StackCardEntry
              key={`${src.cardId}-${i}`}
              source={src}
              onInspectCard={onInspectCard}
            />
          ))}

        {/* Linked cards */}
        {linkedIds.length > 0 && (
          <>
            {linkedIds.map((cardId, i) => (
              <div
                key={`link-${cardId}-${i}`}
                className="rounded-lg overflow-hidden border-l-3"
                style={{ borderLeft: '3px solid #a855f7' }}
              >
                <div className="p-2 bg-purple-900/20">
                  <div className="text-[9px] text-purple-400 font-bold uppercase mb-1">Linked</div>
                  <Card
                    cardId={cardId}
                    size="md"
                    onContextMenu={inspectOnRightClick(cardId)}
                  />
                </div>
              </div>
            ))}
          </>
        )}
      </div>

      {/* ── DP Breakdown (bottom) ── */}
      {hasDpBreakdown && (
        <div className="px-3 py-2 border-t border-gray-700/50 shrink-0">
          <div className="text-[10px] text-gray-500 mb-0.5">DP Breakdown</div>
          <div className="flex flex-wrap gap-x-3 gap-y-0.5 text-[11px]">
            <span className="text-gray-400">
              Base: <span className="text-gray-200">{permanent.dpBreakdown.base ?? 0}</span>
            </span>
            {nonZeroSourceDp.map((src, i) => (
              <span key={`${src.cardId}-${i}`} className="text-gray-400">
                {src.cardName ?? src.cardId}: <span className="text-cyan-300">{src.value > 0 ? `+${src.value}` : src.value}</span>
              </span>
            ))}
            {permanent.dpBreakdown.temporary !== 0 && (
              <span className="text-gray-400">
                Temp: <span className="text-amber-300">
                  {permanent.dpBreakdown.temporary > 0 ? `+${permanent.dpBreakdown.temporary}` : permanent.dpBreakdown.temporary}
                </span>
              </span>
            )}
            <span className="text-gray-300 font-medium">
              = {permanent.dpBreakdown.total?.toLocaleString()}
            </span>
          </div>
        </div>
      )}
    </div>
  );
}

/** Single card entry in the digivolution stack, matching DCGO's CardInfo. */
function StackCardEntry({
  source,
  onInspectCard,
}: {
  source: SourceInfo;
  onInspectCard?: (cardId: string) => void;
}) {
  const borderColor = colorBorder(source.colors);
  const hasInherited = source.inheritedEffectText.trim().length > 0;

  // Hidden / face-down source (opponent's cards)
  if (!source.cardId) {
    return (
      <div
        className="rounded-lg overflow-hidden"
        style={{ borderLeft: '3px solid #6b7280' }}
      >
        <div className="p-2 bg-gray-800/40 text-center text-gray-500 text-sm py-6">
          ???
        </div>
      </div>
    );
  }

  return (
    <div
      className="rounded-lg overflow-hidden"
      style={{ borderLeft: `3px solid ${borderColor}` }}
    >
      <div className="p-2 bg-gray-800/40">
        <div className="flex items-start gap-2">
          <Card
            cardId={source.cardId}
            size="md"
            onContextMenu={(e) => {
              e.preventDefault();
              onInspectCard?.(source.cardId);
            }}
          />
          {hasInherited && (
            <div className="flex-1 min-w-0 pt-1">
              <div className="text-[9px] text-amber-500 font-bold uppercase mb-0.5">
                Inherited Effect
              </div>
              <CardEffectText text={source.inheritedEffectText} />
            </div>
          )}
        </div>
        {source.cardName && (
          <div className="text-[10px] text-gray-500 mt-1 truncate">{source.cardName}</div>
        )}
      </div>
    </div>
  );
}

/** Get CSS border color from card color enum values. */
function colorBorder(colors: number[] | undefined): string {
  if (!colors || colors.length === 0) return '#6b7280'; // gray fallback
  const colorVal = colors[0];
  if (colorVal === undefined) return '#6b7280';
  const name = COLOR_NAMES[colorVal];
  if (!name) return '#6b7280';
  return COLOR_HEX[name] ?? '#6b7280';
}
