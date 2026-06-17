import { renderHook } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { useActionMask } from './useActionMask';
import { ACTION, ATTACK_TARGETS_PER_SLOT, SELECTION } from '@/utils/constants';

/** Build a zeroed action mask big enough to cover every range the hook scans. */
function emptyMask(): number[] {
  return new Array(ACTION.BREEDING_SOURCE_END + 1).fill(0);
}

describe('useActionMask — AnyField (mixed-side) field ranges', () => {
  // `AnyField` prompts (select_any_permanent) encode opponent slots as
  // `OWN_FIELD_START + ATTACK_TARGETS_PER_SLOT + slot` (115-128). Opponent slot
  // 13 is id 128, which the legacy `ENEMY_FIELD_END` (127) range dropped —
  // making the 14th opposing Digimon unclickable in an AnyField pick. This pins
  // that the full 115-128 opponent range is collected into validSelections.
  it('collects opponent AnyField slot 13 (action id 128), not just up to 127', () => {
    const mask = emptyMask();
    const ownSlot0 = SELECTION.OWN_FIELD_START; // 100
    const oppSlot0 = SELECTION.OWN_FIELD_START + ATTACK_TARGETS_PER_SLOT; // 115
    const oppSlot13 = SELECTION.OWN_FIELD_START + ATTACK_TARGETS_PER_SLOT + 13; // 128
    mask[ownSlot0] = 1;
    mask[oppSlot0] = 1;
    mask[oppSlot13] = 1;

    const { result } = renderHook(() => useActionMask(mask));
    const valid = result.current.validSelections;

    expect(valid.has(ownSlot0)).toBe(true);
    expect(valid.has(oppSlot0)).toBe(true);
    expect(valid.has(oppSlot13)).toBe(true); // the previously-dropped 14th slot
  });
});
