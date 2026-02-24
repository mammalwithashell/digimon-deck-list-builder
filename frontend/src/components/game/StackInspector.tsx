import { Card } from '@/components/shared/Card';
import { KeywordBadges } from '@/components/board/KeywordBadges';
import type { PermanentInfo } from '@/types/game';

interface StackInspectorProps {
  permanent: PermanentInfo | null;
  onClose: () => void;
}

export function StackInspector({ permanent, onClose }: StackInspectorProps) {
  if (!permanent) return null;

  const nonZeroSourceDp = permanent.dpBreakdown.sources.filter((s) => s.value !== 0);

  return (
    <div className="w-[240px] bg-gray-800 border-l border-gray-700 flex flex-col h-full overflow-y-auto">
      <div className="flex items-center justify-between px-3 py-2 border-b border-gray-700">
        <span className="text-sm font-medium text-gray-200">
          {permanent.topCardName ?? 'Unknown'}
        </span>
        <button data-testid="stack-inspector-close" onClick={onClose} className="text-gray-500 hover:text-gray-300">&times;</button>
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

      {/* Main effect text */}
      {permanent.mainEffectText && (
        <div className="px-3 py-2" data-testid="stack-main-effect">
          <div className="text-[10px] text-gray-500 mb-1">Main Effect</div>
          <div className="text-[11px] text-gray-300 whitespace-pre-wrap leading-relaxed">
            {permanent.mainEffectText}
          </div>
        </div>
      )}

      {/* Keywords */}
      {permanent.keywords.length > 0 && (
        <div className="px-3 py-1">
          <div className="text-[10px] text-gray-500 mb-1">Keywords</div>
          <KeywordBadges keywords={permanent.keywords} />
        </div>
      )}

      {/* Keyword origin breakdown */}
      {(permanent.keywordBreakdown.innate.length > 0 || permanent.keywordBreakdown.gained.length > 0) && (
        <div className="px-3 py-2" data-testid="stack-keyword-breakdown">
          <div className="text-[10px] text-gray-500 mb-1">Keyword Origins</div>
          {permanent.keywordBreakdown.innate.length > 0 && (
            <div className="mb-1">
              <div className="text-[10px] text-gray-500">Innate</div>
              <KeywordBadges keywords={permanent.keywordBreakdown.innate} />
            </div>
          )}
          {permanent.keywordBreakdown.gained.length > 0 && (
            <div>
              <div className="text-[10px] text-gray-500">Gained</div>
              <KeywordBadges keywords={permanent.keywordBreakdown.gained} />
            </div>
          )}
        </div>
      )}

      {/* DP breakdown */}
      {permanent.dpBreakdown.total != null && (
        <div className="px-3 py-2" data-testid="stack-dp-breakdown">
          <div className="text-[10px] text-gray-500 mb-1">DP Breakdown</div>
          <div className="space-y-1 text-[11px]">
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
                Temporary: <span className="text-amber-300">
                  {permanent.dpBreakdown.temporary > 0 ? `+${permanent.dpBreakdown.temporary}` : permanent.dpBreakdown.temporary}
                </span>
              </div>
            )}
            <div className="text-gray-300">
              Total: <span className="text-white font-semibold">{permanent.dpBreakdown.total}</span>
            </div>
          </div>
        </div>
      )}

      {/* Inherited effects */}
      {permanent.inheritedEffects.length > 0 && (
        <div className="px-3 py-2" data-testid="stack-inherited-effects">
          <div className="text-[10px] text-gray-500 mb-1">Inherited Effects</div>
          <div className="space-y-2">
            {permanent.inheritedEffects.map((eff, i) => (
              <div key={`${eff.cardId}-${i}`} className="bg-gray-900/60 border border-gray-700 rounded p-1.5">
                <div className="flex items-start gap-1.5">
                  <Card cardId={eff.cardId} size="sm" />
                  <div className="min-w-0">
                    <div className="text-[10px] text-gray-400 truncate">{eff.cardName ?? eff.cardId}</div>
                    <div className="text-[11px] text-gray-200 whitespace-pre-wrap leading-relaxed">
                      {eff.text}
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
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
