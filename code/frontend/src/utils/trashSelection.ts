import { SELECTION } from '@/utils/constants';
import type { PendingSelection } from '@/types/game';

/**
 * Trash-selection helpers for `TrashSelectModal`.
 *
 * The engine encodes both single-select (`SelectionKind::Trash`) and
 * capped multi-select (`SelectionKind::CountCappedMultiSelect`) trash picks
 * with action ids in the trash range (`SELECTION.TRASH_START..TRASH_END`,
 * mirroring the Rust engine's `TRASH_EFFECT_START..END`). A trash card at
 * zone index `i` maps to action `SELECTION.TRASH_START + i`.
 *
 * `min`/`max`/`picked`/`distinct` for a multi-select ride the serialized
 * `kind` string (`format!("{:?}", kind)` on both wires), so we parse them
 * out here rather than depending on extra DTO fields.
 */

export interface CountCappedKind {
  min: number;
  max: number;
  picked: number;
  distinct: boolean;
}

/** True iff `id` falls in the engine's trash selection action range. */
export function isTrashAction(id: number): boolean {
  return id >= SELECTION.TRASH_START && id <= SELECTION.TRASH_END;
}

/**
 * Parse a `CountCappedMultiSelect { min: N, max: N, picked: N, distinct: bool }`
 * kind string into its fields. Tolerant of field order and whitespace.
 * Returns null for any other kind (or a malformed count-capped string missing
 * a required numeric field).
 */
export function parseCountCappedKind(kind: string | undefined | null): CountCappedKind | null {
  if (!kind || !kind.startsWith('CountCappedMultiSelect')) return null;
  const num = (field: string): number | null => {
    const m = kind.match(new RegExp(`${field}\\s*:\\s*(\\d+)`));
    return m ? Number(m[1]) : null;
  };
  const min = num('min');
  const max = num('max');
  const picked = num('picked');
  if (min === null || max === null || picked === null) return null;
  const distinct = /distinct\s*:\s*true/.test(kind);
  return { min, max, picked, distinct };
}

/**
 * Classify a pending selection as a trash pick the `TrashSelectModal` owns.
 * - `'single'`: `SelectionKind::Trash`.
 * - `'multi'`:  `CountCappedMultiSelect` whose every valid action id is in the
 *   trash range (distinguishes a trash multi-select from a hand/material one,
 *   since the kind itself doesn't name the zone).
 * - `null`: not a trash selection.
 */
export function trashSelectionMode(
  pending: PendingSelection | null | undefined,
): 'single' | 'multi' | null {
  if (!pending) return null;
  if (pending.kind === 'Trash') return 'single';
  if (
    pending.kind?.startsWith('CountCappedMultiSelect') &&
    pending.validIndices.length > 0 &&
    pending.validIndices.every(isTrashAction)
  ) {
    return 'multi';
  }
  return null;
}
