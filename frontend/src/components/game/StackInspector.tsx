import { Card } from '@/components/shared/Card';
import { KeywordBadges } from '@/components/board/KeywordBadges';
import type { PermanentInfo } from '@/types/game';

interface StackInspectorProps {
  permanent: PermanentInfo | null;
  onClose: () => void;
}

export function StackInspector({ permanent, onClose }: StackInspectorProps) {
  if (!permanent) return null;

  return (
    <div className="w-[240px] bg-gray-800 border-l border-gray-700 flex flex-col h-full overflow-y-auto">
      <div className="flex items-center justify-between px-3 py-2 border-b border-gray-700">
        <span className="text-sm font-medium text-gray-200">
          {permanent.topCardName ?? 'Unknown'}
        </span>
        <button onClick={onClose} className="text-gray-500 hover:text-gray-300">&times;</button>
      </div>

      {/* Top card image */}
      {permanent.topCardId && (
        <div className="p-2 flex justify-center">
          <Card cardId={permanent.topCardId} size="lg" />
        </div>
      )}

      {/* Stats */}
      <div className="px-3 py-2 space-y-1 text-xs">
        {permanent.dp != null && (
          <div className="text-gray-400">DP: <span className="text-white font-bold">{permanent.dp}</span></div>
        )}
        {permanent.level != null && (
          <div className="text-gray-400">Level: <span className="text-white font-bold">{permanent.level}</span></div>
        )}
        {permanent.securityAttackModifier !== 0 && (
          <div className="text-gray-400">
            SA: <span className="text-yellow-400 font-bold">
              {permanent.securityAttackModifier > 0 ? '+' : ''}{permanent.securityAttackModifier}
            </span>
          </div>
        )}
      </div>

      {/* Keywords */}
      {permanent.keywords.length > 0 && (
        <div className="px-3 py-1">
          <div className="text-[10px] text-gray-500 mb-1">Keywords</div>
          <KeywordBadges keywords={permanent.keywords} />
        </div>
      )}

      {/* Digivolution stack */}
      {permanent.sources.length > 1 && (
        <div className="px-3 py-2">
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

      {/* Linked cards */}
      {permanent.linkedCardIds.length > 0 && (
        <div className="px-3 py-2">
          <div className="text-[10px] text-gray-500 mb-1">Linked Cards</div>
          <div className="flex gap-1">
            {permanent.linkedCardIds.map((id, i) => (
              <Card key={i} cardId={id} size="sm" />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
