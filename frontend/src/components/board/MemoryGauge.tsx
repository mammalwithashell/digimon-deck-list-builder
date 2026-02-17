import { MEMORY_MIN, MEMORY_MAX } from '@/utils/constants';

interface MemoryGaugeProps {
  value: number;
}

export function MemoryGauge({ value }: MemoryGaugeProps) {
  const segments = Array.from(
    { length: MEMORY_MAX - MEMORY_MIN + 1 },
    (_, i) => MEMORY_MIN + i,
  );

  return (
    <div className="flex items-center gap-px py-2 px-1">
      {segments.map((seg) => {
        const isActive = seg === value;
        const isPlayer1Side = seg > 0;
        const isPlayer2Side = seg < 0;
        const isZero = seg === 0;

        let bgClass: string;
        if (isActive) {
          bgClass = 'bg-yellow-400';
        } else if (isPlayer1Side) {
          bgClass = 'bg-blue-800/40';
        } else if (isPlayer2Side) {
          bgClass = 'bg-red-800/40';
        } else {
          bgClass = 'bg-gray-700';
        }

        return (
          <div key={seg} className="flex flex-col items-center">
            <div
              className={`w-5 h-6 rounded-sm flex items-center justify-center ${bgClass} ${
                isActive ? 'ring-1 ring-yellow-300' : ''
              }`}
            >
              {(isZero || isActive || seg % 5 === 0) && (
                <span
                  className={`text-[8px] font-bold ${
                    isActive ? 'text-gray-900' : 'text-gray-400'
                  }`}
                >
                  {seg}
                </span>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
