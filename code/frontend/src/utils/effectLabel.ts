import type { DecodedAction } from '@/types/game';

/**
 * Decoded-action kinds that represent a player-activatable [Main] effect the
 * action bar surfaces: field [Main] (incl. Digiburst, breeding `<Training>`,
 * delayed-Option [Main]), hand [Main], and trash [Main]. All are decoded by
 * the engine with correct zone + card identity, so the action bar never
 * re-derives them from raw mask bit-ranges.
 */
export const MAIN_EFFECT_KINDS: ReadonlySet<DecodedAction['kind']> = new Set([
  'field_effect',
  'hand_effect',
  'trash_effect',
]);

export function isActivatableEffect(action: DecodedAction): boolean {
  return MAIN_EFFECT_KINDS.has(action.kind);
}

const MAX_TAG_LEN = 28;

/**
 * Derive a short, human-readable effect tag from an authored effect name (the
 * DSL `summary`, e.g. `"[Main] Digi-Burst 2: play 1 purple level 3 Digimon
 * from trash free"`). Strips a leading run of bracketed/angle timing tags,
 * keeps the headline segment before the first `":"`, and truncates. Returns
 * `null` when nothing meaningful remains (caller falls back to the card name).
 *
 * Examples:
 *   "[Main] Digi-Burst 2: play 1 purple…"  -> "Digi-Burst 2"
 *   "[On Play][When Digivolving] Draw 1"   -> "Draw 1"
 *   "<Blocker>"                             -> null
 */
export function shortEffectTag(
  effectName: string | null | undefined,
): string | null {
  if (!effectName) return null;
  let s = effectName.trim();
  // Strip a leading run of "[...]" / "<...>" tag tokens and surrounding space.
  let prev: string;
  do {
    prev = s;
    s = s.replace(/^\s*(?:\[[^\]]*\]|<[^>]*>)\s*/, '');
  } while (s !== prev);
  // Keep the headline segment before the first ":".
  const colon = s.indexOf(':');
  if (colon >= 0) s = s.slice(0, colon);
  s = s.trim();
  if (!s) return null;
  if (s.length > MAX_TAG_LEN) {
    s = `${s.slice(0, MAX_TAG_LEN - 1).trimEnd()}…`;
  }
  return s;
}

/**
 * Compose the action-bar button label for an activatable effect:
 * `"{card}: {short tag}"`, or just `"{card}"` when there is no usable effect
 * tag. The board/zone slot is appended (`"{card} (slot N): …"`) only when
 * `disambiguateSlot` is set — i.e. two or more surfaced entries share a card
 * name (per the "humans don't care about slot unless there's a duplicate"
 * rule).
 */
export function activatableEffectLabel(
  action: DecodedAction,
  disambiguateSlot: boolean,
): string {
  const card = action.cardName ?? 'Effect';
  const slotSuffix =
    disambiguateSlot && action.sourceIndex != null
      ? ` (slot ${action.sourceIndex})`
      : '';
  const tag = shortEffectTag(action.effectName);
  return tag ? `${card}${slotSuffix}: ${tag}` : `${card}${slotSuffix}`;
}

/** The hover tooltip for an activatable effect: the full authored effect
 *  summary, falling back to the engine's decoded label. */
export function effectTooltip(action: DecodedAction): string {
  return action.effectName ?? action.label;
}

/**
 * Count cardName occurrences across the surfaced activatable entries, so the
 * caller can decide which labels need a slot disambiguator. Entries with no
 * card name are keyed under the empty string.
 */
export function effectNameCounts(
  actions: readonly DecodedAction[],
): Map<string, number> {
  const counts = new Map<string, number>();
  for (const a of actions) {
    const n = a.cardName ?? '';
    counts.set(n, (counts.get(n) ?? 0) + 1);
  }
  return counts;
}
