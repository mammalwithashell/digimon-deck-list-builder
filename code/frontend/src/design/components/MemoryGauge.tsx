export interface MemoryGaugeProps {
  /** Memory value: positive = your side, negative = opponent's side. */
  value: number;
  max?: number;
}

/** Shared memory gauge — opponent pips fill leftward, your pips fill rightward. */
export function MemoryGauge({ value, max = 10 }: MemoryGaugeProps) {
  const oppCount = value < 0 ? Math.min(max, -value) : 0;
  const playerCount = value > 0 ? Math.min(max, value) : 0;
  const oppPips = Array.from({ length: max }, (_, i) => max - i <= oppCount);
  const playerPips = Array.from({ length: max }, (_, i) => i + 1 <= playerCount);
  return (
    <div className="ds-memgauge">
      <span className="ds-memgauge__label" style={{ color: 'var(--opp-2)' }}>OPP</span>
      <span className="ds-memgauge__pips">
        {oppPips.map((on, i) => (
          <span key={`o${i}`} className={`ds-memgauge__pip${on ? ' ds-memgauge__pip--opp' : ''}`} />
        ))}
        {playerPips.map((on, i) => (
          <span key={`p${i}`} className={`ds-memgauge__pip${on ? ' ds-memgauge__pip--player' : ''}`} />
        ))}
      </span>
      <span className="ds-memgauge__label" style={{ color: 'var(--player-2)' }}>YOU</span>
    </div>
  );
}
