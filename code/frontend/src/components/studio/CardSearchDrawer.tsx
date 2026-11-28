import { useMemo, useState } from 'react';
import { useStudioStore } from '@/stores/studioStore';
import { COLOR_HEX, COLOR_NAMES } from '@/utils/constants';

export type PlacementTarget =
  | 'hand'
  | 'field_new'
  | 'field_stack_top'
  | 'field_stack_bottom'
  | 'field_link'
  | 'breeding'
  | 'security'
  | 'trash'
  | 'deck_top'
  | 'egg_deck';

const CARD_KIND_LABELS: Record<number, string> = {
  0: 'Digimon',
  1: 'Tamer',
  2: 'Option',
  3: 'Digi-Egg',
};

const COLOR_OPTIONS: Array<{ code: number; name: string }> = [
  { code: 0, name: 'Red' },
  { code: 1, name: 'Blue' },
  { code: 2, name: 'Yellow' },
  { code: 3, name: 'Green' },
  { code: 4, name: 'White' },
  { code: 5, name: 'Black' },
  { code: 6, name: 'Purple' },
];

const LEVEL_OPTIONS = [2, 3, 4, 5, 6, 7];

interface CardSearchDrawerProps {
  targetPlayer: 1 | 2;
  onTargetPlayerChange: (p: 1 | 2) => void;
  placementTarget: PlacementTarget;
  onPlacementTargetChange: (t: PlacementTarget) => void;
  selectedSlotIndex: number | null;
  onPlaceCard: (cardId: string) => void;
}

