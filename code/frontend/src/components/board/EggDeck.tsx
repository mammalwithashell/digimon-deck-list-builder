import type { KeyboardEvent } from 'react';
import { Card } from '@/components/shared/Card';

interface EggDeckProps {
  count: number;
  canHatch?: boolean;
  isOpponent?: boolean;
  onClick?: () => void;
}

export function EggDeck({
  count,
  canHatch = false,
  isOpponent = false,
  onClick,
}: EggDeckProps) {
  const interactive = canHatch && count > 0 && Boolean(onClick);
  const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (interactive && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      onClick?.();
    }
  };

  return (
    <div
      className={`ib-zone ib-zone--egg ${canHatch ? 'ib-zone--ready' : ''}`}
      data-testid="egg-deck"
    >
      <div className="ib-zone__label">Eggs</div>
      <div
        className={`ib-zone__body ib-zone__body--egg relative overflow-hidden p-0 transition-all duration-200 ${
          canHatch
            ? 'cursor-pointer ring-2 ring-emerald-400 shadow-[0_0_18px_rgba(52,211,153,0.65)] hover:scale-[1.04]'
            : ''
        }`}
        style={{ width: 60, height: 84 }}
        role={interactive ? 'button' : undefined}
        tabIndex={interactive ? 0 : undefined}
        aria-label={
          canHatch
            ? `Hatch Digi-Egg (${count} remaining)`
            : `Digi-Egg deck: ${count} cards`
        }
        title={canHatch ? 'Click to hatch a Digi-Egg' : `Digi-Eggs: ${count}`}
        onClick={canHatch ? onClick : undefined}
        onKeyDown={handleKeyDown}
      >
        {count > 0 ? (
          <Card
            cardId="egg-deck-back"
            size="sm"
            faceDown
            isEgg
            useDefaultSleeve={isOpponent}
            className="!block !w-full !h-full pointer-events-none"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center text-[10px] font-mono text-[var(--ink-2)] opacity-60">
            Empty
          </div>
        )}
        {canHatch && count > 0 && (
          <div
            data-testid="egg-hatch-indicator"
            className="absolute top-1 left-1/2 -translate-x-1/2 z-10 px-1.5 py-0.5 rounded-full bg-emerald-400 text-slate-950 text-[9px] font-extrabold uppercase tracking-wider leading-none shadow-[0_0_10px_rgba(52,211,153,0.9)] animate-pulse pointer-events-none whitespace-nowrap"
          >
            Hatch
          </div>
        )}
        <div
          data-testid="egg-deck-count"
          className="absolute bottom-1 right-1 z-10 min-w-[20px] px-1.5 py-0.5 rounded-full bg-slate-950/85 border border-white/25 text-white text-[11px] font-bold leading-none text-center tabular-nums shadow-md pointer-events-none"
        >
          {count}
        </div>
      </div>
    </div>
  );
}
