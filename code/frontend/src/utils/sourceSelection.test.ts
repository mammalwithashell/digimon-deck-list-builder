import { describe, it, expect } from 'vitest';

import {
  isSourceSelectAction,
  sourceSelectionCards,
  sourceSelectionHosts,
  SOURCE_SELECT_START,
} from './sourceSelection';

describe('isSourceSelectAction', () => {
  it('detects the SOURCE_SELECT range and rejects field/attack ids', () => {
    expect(isSourceSelectAction(SOURCE_SELECT_START)).toBe(true); // 2000
    expect(isSourceSelectAction(2191)).toBe(true);
    expect(isSourceSelectAction(2192)).toBe(false); // SOURCE_SELECT_END
    expect(isSourceSelectAction(100)).toBe(false); // field/attack range
    expect(isSourceSelectAction(1999)).toBe(false);
  });
});

describe('sourceSelectionCards', () => {
  // Own field slot 0: a stack of TOP (active) over two sources A (bottom), B.
  // `sources` keeps the top first; the picker filters it out and indexes the
  // remaining NON-TOP cards bottom→top to match the engine's source_index.
  const battleArea = [
    {
      sources: [
        { cardId: 'TOP', cardName: 'Top', isTop: true },
        { cardId: 'A', cardName: 'Source A', isTop: false },
        { cardId: 'B', cardName: 'Source B', isTop: false },
      ],
    },
  ];

  it('maps SOURCE_SELECT ids to the non-top source at (field, sourceIndex)', () => {
    // field 0 → base 2000; source 0 → 2000, source 1 → 2001.
    expect(sourceSelectionCards([2000, 2001], battleArea)).toEqual([
      { actionId: 2000, cardId: 'A', cardName: 'Source A' },
      { actionId: 2001, cardId: 'B', cardName: 'Source B' },
    ]);
  });

  it('skips ids whose permanent or source index does not resolve', () => {
    // field 3 has no permanent; source index 5 on field 0 has no such source.
    expect(sourceSelectionCards([2000 + 3 * 12, 2000 + 5], battleArea)).toEqual([]);
  });
});

describe('sourceSelectionHosts', () => {
  const twoHostArea = [
    {
      sources: [
        { cardId: 'BT1-025', cardName: 'WarGreymon', isTop: true },
        { cardId: 'BT1-010', cardName: 'Agumon', isTop: false },
      ],
    },
    {
      sources: [
        { cardId: 'BT1-044', cardName: 'MetalGarurumon', isTop: true },
        { cardId: 'BT1-029', cardName: 'Gabumon', isTop: false },
        { cardId: 'BT1-036', cardName: 'Garurumon', isTop: false },
      ],
    },
  ];

  it('groups source tiles by host permanent fieldIndex for two-step picking', () => {
    const groups = sourceSelectionHosts([2000, 2012, 2013], twoHostArea);
    expect(groups).toHaveLength(2);
    expect(groups[0]).toEqual({
      fieldIndex: 0,
      hostCardId: 'BT1-025',
      hostCardName: 'WarGreymon',
      tiles: [{ actionId: 2000, cardId: 'BT1-010', cardName: 'Agumon' }],
    });
    expect(groups[1]).toEqual({
      fieldIndex: 1,
      hostCardId: 'BT1-044',
      hostCardName: 'MetalGarurumon',
      tiles: [
        { actionId: 2012, cardId: 'BT1-029', cardName: 'Gabumon' },
        { actionId: 2013, cardId: 'BT1-036', cardName: 'Garurumon' },
      ],
    });
  });
});