export function CardSearchDrawer({
  targetPlayer,
  onTargetPlayerChange,
  placementTarget,
  onPlacementTargetChange,
  selectedSlotIndex,
  onPlaceCard,
}: CardSearchDrawerProps) {
  const cardCatalog = useStudioStore((s) => s.cardCatalog);
  const implementedCardIds = useStudioStore((s) => s.implementedCardIds);

  const [query, setQuery] = useState('');
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [implementedOnly, setImplementedOnly] = useState(true);
  const [selectedColors, setSelectedColors] = useState<number[]>([]);
  const [selectedLevels, setSelectedLevels] = useState<number[]>([]);
  const [selectedKinds, setSelectedKinds] = useState<number[]>([]);

  const activeFilterCount =
    selectedColors.length +
    selectedLevels.length +
    selectedKinds.length +
    (implementedOnly ? 0 : 1);

  const results = useMemo(() => {
    const q = query.trim().toLowerCase();
    // Fallback to implementedCardIds if catalog is empty (e.g. minimal mock)
    if (cardCatalog.length === 0) {
      const list = [...implementedCardIds].sort();
      const filtered = q ? list.filter((id) => id.toLowerCase().includes(q)) : list;
      return filtered.slice(0, 80).map((card_id) => ({
        card_id,
        name: card_id,
        card_kind: 0,
        level: null as number | null,
        colors: [] as number[],
        dp: null as number | null,
        play_cost: null as number | null,
        traits: [] as string[],
        effect_text: '',
        inherited_text: '',
        security_text: '',
        implemented: true,
      }));
    }

    const out = [];
    for (const c of cardCatalog) {
      if (implementedOnly && !c.implemented) continue;
      if (selectedColors.length > 0 && !selectedColors.some((col) => c.colors.includes(col))) {
        continue;
      }
      if (selectedLevels.length > 0 && (c.level == null || !selectedLevels.includes(c.level))) {
        continue;
      }
      if (selectedKinds.length > 0 && !selectedKinds.includes(c.card_kind)) {
        continue;
      }
      if (q) {
        const matchName = c.name.toLowerCase().includes(q);
        const matchId = c.card_id.toLowerCase().includes(q);
        const matchText =
          c.effect_text.toLowerCase().includes(q) ||
          c.inherited_text.toLowerCase().includes(q) ||
          c.security_text.toLowerCase().includes(q) ||
          c.traits.some((t) => t.toLowerCase().includes(q));
        if (!matchName && !matchId && !matchText) continue;
      }
      out.push(c);
      if (out.length >= 100) break;
    }
    return out;
  }, [cardCatalog, implementedCardIds, query, implementedOnly, selectedColors, selectedLevels, selectedKinds]);

  const toggleNum = (list: number[], val: number, setter: (v: number[]) => void) => {
    setter(list.includes(val) ? list.filter((x) => x !== val) : [...list, val]);
  };

  return (
    <div
      className="studio-panel"
      style={{ flex: 1, minHeight: 0 }}
      data-testid="studio-card-search"
    >
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span className="studio-panel__title">Card Search</span>
        <button
          type="button"
          className="studio-btn studio-btn--sm"
          aria-expanded={filtersOpen}
          data-testid="studio-filters-toggle"
          onClick={() => setFiltersOpen((o) => !o)}
        >
          Filters{activeFilterCount > 0 ? ` (${activeFilterCount})` : ''} {filtersOpen ? '▴' : '▾'}
        </button>
      </div>

      <div className="studio-seg" style={{ width: '100%' }}>
        {([1, 2] as const).map((p) => (
          <button
            key={p}
            type="button"
            className={`studio-seg__btn ${targetPlayer === p ? 'is-active' : ''}`}
            style={{ flex: 1 }}
            onClick={() => onTargetPlayerChange(p)}
          >
            Player {p}
          </button>
        ))}
      </div>

      <select
        className="studio-select"
        value={placementTarget}
        onChange={(e) => onPlacementTargetChange(e.target.value as PlacementTarget)}
        aria-label="Placement zone"
      >
        <option value="field_new">Battle Area (New Permanent)</option>
        <option value="field_stack_top">
          Selected Stack — Top {selectedSlotIndex != null ? `(Slot ${selectedSlotIndex})` : '(pick slot)'}
        </option>
        <option value="field_stack_bottom">
          Selected Stack — Source Bottom {selectedSlotIndex != null ? `(Slot ${selectedSlotIndex})` : '(pick slot)'}
        </option>
        <option value="field_link">
          Selected Stack — Linked Card {selectedSlotIndex != null ? `(Slot ${selectedSlotIndex})` : '(pick slot)'}
        </option>
        <option value="breeding">Breeding Stack</option>
        <option value="hand">Hand</option>
        <option value="security">Security Top</option>
        <option value="trash">Trash</option>
        <option value="deck_top">Deck Top</option>
        <option value="egg_deck">Egg Deck Top</option>
      </select>

      <input
        type="search"
        className="studio-input"
        placeholder="Search by name, ID, trait, or card text..."
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        aria-label="Search cards"
      />

      {filtersOpen && (
        <div className="studio-filter-drawer" data-testid="studio-filters-drawer">
          <div className="studio-panel__title">Color</div>
          <div className="studio-pill-group">
            {COLOR_OPTIONS.map(({ code, name }) => (
              <button
                key={code}
                type="button"
                className={`studio-chip ${selectedColors.includes(code) ? 'is-active' : ''}`}
                onClick={() => toggleNum(selectedColors, code, setSelectedColors)}
              >
                <span
                  className="studio-color-dot"
                  style={{ backgroundColor: COLOR_HEX[name] ?? 'var(--ink-2)' }}
                />
                {name}
              </button>
            ))}
          </div>

          <div className="studio-panel__title">Level</div>
          <div className="studio-pill-group">
            {LEVEL_OPTIONS.map((lv) => (
              <button
                key={lv}
                type="button"
                className={`studio-chip ${selectedLevels.includes(lv) ? 'is-active' : ''}`}
                onClick={() => toggleNum(selectedLevels, lv, setSelectedLevels)}
              >
                Lv.{lv}
              </button>
            ))}
          </div>

          <div className="studio-panel__title">Card Type</div>
          <div className="studio-pill-group">
            {Object.entries(CARD_KIND_LABELS).map(([k, label]) => {
              const code = Number(k);
              return (
                <button
                  key={code}
                  type="button"
                  className={`studio-chip ${selectedKinds.includes(code) ? 'is-active' : ''}`}
                  onClick={() => toggleNum(selectedKinds, code, setSelectedKinds)}
                >
                  {label}
                </button>
              );
            })}
          </div>

          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginTop: 4 }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 11, color: 'var(--ink-1)' }}>
              <input
                type="checkbox"
                checked={implementedOnly}
                onChange={(e) => setImplementedOnly(e.target.checked)}
              />
              Implemented only
            </label>
            <button
              type="button"
              className="studio-btn studio-btn--sm"
              onClick={() => {
                setSelectedColors([]);
                setSelectedLevels([]);
                setSelectedKinds([]);
                setImplementedOnly(true);
              }}
            >
              Reset
            </button>
          </div>
        </div>
      )}

      <div style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 4 }}>
        {results.map((c) => {
          const firstColor = c.colors[0];
          const primaryColorName = firstColor !== undefined ? COLOR_NAMES[firstColor] : undefined;
          const colorHex = primaryColorName ? COLOR_HEX[primaryColorName] : 'var(--ink-2)';
          const kindLabel = CARD_KIND_LABELS[c.card_kind] ?? 'Card';
          const snippet = c.effect_text || c.inherited_text || c.security_text;
          return (
            <button
              key={c.card_id}
              type="button"
              className="studio-card-item"
              title={snippet || `${c.name} (${c.card_id})`}
              onClick={() => onPlaceCard(c.card_id)}
            >
              <div className="studio-card-item__top">
                <span className="studio-card-item__name" style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <span className="studio-color-dot" style={{ backgroundColor: colorHex }} />
                  {c.name}
                </span>
                <span style={{ fontFamily: 'var(--font-mono)', fontSize: 10, color: 'var(--accent)' }}>
                  + ADD
                </span>
              </div>
              <div className="studio-card-item__meta">
                <span>{c.card_id}</span>
                <span>· {kindLabel}</span>
                {c.level != null && <span>· Lv.{c.level}</span>}
                {c.dp != null && <span>· {c.dp} DP</span>}
                {c.play_cost != null && <span>· Cost {c.play_cost}</span>}
                {!c.implemented && <span style={{ color: 'var(--warn)' }}>· unimpl</span>}
              </div>
              {snippet && <div className="studio-card-item__snippet">{snippet}</div>}
            </button>
          );
        })}
        {results.length === 0 && (
          <div style={{ padding: 16, textAlign: 'center', fontSize: 12, color: 'var(--ink-3)' }}>
            No matching cards
          </div>
        )}
      </div>
    </div>
  );
}
