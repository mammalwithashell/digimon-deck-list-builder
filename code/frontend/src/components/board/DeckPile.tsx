interface DeckPileProps {
  count: number;
}

export function DeckPile({ count }: DeckPileProps) {
  return (
    <div className="flex flex-col items-center gap-1">
      <div className="w-[60px] h-[84px] bg-gradient-to-br from-gray-700 to-gray-600 border border-gray-500 rounded flex items-center justify-center">
        <span className="text-xl font-bold text-gray-300">{count}</span>
      </div>
      <span className="text-[9px] text-gray-500">Deck</span>
    </div>
  );
}
