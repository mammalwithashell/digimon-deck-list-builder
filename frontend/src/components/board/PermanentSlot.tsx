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
      className={`flex flex-col items-center gap-0.5 ${
        isEffectSource ? 'animate-effect-pulse rounded' : ''
      }`}
      onClick={onClick}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
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
        <div className="text-[8px] text-gray-400">
          {perm.sourceCount} sources
        </div>
      )}
      <KeywordBadges keywords={perm.keywords} />
    </div>
  );
}
