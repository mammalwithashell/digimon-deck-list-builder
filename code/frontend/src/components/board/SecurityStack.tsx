import { useEffect, useRef, useState } from 'react';

interface SecurityStackProps {
  count: number;
  isOpponent: boolean;
}

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
    <div className={`ib-security ${isOpponent ? 'ib-security--opp' : 'ib-security--you'}`}>
      <div className="ib-security__label">Security</div>
      <div
        className={`ib-security__shield ${
          breaking ? 'animate-security-break' : ''
        }`}
      >
        <span>{String(count).padStart(2, '0')}</span>
        {breaking && (
          <div className="absolute inset-0 pointer-events-none animate-security-shatter" />
        )}
      </div>
      <div className="ib-security__tokens" aria-hidden="true">
        {Array.from({ length: Math.min(count, 5) }).map((_, i) => (
          <span key={i} className={i === 0 ? 'is-top' : ''} />
        ))}
      </div>
    </div>
  );
}
