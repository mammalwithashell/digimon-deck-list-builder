import { useRef, useEffect } from 'react';

interface GameLogProps {
  logs: string[];
}

export function GameLog({ logs }: GameLogProps) {
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [logs.length]);

  return (
    <div className="flex flex-col h-full bg-gray-900/50 border border-gray-700 rounded overflow-hidden">
      <div className="px-2 py-1 bg-gray-800 text-xs text-gray-400 border-b border-gray-700">
        Game Log
      </div>
      <div className="flex-1 overflow-y-auto p-2 space-y-0.5">
        {logs.map((log, i) => (
          <div key={i} className="text-[11px] text-gray-400 leading-relaxed">
            {log}
          </div>
        ))}
        {logs.length === 0 && (
          <div className="text-xs text-gray-600">No log entries yet</div>
        )}
        <div ref={endRef} />
      </div>
    </div>
  );
}
