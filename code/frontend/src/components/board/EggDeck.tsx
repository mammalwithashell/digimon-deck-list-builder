interface EggDeckProps {
  count: number;
  canHatch?: boolean;
  onClick?: () => void;
}

export function EggDeck({ count, canHatch = false, onClick }: EggDeckProps) {
  return (
    <div className="flex flex-col items-center gap-1">
      <div
        className={`w-[60px] h-[84px] bg-gradient-to-br from-yellow-900 to-yellow-700 border rounded flex items-center justify-center cursor-pointer transition-all ${
          canHatch ? 'border-green-400 ring-1 ring-green-400/50' : 'border-yellow-600'
        }`}
        onClick={onClick}
      >
        <span className="text-xl font-bold text-yellow-200">{count}</span>
      </div>
      <span className="text-[9px] text-gray-500">Eggs</span>
    </div>
  );
}
