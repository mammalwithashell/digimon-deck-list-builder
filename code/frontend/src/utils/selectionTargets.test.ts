import { describe, it, expect } from 'vitest';

import {
  anyFieldSelectionActionId,
  anyFieldSelectionHighlights,
  fieldSelectionActionId,
  fieldSelectionHighlights,
  isAnyFieldSelectionKind,
  isFieldSelectionKind,
} from './selectionTargets';
import { ACTION, ATTACK_TARGETS_PER_SLOT, SELECTION } from './constants';

/** Engine `encode_attack(player, index)` — how AnyField ids are wired. */
const anyId = (enginePlayer: number, slot: number) =>
  ACTION.ATTACK_START + enginePlayer * ATTACK_TARGETS_PER_SLOT + slot;

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

// `AnyField` selections (`select_any_permanent` / `select_dna_pair`, e.g.
// EX8-028 Skadimon "place 1 Digimon with no digivolution cards as the bottom
// security card") span BOTH battle areas. Unlike OwnField/OppField, each valid
// id carries the ABSOLUTE engine player: `encode_attack(player, index)`. The
// own engine id is `selectingPlayer - 1`, so the decode is perspective-robust.
// Routing these as `Target` left every target unclickable AND uncancellable
// (the inner pick is mandatory) — the softlock these tests pin against.
describe('isAnyFieldSelectionKind', () => {
  it('accepts AnyField and rejects everything else', () => {
    expect(isAnyFieldSelectionKind('AnyField')).toBe(true);
    expect(isAnyFieldSelectionKind('OppField')).toBe(false);
    expect(isAnyFieldSelectionKind('Target')).toBe(false);
    expect(isAnyFieldSelectionKind(undefined)).toBe(false);
  });
});

describe('anyFieldSelectionHighlights', () => {
  it('splits both-field ids into own/enemy by decoding the engine player (local = P1)', () => {
    // selectingPlayer 1 → own engine id 0. Own slots 0,1 (ids 100,101);
    // enemy slots 1,3 (ids 116,118 = 115 + slot).
    const { own, enemy } = anyFieldSelectionHighlights(
      'AnyField',
      [anyId(0, 0), anyId(0, 1), anyId(1, 1), anyId(1, 3)],
      1,
    );
    expect([...own].sort((a, b) => a - b)).toEqual([0, 1]);
    expect([...enemy].sort((a, b) => a - b)).toEqual([1, 3]);
  });

  it('is perspective-robust — when the selecting player is P2, side mapping flips', () => {
    // selectingPlayer 2 → own engine id 1. Now engine-player-1 ids are OWN.
    const { own, enemy } = anyFieldSelectionHighlights(
      'AnyField',
      [anyId(0, 0), anyId(1, 2)],
      2,
    );
    expect([...own]).toEqual([2]); // engine player 1 slot 2 is "own"
    expect([...enemy]).toEqual([0]); // engine player 0 slot 0 is the opponent
  });

  it('decodes the opponent 14th slot (slot 13 → id 128), the previously-dropped edge', () => {
    const { own, enemy } = anyFieldSelectionHighlights(
      'AnyField',
      [anyId(1, 13)], // 100 + 15 + 13 = 128
      1,
    );
    expect(own.size).toBe(0);
    expect([...enemy]).toEqual([13]);
  });

  it('returns empty sets for non-AnyField kinds', () => {
    const { own, enemy } = anyFieldSelectionHighlights('OppField', [anyId(0, 0)], 1);
    expect(own.size).toBe(0);
    expect(enemy.size).toBe(0);
  });
});

describe('anyFieldSelectionActionId', () => {
  const valid = new Set([anyId(0, 1), anyId(1, 3)]); // own slot 1, enemy slot 3 (P1 view)

  it('maps an own-side click to the engine-player-encoded id', () => {
    expect(anyFieldSelectionActionId('AnyField', false, 1, valid, 1)).toBe(anyId(0, 1));
  });

  it('maps an opponent-side click to the opponent engine-player-encoded id', () => {
    expect(anyFieldSelectionActionId('AnyField', true, 3, valid, 1)).toBe(anyId(1, 3));
  });

  it('returns null for a slot that is not a valid target', () => {
    expect(anyFieldSelectionActionId('AnyField', false, 2, valid, 1)).toBeNull();
    expect(anyFieldSelectionActionId('AnyField', true, 0, valid, 1)).toBeNull();
  });

  it('returns null for non-AnyField kinds', () => {
    expect(anyFieldSelectionActionId('OppField', true, 3, valid, 1)).toBeNull();
    expect(anyFieldSelectionActionId(undefined, true, 3, valid, 1)).toBeNull();
  });
});
