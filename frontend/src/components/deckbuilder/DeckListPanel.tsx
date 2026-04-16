import { useMemo } from 'react';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';
import { DeckCardEntry } from './DeckCardEntry';
import type { DeckEntry } from '@/types/deck';

interface GroupedSection {
  label: string;
  entries: DeckEntry[];
  totalCount: number;
}

function groupByLevel(entries: DeckEntry[]): GroupedSection[] {
  const groups: Record<string, DeckEntry[]> = {
    'Digi-Eggs': [],
    'Lv.3 Rookies': [],
    'Lv.4 Champions': [],
    'Lv.5 Ultimates': [],
    'Lv.6+ Megas': [],
    'Tamers': [],
    'Options': [],
    'Other': [],
  };

  for (const entry of entries) {
    const card = entry.cardData;
    if (!card) {
      groups['Other']!.push(entry);
      continue;
    }
    const type = card.type;
    if (type === 'Digi-Egg') {
      groups['Digi-Eggs']!.push(entry);
    } else if (type === 'Tamer') {
      groups['Tamers']!.push(entry);
    } else if (type === 'Option') {
      groups['Options']!.push(entry);
    } else {
      const level = parseInt(card.level, 10);
      if (level <= 2) groups['Digi-Eggs']!.push(entry);
      else if (level === 3) groups['Lv.3 Rookies']!.push(entry);
      else if (level === 4) groups['Lv.4 Champions']!.push(entry);
      else if (level === 5) groups['Lv.5 Ultimates']!.push(entry);
      else if (level >= 6) groups['Lv.6+ Megas']!.push(entry);
      else groups['Other']!.push(entry);
    }
  }

  return Object.entries(groups)
    .filter(([, entries]) => entries.length > 0)
    .map(([label, entries]) => ({
      label,
      entries,
      totalCount: entries.reduce((sum, e) => sum + e.count, 0),
    }));
}

export function DeckListPanel() {
  const { mainDeck, eggDeck } = useDeckBuilderStore();

  const allEntries = useMemo(() => [...eggDeck, ...mainDeck], [eggDeck, mainDeck]);
  const sections = useMemo(() => groupByLevel(allEntries), [allEntries]);

  const totalMain = mainDeck.reduce((sum, e) => sum + e.count, 0);
  const totalEgg = eggDeck.reduce((sum, e) => sum + e.count, 0);

  return (
    <div className="flex flex-col gap-1 h-full overflow-y-auto">
      <div className="flex items-center justify-between px-1 py-1">
        <span className="text-xs text-gray-400">
          Main: <span className={totalMain === 50 ? 'text-green-400' : 'text-red-400'}>{totalMain}/50</span>
        </span>
        <span className="text-xs text-gray-400">
          Eggs: <span className={totalEgg <= 5 ? 'text-green-400' : 'text-red-400'}>{totalEgg}/5</span>
        </span>
      </div>

      {sections.map((section) => (
        <div key={section.label}>
          <div className="text-[10px] text-gray-500 uppercase tracking-wider px-1 py-0.5 border-b border-gray-700/50">
            {section.label} ({section.totalCount})
          </div>
          {section.entries.map((entry) => (
            <DeckCardEntry
              key={`${entry.cardId}${entry.isAltArt ? '-alt' : ''}`}
              entry={entry}
            />
          ))}
        </div>
      ))}

      {allEntries.length === 0 && (
        <div className="text-center text-gray-500 text-sm py-8">
          Add cards to your deck
        </div>
      )}
    </div>
  );
}
