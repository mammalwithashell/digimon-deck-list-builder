import { SELECTION } from './constants';

/**
 * Field-target selection handling.
 *
 * The Rust engine (`effect_context/selections.rs::install_field_selection`)
 * encodes EVERY field-target selection — own field AND opponent field —
 * identically as `encode_attack(0, slot)` = `OWN_FIELD_START + slot`
 * (=100+slot). Which player's field a `100+slot` id refers to is carried
 * ONLY in `pendingSelection.kind` ("OwnField" | "OppField"). There is no
 * separate enemy-field id range on the wire.
 *
 * The frontend previously assumed a `114+slot` enemy range (constants
 * `ENEMY_FIELD_START`), so opponent-field clicks computed ids that never
 * matched the engine's valid set → the click was swallowed and the
 * opponent's permanents were never highlighted. These helpers route field
 * selections off `kind` instead, which is the engine's source of truth.
 */

export type FieldSelectionKind = 'OwnField' | 'OppField';

export function isFieldSelectionKind(
  kind: string | undefined | null,
): kind is FieldSelectionKind {
  return kind === 'OwnField' || kind === 'OppField';
}

function asSet(validIndices: ReadonlySet<number> | readonly number[]): ReadonlySet<number> {
  return validIndices instanceof Set ? validIndices : new Set(validIndices);
}

/**
 * Resolve a board-slot click to the engine action id for a field-target
 * selection, or `null` when the click does not satisfy the active selection.
 *
 * Both field kinds encode as `OWN_FIELD_START + slot`; the click is only
 * valid when the clicked field (`isOpponentClick`) matches the selection's
 * `kind` AND that id is in the engine's valid set.
 */
export function fieldSelectionActionId(
  kind: string | undefined | null,
  isOpponentClick: boolean,
  slotIndex: number,
  validIndices: ReadonlySet<number> | readonly number[],
): number | null {
  if (!isFieldSelectionKind(kind)) return null;
  const wantOpponent = kind === 'OppField';
  if (isOpponentClick !== wantOpponent) return null;
  const actionId = SELECTION.OWN_FIELD_START + slotIndex;
  return asSet(validIndices).has(actionId) ? actionId : null;
}

/**
 * Split a selection's valid field ids into own/enemy highlight slot sets,
 * routed by `kind`. Engine ids are `100+slot` for either field, so the slot
 * index is `id - OWN_FIELD_START` and the destination set is chosen by kind.
 * Ids outside the field range are ignored.
 */
export function fieldSelectionHighlights(
  kind: string | undefined | null,
  validIndices: readonly number[],
): { own: Set<number>; enemy: Set<number> } {
  const own = new Set<number>();
  const enemy = new Set<number>();
  if (!isFieldSelectionKind(kind)) return { own, enemy };
  const target = kind === 'OppField' ? enemy : own;
  for (const idx of validIndices) {
    if (idx >= SELECTION.OWN_FIELD_START && idx <= SELECTION.OWN_FIELD_END) {
      target.add(idx - SELECTION.OWN_FIELD_START);
    }
  }
  return { own, enemy };
}
