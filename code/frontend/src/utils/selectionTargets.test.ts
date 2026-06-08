import { describe, it, expect } from 'vitest';

import {
  fieldSelectionActionId,
  fieldSelectionHighlights,
  isFieldSelectionKind,
} from './selectionTargets';
import { SELECTION } from './constants';

// The engine encodes EVERY field-target selection — own field or opponent
// field — as `OWN_FIELD_START + slot` (== `encode_attack(0, slot)` in
// `selections.rs::install_field_selection`). Which player's field it refers
// to is carried solely in `pendingSelection.kind` (`OwnField` / `OppField`),
// NOT in a separate ID range. These tests pin that contract so the UI can
// never regress back to the bogus `ENEMY_FIELD_START (114)` assumption that
// made "delete an opponent's Digimon" prompts unclickable.

// `isFieldSelectionKind` is the gate GamePage's board-click handler uses to
// decide whether a slot click is a field-target pick. It MUST key off the kind
// alone — NOT the selection phase — because both single-target field prompts
// (phase `SelectTarget`) AND capped-multi-select field prompts (phase
// `SelectBudgeted`, e.g. "delete up to 2 of your opponent's Digimon") surface
// as `OppField` / `OwnField`. A prior phase-range gate excluded `SelectBudgeted`
// and left those multi-select prompts unclickable. These cases pin the contract.
describe('isFieldSelectionKind', () => {
  it('accepts OppField and OwnField (single- AND multi-target field prompts)', () => {
    expect(isFieldSelectionKind('OppField')).toBe(true);
    expect(isFieldSelectionKind('OwnField')).toBe(true);
  });

  it('rejects non-field kinds and undefined so their clicks are not mis-routed', () => {
    // The bespoke multi-select tag must never reach the board router again —
    // capped-multi-select field prompts now arrive tagged OppField/OwnField.
    expect(isFieldSelectionKind('CountCappedMultiSelect')).toBe(false);
    expect(isFieldSelectionKind('Material')).toBe(false);
    expect(isFieldSelectionKind('Hand')).toBe(false);
    expect(isFieldSelectionKind(undefined)).toBe(false);
  });
});

describe('fieldSelectionActionId', () => {
  // Opponent field slots 0, 1, 2, 4 are valid targets — encoded by the engine
  // as OWN_FIELD_START + slot.
  const oppValid = new Set([
    SELECTION.OWN_FIELD_START + 0,
    SELECTION.OWN_FIELD_START + 1,
    SELECTION.OWN_FIELD_START + 2,
    SELECTION.OWN_FIELD_START + 4,
  ]);

  it('maps an opponent-side click to OWN_FIELD_START+slot for an OppField selection', () => {
    expect(fieldSelectionActionId('OppField', true, 2, oppValid)).toBe(
      SELECTION.OWN_FIELD_START + 2,
    );
  });

  it('returns null when the click is on the own side during an OppField selection', () => {
    expect(fieldSelectionActionId('OppField', false, 2, oppValid)).toBeNull();
  });

  it('returns null for an opponent slot that is not a valid target', () => {
    // slot 3 → 103, which is not in the valid set
    expect(fieldSelectionActionId('OppField', true, 3, oppValid)).toBeNull();
  });

  it('maps an own-side click to OWN_FIELD_START+slot for an OwnField selection', () => {
    const ownValid = new Set([SELECTION.OWN_FIELD_START + 0, SELECTION.OWN_FIELD_START + 3]);
    expect(fieldSelectionActionId('OwnField', false, 3, ownValid)).toBe(
      SELECTION.OWN_FIELD_START + 3,
    );
    expect(fieldSelectionActionId('OwnField', true, 3, ownValid)).toBeNull();
  });

  it('returns null for non-field selection kinds and undefined kind', () => {
    const set = new Set([SELECTION.OWN_FIELD_START]);
    expect(fieldSelectionActionId('Hand', true, 0, set)).toBeNull();
    expect(fieldSelectionActionId('Trash', false, 0, set)).toBeNull();
    expect(fieldSelectionActionId(undefined, true, 0, set)).toBeNull();
  });
});

describe('fieldSelectionHighlights', () => {
  it('routes OppField valid indices to the enemy side, leaving own empty', () => {
    const { own, enemy } = fieldSelectionHighlights('OppField', [
      SELECTION.OWN_FIELD_START + 0,
      SELECTION.OWN_FIELD_START + 1,
      SELECTION.OWN_FIELD_START + 2,
      SELECTION.OWN_FIELD_START + 4,
    ]);
    expect([...enemy].sort((a, b) => a - b)).toEqual([0, 1, 2, 4]);
    expect(own.size).toBe(0);
  });

  it('routes OwnField valid indices to the own side, leaving enemy empty', () => {
    const { own, enemy } = fieldSelectionHighlights('OwnField', [
      SELECTION.OWN_FIELD_START + 0,
      SELECTION.OWN_FIELD_START + 5,
    ]);
    expect([...own].sort((a, b) => a - b)).toEqual([0, 5]);
    expect(enemy.size).toBe(0);
  });

  it('returns empty sets for non-field selection kinds', () => {
    const { own, enemy } = fieldSelectionHighlights('Trash', [
      SELECTION.OWN_FIELD_START,
      SELECTION.OWN_FIELD_START + 1,
    ]);
    expect(own.size).toBe(0);
    expect(enemy.size).toBe(0);
  });
});
