import { useEffect, useRef, useState } from 'react';
import type { BoardStyle } from '@/stores/uiStore';

/** Arena shields drawn before the row collapses to the label count alone. */
const MAX_SHIELDS = 10;

interface SecurityStackProps {
  count: number;
  isOpponent: boolean;
  /** `classic`: one shield with the count and a token strip. `arena`: a row
   *  of shield gems. */
  variant?: BoardStyle;
}

export function SecurityStack({ count, isOpponent, variant = 'classic' }: SecurityStackProps) {
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

  const shatter = breaking && (
    <div className="absolute inset-0 pointer-events-none animate-security-shatter" />
  );

  if (variant === 'arena') {
    return (
      <div
        className={`ib-security ${isOpponent ? 'ib-security--opp' : 'ib-security--you'}`}
        aria-label={`${count} security cards`}
      >
        <div className="ib-security__label">Security · {count}</div>
        <div className={`ib-security__shields relative ${breaking ? 'animate-security-break' : ''}`}>
          {count === 0 ? (
            <span className="ib-security__empty">No security</span>
          ) : (
            Array.from({ length: Math.min(count, MAX_SHIELDS) }).map((_, i) => (
              <span key={i} className={`ib-security__shield ${i === 0 ? 'is-top' : ''}`} aria-hidden="true" />
            ))
          )}
          {shatter}
        </div>
      </div>
    );
  }

  return (
    <div className={`ib-security ${isOpponent ? 'ib-security--opp' : 'ib-security--you'}`}>
      <div className="ib-security__label">Security</div>
      <div
        className={`ib-security__shield ${
          breaking ? 'animate-security-break' : ''
        }`}
      >
        <span>{String(count).padStart(2, '0')}</span>
        {shatter}
      </div>
      <div className="ib-security__tokens" aria-hidden="true">
        {Array.from({ length: Math.min(count, 5) }).map((_, i) => (
          <span key={i} className={i === 0 ? 'is-top' : ''} />
        ))}
      </div>
    </div>
  );
}
