import { Card } from '@/components/shared/Card';
import { KeywordBadges } from './KeywordBadges';
import { useGameStore } from '@/stores/gameStore';
import type { PermanentInfo } from '@/types/game';

interface PermanentSlotProps {
  perm: PermanentInfo;
  slotIndex: number;
  isOpponent: boolean;
  highlighted?: boolean;
  /** Already picked during an in-progress "choose N" selection. */
  selected?: boolean;
  targeted?: boolean;
  onClick?: () => void;
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
  /** Right-click (context-menu) to open the stack inspector. */
  onInspect?: () => void;
}

export function PermanentSlot({
  perm,
  slotIndex,
  isOpponent,
  highlighted = false,
  selected = false,
  targeted = false,
  onClick,
  onMouseEnter,
  onMouseLeave,
  onInspect,
}: PermanentSlotProps) {
  const activeEffectSlot = useGameStore((s) => s.activeEffectSlot);
  const activeEffectPlayer = useGameStore((s) => s.activeEffectPlayer);

  if (!perm.topCardId) return null;

  // Check if this slot has an active effect
  const isEffectSource =
    activeEffectSlot === slotIndex &&
    ((isOpponent && activeEffectPlayer === 2) ||
     (!isOpponent && activeEffectPlayer === 1));

  return (
    <div
      className={`ib-permanent-slot ${
        isEffectSource ? 'animate-effect-pulse ib-permanent-slot--effect' : ''
      }`}
      onClick={onClick}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
      onContextMenu={
        onInspect
          ? (e) => {
              // Suppress the browser menu and open the stack inspector.
              e.preventDefault();
              e.stopPropagation();
              onInspect();
            }
          : undefined
      }
    >
      <Card
        cardId={perm.topCardId}
        cardName={perm.topCardName ?? undefined}
        cardColor={perm.colors[0]}
        size="md"
        suspended={perm.isSuspended}
        highlighted={highlighted}
        selected={selected}
        targeted={targeted}
        overlay={{
          dp: perm.dp,
          level: perm.level,
          keywords: perm.keywords,
          saModifier: perm.securityAttackModifier,
        }}
      />
      {/* Selected-for-"choose N" check badge — mirrors the trash modal's badge */}
      {selected && (
        <div
          data-testid="permanent-selected-badge"
          className="absolute top-1 right-1 z-10 w-5 h-5 rounded-full bg-emerald-500 text-white text-xs font-bold flex items-center justify-center shadow"
        >
          ✓
        </div>
      )}
      {/* Digivolution-card count badge — `sourceCount` counts only the cards
          UNDER the top card (the top card is not a digivolution card), so it
          renders whenever there is at least one source. */}
      {perm.sourceCount > 0 && (
        <div className="ib-source-count">
          x{perm.sourceCount}
        </div>
      )}
      <KeywordBadges keywords={perm.keywords} />
    </div>
  );
}
