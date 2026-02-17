import { KEYWORD_COLORS, KEYWORD_DISPLAY } from '@/utils/constants';

interface KeywordBadgesProps {
  keywords: string[];
}

export function KeywordBadges({ keywords }: KeywordBadgesProps) {
  if (keywords.length === 0) return null;

  return (
    <div className="flex flex-wrap gap-0.5">
      {keywords.map((kw) => (
        <span
          key={kw}
          className="px-1 py-px rounded text-[7px] font-bold text-white leading-none"
          style={{ backgroundColor: KEYWORD_COLORS[kw] ?? '#6b7280' }}
          title={KEYWORD_DISPLAY[kw] ?? kw}
        >
          {(KEYWORD_DISPLAY[kw] ?? kw).slice(0, 6)}
        </span>
      ))}
    </div>
  );
}
