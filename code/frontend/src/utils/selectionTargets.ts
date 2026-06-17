import { ACTION, ATTACK_TARGETS_PER_SLOT, FIELD_SLOTS, SELECTION } from './constants';

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

/**
 * `AnyField` — a selection whose candidates span BOTH battle areas
 * (`select_any_permanent` / `select_dna_pair`, e.g. EX8-028 Skadimon "place 1
 * Digimon with no digivolution cards as the bottom security card").
 *
 * Unlike `OwnField`/`OppField`, an `AnyField` action id carries the ABSOLUTE
 * engine player in the id itself: `encode_attack(player, index)` =
 * `ATTACK_START + player * ATTACK_TARGETS_PER_SLOT + index`. So which side a
 * candidate is on is recovered by DECODING the id, not from a single `kind`.
 * Routing these as `Target` (the old behavior) left the board with no handler —
 * `isFieldSelectionKind('Target')` is false — so every target was unclickable
 * AND, since the inner pick is mandatory, no Decline showed either: the softlock.
 *
 * `selectingPlayer` is the UI 1/2 number of the player making the pick (the
 * frontend's `pendingSelection.selectingPlayer`); the selecting player's engine
 * id is `selectingPlayer - 1`, which is what the action id encodes for that
 * player's own field. This is perspective-robust (works regardless of seat),
 * unlike keying off the hardcoded `localPlayer`.
 */
export function isAnyFieldSelectionKind(kind: string | undefined): boolean {
  return kind === 'AnyField';
}

/** Decode an `encode_attack(player, index)` field action id into its engine
 *  player + battle-area slot, or `null` if it isn't a field-range id. */
function decodeFieldActionId(
  id: number,
): { enginePlayer: number; slot: number } | null {
  const span = 2 * ATTACK_TARGETS_PER_SLOT;
  if (id < ACTION.ATTACK_START || id >= ACTION.ATTACK_START + span) return null;
  const offset = id - ACTION.ATTACK_START;
  return {
    enginePlayer: Math.floor(offset / ATTACK_TARGETS_PER_SLOT),
    slot: offset % ATTACK_TARGETS_PER_SLOT,
  };
}

/** Engine id of the player making an `AnyField` pick (their own field). */
function ownEngineId(selectingPlayer: number | undefined): number {
  return (selectingPlayer ?? 1) - 1;
}

/**
 * Map a board click on `slotIndex` of the `isOpponent` side during an
 * `AnyField` selection to the matching engine action id, or `null`.
 */
export function anyFieldSelectionActionId(
  kind: string | undefined,
  isOpponent: boolean,
  slotIndex: number,
  validSelections: ReadonlySet<number>,
  selectingPlayer: number | undefined,
): number | null {
  if (!isAnyFieldSelectionKind(kind)) return null;
  const own = ownEngineId(selectingPlayer);
  const wantPlayer = isOpponent ? 1 - own : own;
  const actionId =
    ACTION.ATTACK_START + wantPlayer * ATTACK_TARGETS_PER_SLOT + slotIndex;
  return validSelections.has(actionId) ? actionId : null;
}

/**
 * Split an `AnyField` selection's valid ids into own- vs enemy-side slot
 * indices for highlighting. Returns empty sets for non-`AnyField` kinds.
 */
export function anyFieldSelectionHighlights(
  kind: string | undefined,
  validSelections: Iterable<number>,
  selectingPlayer: number | undefined,
): { own: Set<number>; enemy: Set<number> } {
  const own = new Set<number>();
  const enemy = new Set<number>();
  if (!isAnyFieldSelectionKind(kind)) return { own, enemy };
  const ownEngine = ownEngineId(selectingPlayer);
  for (const idx of validSelections) {
    const decoded = decodeFieldActionId(idx);
    if (!decoded || decoded.slot >= FIELD_SLOTS) continue;
    (decoded.enginePlayer === ownEngine ? own : enemy).add(decoded.slot);
  }
  return { own, enemy };
}
