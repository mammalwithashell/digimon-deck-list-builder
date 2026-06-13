import { describe, it, expect } from 'vitest';
import { isTrashAction, parseCountCappedKind, trashSelectionMode } from './trashSelection';
import { SELECTION } from './constants';
import type { PendingSelection } from '@/types/game';

describe('isTrashAction', () => {
  it('accepts the engine trash range and rejects outside it', () => {
    expect(isTrashAction(SELECTION.TRASH_START)).toBe(true);
    expect(isTrashAction(SELECTION.TRASH_END)).toBe(true);
    expect(isTrashAction(SELECTION.TRASH_START - 1)).toBe(false);
    expect(isTrashAction(SELECTION.TRASH_END + 1)).toBe(false);
    expect(isTrashAction(0)).toBe(false);
  });
});

describe('parseCountCappedKind', () => {
  it('parses min/max/picked/distinct (distinct false)', () => {
    expect(
      parseCountCappedKind('CountCappedMultiSelect { min: 1, max: 3, picked: 0, distinct: false }'),
    ).toEqual({ min: 1, max: 3, picked: 0, distinct: false });
  });

  it('parses distinct true', () => {
    expect(
      parseCountCappedKind('CountCappedMultiSelect { min: 2, max: 2, picked: 1, distinct: true }'),
    ).toEqual({ min: 2, max: 2, picked: 1, distinct: true });
  });

  it('is tolerant of field order and spacing', () => {
    expect(
      parseCountCappedKind('CountCappedMultiSelect {distinct:true,picked:2,max:4,min:0}'),
    ).toEqual({ min: 0, max: 4, picked: 2, distinct: true });
  });

  it('returns null for non-count-capped kinds', () => {
    expect(parseCountCappedKind('Trash')).toBeNull();
    expect(parseCountCappedKind('OwnField')).toBeNull();
    expect(parseCountCappedKind(undefined)).toBeNull();
    expect(parseCountCappedKind(null)).toBeNull();
  });

  it('returns null when a required numeric field is missing', () => {
    expect(parseCountCappedKind('CountCappedMultiSelect { max: 3, picked: 0 }')).toBeNull();
  });
});

function pending(partial: Partial<PendingSelection>): PendingSelection {
  return {
    phase: 0,
    validIndices: [],
    isOptional: false,
    prompt: '',
    selectingPlayer: 1,
    ...partial,
  } as PendingSelection;
}

describe('trashSelectionMode', () => {
  it('returns single for kind Trash', () => {
    expect(trashSelectionMode(pending({ kind: 'Trash', validIndices: [SELECTION.TRASH_START] }))).toBe('single');
  });

  it('returns multi for count-capped with all trash-range ids', () => {
    expect(
      trashSelectionMode(
        pending({
          kind: 'CountCappedMultiSelect { min: 1, max: 3, picked: 0, distinct: false }',
          validIndices: [SELECTION.TRASH_START, SELECTION.TRASH_START + 1],
        }),
      ),
    ).toBe('multi');
  });

  it('returns null for a count-capped select that is NOT in the trash range', () => {
    expect(
      trashSelectionMode(
        pending({
          kind: 'CountCappedMultiSelect { min: 1, max: 2, picked: 0, distinct: false }',
          validIndices: [0, 1], // hand/material range
        }),
      ),
    ).toBeNull();
  });

  it('returns null for no pending or unrelated kinds', () => {
    expect(trashSelectionMode(null)).toBeNull();
    expect(trashSelectionMode(pending({ kind: 'Hand', validIndices: [0] }))).toBeNull();
  });
});
