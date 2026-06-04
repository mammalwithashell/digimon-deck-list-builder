import { SELECTION } from './constants';

/**
 * Field-target selection helpers.
 *
 * The Rust engine encodes EVERY field-target selection — the controller's own
 * field OR the opponent's field — identically as `OWN_FIELD_START + slot`
 * (`encode_attack(0, slot)` in `effect_context/selections.rs::
 * install_field_selection`). Which player's field a `100 + slot` action id
 * refers to is carried *only* in `pendingSelection.kind` (`OwnField` /
 * `OppField`), never in a separate id range. There is no `ENEMY_FIELD` id
 * range on the wire — `SELECTION.ENEMY_FIELD_*` is a UI-side fiction.
 *
 * These helpers consume `kind` so the board can (a) highlight the correct
 * side and (b) route a board click to the right action id. Without them,
 * "delete an opponent's Digimon"-style prompts render no affordance and
 * swallow clicks (the opponent click computed `114 + slot`, which never
 * matched the engine's `100 + slot` valid set).
 */

/** The two `SelectionKind`s that address a battle-area slot via `100 + slot`. */
export type FieldSelectionKind = 'OwnField' | 'OppField';

export function isFieldSelectionKind(
  kind: string | undefined,
): kind is FieldSelectionKind {
  return kind === 'OwnField' || kind === 'OppField';
}

/**
 * Given a board click on `slotIndex` of the `isOpponent` side during a field
 * selection of `kind`, return the engine action id to dispatch, or `null` if
 * the click is on the wrong side or the slot is not a valid target.
 */
export function fieldSelectionActionId(
  kind: string | undefined,
  isOpponent: boolean,
  slotIndex: number,
  validSelections: ReadonlySet<number>,
): number | null {
  if (!isFieldSelectionKind(kind)) return null;
  const sideMatches = kind === 'OppField' ? isOpponent : !isOpponent;
  if (!sideMatches) return null;
  const actionId = SELECTION.OWN_FIELD_START + slotIndex;
  return validSelections.has(actionId) ? actionId : null;
}

/**
 * Split a field selection's valid action ids into own- vs enemy-side slot
 * indices for highlighting, based on `kind`. Returns empty sets for
 * non-field selection kinds.
 */
export function fieldSelectionHighlights(
  kind: string | undefined,
  validSelections: Iterable<number>,
): { own: Set<number>; enemy: Set<number> } {
  const own = new Set<number>();
  const enemy = new Set<number>();
  if (!isFieldSelectionKind(kind)) return { own, enemy };
  const target = kind === 'OppField' ? enemy : own;
  for (const idx of validSelections) {
    if (idx >= SELECTION.OWN_FIELD_START && idx <= SELECTION.OWN_FIELD_END) {
      target.add(idx - SELECTION.OWN_FIELD_START);
    }
  }
  return { own, enemy };
}
