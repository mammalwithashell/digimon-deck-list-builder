import { useDeckBuilderStore } from '@/stores/deckBuilderStore';

const COLORS = ['Red', 'Blue', 'Yellow', 'Green', 'Black', 'Purple', 'White'];
const TYPES = ['Digimon', 'Tamer', 'Option', 'Digi-Egg'];
const LEVELS = ['2', '3', '4', '5', '6', '7'];

const COLOR_CHIPS: Record<string, string> = {
  Red: 'bg-red-600',
  Blue: 'bg-blue-600',
  Yellow: 'bg-yellow-500 text-black',
  Green: 'bg-green-600',
  Black: 'bg-gray-800 border border-gray-500',
  Purple: 'bg-purple-600',
  White: 'bg-gray-200 text-black',
};

function ToggleChip({
  label,
  active,
  onClick,
  colorClass,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
  colorClass?: string;
}) {
  return (
    <button
      onClick={onClick}
      className={`px-2 py-0.5 rounded text-xs font-medium transition-all ${
        active
          ? colorClass ?? 'bg-blue-600 text-white'
          : 'bg-gray-700 text-gray-400 hover:bg-gray-600'
      }`}
    >
      {label}
    </button>
  );
}

export function FilterBar() {
  const { filters, setFilters, resetFilters, searchQuery } = useDeckBuilderStore();

  const toggleColor = (color: string) => {
    const current = filters.color;
    const next = current.includes(color)
      ? current.filter((c) => c !== color)
      : [...current, color];
    setFilters({ color: next });
  };

  const toggleType = (type: string) => {
    const current = filters.type;
    const next = current.includes(type)
      ? current.filter((t) => t !== type)
      : [...current, type];
    setFilters({ type: next });
  };

  const toggleLevel = (level: string) => {
    const current = filters.level;
    const next = current.includes(level)
      ? current.filter((l) => l !== level)
      : [...current, level];
    setFilters({ level: next });
  };

  const hasActiveFilters =
    filters.color.length > 0 ||
    filters.type.length > 0 ||
    filters.level.length > 0 ||
    filters.set !== '' ||
    searchQuery !== '';

  return (
    <div className="flex flex-col gap-2 p-2 bg-gray-800/50 rounded">
      <div className="flex flex-wrap items-center gap-2">
        {/* Color filters */}
        <div className="flex gap-1">
          {COLORS.map((color) => (
            <ToggleChip
              key={color}
              label={color}
              active={filters.color.includes(color)}
              onClick={() => toggleColor(color)}
              colorClass={filters.color.includes(color) ? COLOR_CHIPS[color] : undefined}
            />
          ))}
        </div>

        <div className="w-px h-5 bg-gray-600" />

        {/* Type filters */}
        <div className="flex gap-1">
          {TYPES.map((type) => (
            <ToggleChip
              key={type}
              label={type}
              active={filters.type.includes(type)}
              onClick={() => toggleType(type)}
            />
          ))}
        </div>

        <div className="w-px h-5 bg-gray-600" />

        {/* Level filters */}
        <div className="flex gap-1">
          {LEVELS.map((level) => (
            <ToggleChip
              key={level}
              label={`Lv.${level}`}
              active={filters.level.includes(level)}
              onClick={() => toggleLevel(level)}
            />
          ))}
        </div>
      </div>

      {/* Set search + Clear button row */}
      <div className="flex items-center gap-2">
        <input
          type="text"
          placeholder="Set ID (e.g. BT20, EX8, ST1)"
          value={filters.set}
          onChange={(e) => setFilters({ set: e.target.value.trim() })}
          className="w-40 px-2 py-1 bg-gray-700 border border-gray-600 rounded text-xs text-gray-200 placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500"
        />
        {hasActiveFilters && (
          <button
            onClick={resetFilters}
            className="px-3 py-1 bg-red-600/80 hover:bg-red-500 text-white text-xs font-medium rounded transition-colors"
          >
            Clear All Filters
          </button>
        )}
      </div>
    </div>
  );
}
