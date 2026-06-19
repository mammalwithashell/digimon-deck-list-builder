import { useRef, useEffect, type ReactNode } from 'react';
import { useGameStore } from '@/stores/gameStore';

interface GameLogProps {
  logs: string[];
}

/** Regex to match [CARD_ID:Name] patterns in log messages */
const CARD_REF_PATTERN = /\[([A-Z]{2,3}\d{1,2}-\d{2,3}):([^\]]+)\]/g;

/** Parse a log line, replacing [ID:Name] with clickable spans */
function parseLogLine(line: string, onCardClick: (cardId: string) => void): ReactNode[] {
  const parts: ReactNode[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  // Reset regex state
  CARD_REF_PATTERN.lastIndex = 0;

  while ((match = CARD_REF_PATTERN.exec(line)) !== null) {
    const matchIdx = match.index ?? lastIndex;

    // Add text before this match
    if (matchIdx > lastIndex) {
      parts.push(line.slice(lastIndex, matchIdx));
    }

    const cardId = match[1]!;
    const cardName = match[2]!;
    parts.push(
      <button
        key={`${cardId}-${matchIdx}`}
        onClick={() => onCardClick(cardId)}
        className="text-cyan-400 hover:text-cyan-300 hover:underline cursor-pointer font-medium"
        title={cardId}
      >
        {cardName}
      </button>
    );

    lastIndex = matchIdx + match[0].length;
  }

  // Add remaining text
  if (lastIndex < line.length) {
    parts.push(line.slice(lastIndex));
  }

  return parts.length > 0 ? parts : [line];
}

export function GameLog({ logs }: GameLogProps) {
  const endRef = useRef<HTMLDivElement>(null);
  const setHoveredCard = useGameStore((s) => s.setHoveredCard);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs.length]);

  const handleCardClick = (cardId: string) => {
    setHoveredCard(cardId);
  };

  return (
    <div className="flex flex-col h-full bg-transparent border border-[var(--ib-line)] rounded overflow-hidden">
      <div className="px-2 py-1 bg-[var(--ib-panel-2)] text-xs text-[var(--ib-bone-dd)] border-b border-[var(--ib-line)]">
        Game Log
      </div>
      <div className="flex-1 overflow-y-auto p-2 space-y-0.5">
        {logs.map((log, i) => (
          <div key={i} className="text-[11px] text-[var(--ib-bone-dd)] leading-relaxed">
            {parseLogLine(log, handleCardClick)}
          </div>
        ))}
        {logs.length === 0 && (
          <div className="text-xs text-[var(--ib-bone-dd)]">No log entries yet</div>
        )}
        <div ref={endRef} />
      </div>
    </div>
  );
}
