interface SecurityStackProps {
  count: number;
  isOpponent: boolean;
}

export function SecurityStack({ count, isOpponent: _isOpponent }: SecurityStackProps) {
  return (
    <div className="flex flex-col items-center gap-1">
      <div className="w-[60px] h-[84px] bg-gradient-to-br from-purple-900 to-purple-700 border border-purple-600 rounded flex items-center justify-center relative">
        <span className="text-xl font-bold text-purple-200">{count}</span>
      </div>
      <span className="text-[9px] text-gray-500">Security</span>
    </div>
  );
}
