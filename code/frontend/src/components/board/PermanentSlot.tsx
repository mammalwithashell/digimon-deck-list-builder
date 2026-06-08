import { Card } from '@/components/shared/Card';
import { KeywordBadges } from './KeywordBadges';
import { useGameStore } from '@/stores/gameStore';
import type { PermanentInfo } from '@/types/game';

interface PermanentSlotProps {
  perm: PermanentInfo;
  slotIndex: number;
  isOpponent: boolean;
  highlighted?: boolean;
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
        targeted={targeted}
        overlay={{
          dp: perm.dp,
          level: perm.level,
          keywords: perm.keywords,
          saModifier: perm.securityAttackModifier,
        }}
      />
      {/* Source count badge */}
      {perm.sourceCount > 1 && (
        <div className="ib-source-count">
          x{perm.sourceCount}
        </div>
      )}
      <KeywordBadges keywords={perm.keywords} />
    </div>
  );
}
