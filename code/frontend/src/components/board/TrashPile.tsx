interface TrashPileProps {
  count: number;
  onClick?: () => void;
}

export function TrashPile({ count, onClick }: TrashPileProps) {
  return (
    <div className="flex flex-col items-center gap-1">
      <div
        className="w-[60px] h-[84px] bg-gradient-to-br from-red-900/50 to-red-800/50 border border-red-700/50 rounded flex items-center justify-center cursor-pointer hover:border-red-500/50"
        onClick={onClick}
      >
        <span className="text-xl font-bold text-red-300/70">{count}</span>
      </div>
      <span className="text-[9px] text-gray-500">Trash</span>
    </div>
  );
}
