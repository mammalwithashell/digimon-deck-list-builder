import { Card } from '@/components/shared/Card';
import { KeywordBadges } from './KeywordBadges';
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
  slotIndex: _slotIndex,
  isOpponent: _isOpponent,
  highlighted = false,
  targeted = false,
  onClick,
  onMouseEnter,
  onMouseLeave,
}: PermanentSlotProps) {
  if (!perm.topCardId) return null;

  return (
    <div
      className="flex flex-col items-center gap-0.5"
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
