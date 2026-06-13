import type { PermanentModifier } from '@/types/game';
import {
  MODIFIER_DISPLAY,
  MODIFIER_GROUP_COLORS,
  MODIFIER_GROUP_ORDER,
  type ModifierGroup,
} from './constants';

/** Modifier types whose signed `value` should be appended to the label. */
const STAT_CHANGE_WITH_VALUE = new Set([
  'ChangeDp',
  'ChangeBaseDp',
  'DpFloor',
  'SecurityAttackChange',
  'ChangeLevel',
]);

const DP_TYPES = new Set(['ChangeDp', 'ChangeBaseDp', 'DpFloor']);

/** Human-readable hint for non-permanent expiries; null = no hint shown. */
const EXPIRY_HINTS: Record<string, string> = {
  EndOfTurn: 'until end of turn',
  EndOfOpponentsTurn: "until end of opponent's turn",
  EndOfYourTurn: 'until end of your turn',
  EndOfOpponentsNextTurn: "until end of opponent's next turn",
  EndOfYourNextTurn: 'until end of your next turn',
  EndOfAttack: 'until end of attack',
  EndOfBattle: 'until end of battle',
  UntilLeaveField: 'while in play',
};

export interface FormattedModifier {
  group: ModifierGroup;
  color: string;
  label: string;
  expiryHint: string | null;
}

/** Split a PascalCase type into spaced words as a last-resort label. */
function humanize(type: string): string {
  return type.replace(/([a-z0-9])([A-Z])/g, '$1 $2');
}

/** Resolve a structured modifier into a display label + group + color.
 * Unknown types fall back to the "Other" group with a humanized label so the
 * panel never breaks on an unmapped modifier. */
export function formatModifier(mod: PermanentModifier): FormattedModifier {
  const entry = MODIFIER_DISPLAY[mod.type];
  const group: ModifierGroup = entry?.group ?? 'Other';
  let label = entry?.label ?? humanize(mod.type);

  if (STAT_CHANGE_WITH_VALUE.has(mod.type) && mod.value !== 0) {
    const sign = mod.value > 0 ? '+' : '';
    const magnitude = DP_TYPES.has(mod.type)
      ? mod.value.toLocaleString()
      : String(mod.value);
    label = `${label} ${sign}${magnitude}`;
  }

  const expiryHint =
    mod.expiry && mod.expiry !== 'Permanent'
      ? EXPIRY_HINTS[mod.expiry] ?? null
      : null;

  return { group, color: MODIFIER_GROUP_COLORS[group], label, expiryHint };
}

/** Group + order formatted modifiers for display. Returns groups in
 * `MODIFIER_GROUP_ORDER`, each with its non-empty list of formatted entries. */
export function groupModifiers(
  modifiers: PermanentModifier[],
): { group: ModifierGroup; color: string; items: FormattedModifier[] }[] {
  const formatted = modifiers.map(formatModifier);
  return MODIFIER_GROUP_ORDER.map((group) => ({
    group,
    color: MODIFIER_GROUP_COLORS[group],
    items: formatted.filter((f) => f.group === group),
  })).filter((g) => g.items.length > 0);
}
