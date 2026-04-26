import { useEffect, useRef, useState } from 'react';

interface SecurityStackProps {
  count: number;
  isOpponent: boolean;
}

export function SecurityStack({ count, isOpponent: _isOpponent }: SecurityStackProps) {
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
    <div className="flex flex-col items-center gap-1">
      <div
        className={`w-[60px] h-[84px] bg-gradient-to-br from-purple-900 to-purple-700 border border-purple-600 rounded flex items-center justify-center relative overflow-hidden ${
          breaking ? 'animate-security-break' : ''
        }`}
      >
        <span className="text-xl font-bold text-purple-200">{count}</span>
        {breaking && (
          <div className="absolute inset-0 pointer-events-none animate-security-shatter" />
        )}
      </div>
      <span className="text-[9px] text-gray-500">Security</span>
    </div>
  );
}
