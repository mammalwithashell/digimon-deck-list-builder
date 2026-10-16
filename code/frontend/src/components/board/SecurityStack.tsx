import { useEffect, useRef, useState } from 'react';

/** Shields drawn before the row stops growing (the label keeps the count). */
const MAX_SHIELDS = 10;

interface SecurityStackProps {
  count: number;
  isOpponent: boolean;
}

/** Security as a row of shields, the top card's shield lit. */
export function SecurityStack({ count, isOpponent }: SecurityStackProps) {
  const prevCount = useRef(count);
  const [breaking, setBreaking] = useState(false);

  useEffect(() => {
    if (count < prevCount.current) {
      setBreaking(true);
      const timer = setTimeout(() => setBreaking(false), 600);
      prevCount.current = count;
      return () => clearTimeout(timer);
    }
    prevCount.current = count;
  }, [count]);

  return (
    <div
      className={`ib-security ${isOpponent ? 'ib-security--opp' : 'ib-security--you'}`}
      aria-label={`${count} security cards`}
    >
      <div className="ib-security__label">Security · {count}</div>
      <div className={`ib-security__gems ${breaking ? 'animate-security-break' : ''}`}>
        {count === 0 ? (
          <span className="ib-security__empty">No security</span>
        ) : (
          Array.from({ length: Math.min(count, MAX_SHIELDS) }).map((_, i) => (
            <span key={i} className={`ib-security__gem ${i === 0 ? 'is-top' : ''}`} aria-hidden="true" />
          ))
        )}
        {breaking && (
          <div className="absolute inset-0 pointer-events-none animate-security-shatter" />
        )}
      </div>
    </div>
  );
}
