import { describe, expect, it } from 'vitest';
import {
  fieldSelectionActionId,
  fieldSelectionHighlights,
  isFieldSelectionKind,
} from './selectionTargets';
import { SELECTION } from './constants';

// The engine encodes BOTH own-field and opponent-field target selections as
// `OWN_FIELD_START + slot` (=100+slot). Which field a 100+slot id refers to is
// carried ONLY in `pendingSelection.kind` ("OwnField" | "OppField"). There is
// NO separate 114+slot enemy range on the wire — that frontend assumption was
// the bug (opponent clicks computed 114+slot and never matched the valid set).

describe('isFieldSelectionKind', () => {
  it('recognizes the two field kinds', () => {
    expect(isFieldSelectionKind('OwnField')).toBe(true);
    expect(isFieldSelectionKind('OppField')).toBe(true);
  });
  it('rejects non-field kinds and undefined', () => {
    expect(isFieldSelectionKind('Hand')).toBe(false);
    expect(isFieldSelectionKind('Security')).toBe(false);
    expect(isFieldSelectionKind(undefined)).toBe(false);
  });
});

describe('fieldSelectionActionId', () => {
  const valid = new Set<number>([
    SELECTION.OWN_FIELD_START + 0,
    SELECTION.OWN_FIELD_START + 2,
  ]);

  it('OppField: opponent click on a valid slot returns 100+slot', () => {
    expect(fieldSelectionActionId('OppField', true, 0, valid)).toBe(
      SELECTION.OWN_FIELD_START + 0,
    );
    expect(fieldSelectionActionId('OppField', true, 2, valid)).toBe(
      SELECTION.OWN_FIELD_START + 2,
    );
  });

  it('OppField: opponent click on a slot NOT in the valid set returns null', () => {
    expect(fieldSelectionActionId('OppField', true, 1, valid)).toBeNull();
  });

  it('OppField: clicking your OWN field returns null (wrong field)', () => {
    expect(fieldSelectionActionId('OppField', false, 0, valid)).toBeNull();
  });

  it('OwnField: own click on a valid slot returns 100+slot', () => {
    expect(fieldSelectionActionId('OwnField', false, 2, valid)).toBe(
      SELECTION.OWN_FIELD_START + 2,
    );
  });

  it('OwnField: clicking the opponent field returns null (wrong field)', () => {
    expect(fieldSelectionActionId('OwnField', true, 0, valid)).toBeNull();
  });

  it('non-field kind returns null regardless of click', () => {
    expect(fieldSelectionActionId('Hand', true, 0, valid)).toBeNull();
    expect(fieldSelectionActionId(undefined, false, 0, valid)).toBeNull();
  });

  it('accepts a plain number[] for validIndices', () => {
    expect(
      fieldSelectionActionId('OppField', true, 2, [
        SELECTION.OWN_FIELD_START + 2,
      ]),
    ).toBe(SELECTION.OWN_FIELD_START + 2);
  });
});

describe('fieldSelectionHighlights', () => {
  const ids = [SELECTION.OWN_FIELD_START + 0, SELECTION.OWN_FIELD_START + 3];

  it('OppField highlights the ENEMY slots, leaves own empty', () => {
    const { own, enemy } = fieldSelectionHighlights('OppField', ids);
    expect([...enemy].sort((a, b) => a - b)).toEqual([0, 3]);
    expect(own.size).toBe(0);
  });

  it('OwnField highlights the OWN slots, leaves enemy empty', () => {
    const { own, enemy } = fieldSelectionHighlights('OwnField', ids);
    expect([...own].sort((a, b) => a - b)).toEqual([0, 3]);
    expect(enemy.size).toBe(0);
  });

  it('non-field kind yields no highlights', () => {
    const { own, enemy } = fieldSelectionHighlights('Hand', ids);
    expect(own.size).toBe(0);
    expect(enemy.size).toBe(0);
  });

  it('ignores ids outside the field range', () => {
    const { own, enemy } = fieldSelectionHighlights('OppField', [
      SELECTION.HAND_START + 1,
      SELECTION.OWN_FIELD_START + 5,
      SELECTION.TRASH_START + 1,
    ]);
    expect([...enemy]).toEqual([5]);
    expect(own.size).toBe(0);
  });
});
