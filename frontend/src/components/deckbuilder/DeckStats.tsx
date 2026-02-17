import { useMemo } from 'react';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import { COLOR_HEX } from '@/utils/constants';

export function DeckStats() {
  const { mainDeck, eggDeck } = useDeckBuilderStore();

  const stats = useMemo(() => {
    const allEntries = [...mainDeck, ...eggDeck];
    const totalCards = allEntries.reduce((sum, e) => sum + e.count, 0);

    // Color distribution
    const colorCounts: Record<string, number> = {};
    let totalPlayCost = 0;
    let costCards = 0;

    // Level curve
    const levelCounts: Record<string, number> = {};

    for (const entry of allEntries) {
      const card = entry.cardData;
      if (!card) continue;

      // Color
      const color = card.color || 'Unknown';
      colorCounts[color] = (colorCounts[color] ?? 0) + entry.count;

      // Cost
      const cost = parseInt(card.play_cost, 10);
      if (!isNaN(cost)) {
        totalPlayCost += cost * entry.count;
        costCards += entry.count;
      }

      // Level
      if (card.level) {
        const lvl = `Lv.${card.level}`;
        levelCounts[lvl] = (levelCounts[lvl] ?? 0) + entry.count;
      }
    }

    const avgCost = costCards > 0 ? (totalPlayCost / costCards).toFixed(1) : '-';

    return { totalCards, colorCounts, avgCost, levelCounts };
  }, [mainDeck, eggDeck]);

  const maxLevel = Math.max(...Object.values(stats.levelCounts), 1);

  return (
    <div className="space-y-2 p-2 bg-gray-800/50 rounded text-xs">
      {/* Color bar */}
      {Object.keys(stats.colorCounts).length > 0 && (
        <div className="flex h-3 rounded overflow-hidden">
          {Object.entries(stats.colorCounts).map(([color, count]) => (
            <div
              key={color}
              style={{
                width: `${(count / stats.totalCards) * 100}%`,
                backgroundColor: COLOR_HEX[color] ?? '#6b7280',
              }}
              title={`${color}: ${count}`}
            />
          ))}
        </div>
      )}

      {/* Avg cost */}
      <div className="text-gray-400">
        Avg Play Cost: <span className="text-gray-200">{stats.avgCost}</span>
      </div>

      {/* Level curve */}
      {Object.keys(stats.levelCounts).length > 0 && (
        <div className="flex items-end gap-1 h-12">
          {['Lv.2', 'Lv.3', 'Lv.4', 'Lv.5', 'Lv.6', 'Lv.7'].map((lv) => {
            const count = stats.levelCounts[lv] ?? 0;
            const pct = (count / maxLevel) * 100;
            return (
              <div key={lv} className="flex-1 flex flex-col items-center gap-0.5">
                <div
                  className="w-full bg-blue-600/60 rounded-t min-h-[2px]"
                  style={{ height: `${pct}%` }}
                />
                <span className="text-[8px] text-gray-500">{lv.replace('Lv.', '')}</span>
                {count > 0 && (
                  <span className="text-[8px] text-gray-400">{count}</span>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
