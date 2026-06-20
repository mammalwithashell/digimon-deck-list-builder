// ─── Action Space Ranges (2192 discrete actions) ───────────────────

export const ACTION = {
  MULLIGAN_KEEP: 0,
  MULLIGAN_REDRAW: 1,
  PLAY_START: 0,
  PLAY_END: 29,
  TRASH_START: 30,
  TRASH_END: 59,
  HAND_MAIN_START: 30,
  HAND_MAIN_END: 59,
  HATCH: 60,
  MOVE: 61,
  PASS: 62,
  DNA_START: 63,
  DNA_END: 92,
  ATTACK_START: 100,
  ATTACK_END: 399,
  DIGIVOLVE_START: 400,
  DIGIVOLVE_END: 999,
  EFFECT_START: 1000,
  EFFECT_END: 1999,
  TRASH_MAIN_START: 1150,
  TRASH_MAIN_END: 1194,
  // Battle-area source selection.
  SOURCE_START: 2000,
  SOURCE_END: 2167,
  // Breeding-area carrier source selection (King Drasil — Task S1.3).
  // 2 carriers x 12 sources, keyed by carrier owner.
  BREEDING_SOURCE_START: 2168,
  BREEDING_SOURCE_END: 2191,
} as const;

// Board layout constants
export const FIELD_SLOTS = 14;
export const BREEDING_SLOT = FIELD_SLOTS;  // 14

// Attack formula: 100 + attacker * 15 + target
export const ATTACK_TARGETS_PER_SLOT = 15;
export const ATTACK_TARGET_SECURITY = FIELD_SLOTS;  // 14

// Digivolve formula: 400 + hand * 15 + field
export const DIGIVOLVE_FIELDS_PER_HAND = 15;

// Effect formula: 1000 + source * 10 + effectIdx
export const EFFECTS_PER_SOURCE = 10;

// Source formula: 2000 + field * 12 + sourceIdx
export const SOURCES_PER_FIELD = 12;

// ─── Selection Ranges ──────────────────────────────────────────────

export const SELECTION = {
  HAND_START: 0,
  HAND_END: 29,
  REVEALED_START: 30,
  REVEALED_END: 39,
  OWN_SECURITY_START: 40,
  OWN_SECURITY_END: 49,
  ENEMY_SECURITY_START: 50,
  ENEMY_SECURITY_END: 59,
  DECLINE: 62,
  BREEDING: 99,
  OWN_FIELD_START: 100,
  OWN_FIELD_END: 100 + FIELD_SLOTS - 1,       // 113
  ENEMY_FIELD_START: 100 + FIELD_SLOTS,        // 114
  ENEMY_FIELD_END: 100 + 2 * FIELD_SLOTS - 1,  // 127
  TRASH_START: 1150,
  TRASH_END: 1194,
  EFFECT_CHOICE_START: 1000,
  EFFECT_CHOICE_END: 1009,
} as const;

// ─── Game Phase Names ──────────────────────────────────────────────

export const PHASE_NAMES: Record<number, string> = {
  0: 'Start',
  1: 'Draw',
  2: 'Breeding',
  3: 'Main',
  4: 'End',
  5: 'Select Target',
  6: 'Select Material',
  7: 'Block Timing',
  8: 'Counter Timing',
  9: 'Select Trash',
  10: 'Select Source',
  11: 'Select Hand',
  12: 'Select Reveal',
  13: 'Effect Choice',
  14: 'Select Security',
  15: 'End of Turn Action',
  16: 'Alliance Timing',
  17: 'Mulligan',
};

// ─── Card Color Hex Values ─────────────────────────────────────────

export const COLOR_HEX: Record<string, string> = {
  Red: '#dc2626',
  Blue: '#2563eb',
  Yellow: '#eab308',
  Green: '#16a34a',
  White: '#e5e7eb',
  Black: '#1f2937',
  Purple: '#9333ea',
};

export const COLOR_NAMES: Record<number, string> = {
  0: 'Red',
  1: 'Blue',
  2: 'Yellow',
  3: 'Green',
  4: 'White',
  5: 'Black',
  6: 'Purple',
};

// ─── Keyword Display ───────────────────────────────────────────────

export const KEYWORD_COLORS: Record<string, string> = {
  blocker: '#3b82f6',
  piercing: '#ef4444',
  jamming: '#a855f7',
  retaliation: '#f97316',
  rush: '#22c55e',
  security_attack_plus: '#eab308',
  reboot: '#06b6d4',
  raid: '#dc2626',
  blitz: '#f59e0b',
  alliance: '#6366f1',
  collision: '#ec4899',
  training: '#10b981',
  progress: '#8b5cf6',
  fortitude: '#0ea5e9',
  save: '#14b8a6',
  decoy: '#f472b6',
  material_save: '#78716c',
  vortex: '#6d28d9',
  overclock: '#f43f5e',
  armor_purge: '#78716c',
  evade: '#64748b',
  barrier: '#0284c7',
  blast_digivolve: '#f59e0b',
  blast_dna_digivolve: '#d97706',
  delay: '#9ca3af',
  digisorption: '#0d9488',
  scapegoat: '#b91c1c',
  fragment: '#059669',
  iceclad: '#0ea5e9',
  decode: '#2563eb',
  execute: '#dc2626',
};

export const KEYWORD_DISPLAY: Record<string, string> = {
  blocker: 'Blocker',
  piercing: 'Piercing',
  jamming: 'Jamming',
  retaliation: 'Retaliation',
  rush: 'Rush',
  security_attack_plus: 'Security Attack +',
  reboot: 'Reboot',
  raid: 'Raid',
  blitz: 'Blitz',
  alliance: 'Alliance',
  collision: 'Collision',
  training: 'Training',
  progress: 'Progress',
  fortitude: 'Fortitude',
  save: 'Save',
  decoy: 'Decoy',
  material_save: 'Material Save',
  vortex: 'Vortex',
  overclock: 'Overclock',
  armor_purge: 'Armor Purge',
  evade: 'Evade',
  barrier: 'Barrier',
  blast_digivolve: 'Blast Digivolve',
  blast_dna_digivolve: 'Blast DNA Digivolve',
  delay: 'Delay',
  digisorption: 'Digisorption',
  scapegoat: 'Scapegoat',
  fragment: 'Fragment',
  iceclad: 'Iceclad',
  decode: 'Decode',
  execute: 'Execute',
  cannot_attack: 'Cannot Attack',
  cannot_attack_player: 'Cannot Attack Player',
  cannot_block: 'Cannot Block',
  cannot_be_blocked: 'Cannot Be Blocked',
  cannot_unsuspend: 'Cannot Unsuspend',
};

// ─── Active-modifier display (stack inspector) ─────────────────────
//
// Maps the engine's stable modifier-type strings (from `to_ui_json`'s
// permanent `modifiers` array) to a human label + a display group. The
// group drives ordering and color in the inspector's "Active Modifiers"
// section. Stat-change labels with a magnitude use the entry's `value`
// (see `formatModifierLabel`). Types not listed here fall back to the
// "Other" group with a humanized label — the panel never breaks on an
// unmapped type.

export type ModifierGroup = 'Immunity' | 'Restriction' | 'StatChange' | 'Other';

export interface ModifierDisplay {
  label: string;
  group: ModifierGroup;
}

export const MODIFIER_GROUP_ORDER: readonly ModifierGroup[] = [
  'StatChange',
  'Immunity',
  'Restriction',
  'Other',
];

export const MODIFIER_GROUP_COLORS: Record<ModifierGroup, string> = {
  StatChange: '#f59e0b', // amber
  Immunity: '#22c55e', // green
  Restriction: '#ef4444', // red
  Other: '#9ca3af', // gray
};

export const MODIFIER_DISPLAY: Record<string, ModifierDisplay> = {
  // Stat changes (label magnitude filled in from `value`)
  ChangeDp: { label: 'DP', group: 'StatChange' },
  ChangeBaseDp: { label: 'Base DP', group: 'StatChange' },
  DpFloor: { label: 'DP floor', group: 'StatChange' },
  DontHaveDp: { label: "Doesn't have DP", group: 'StatChange' },
  SecurityAttackChange: { label: 'Security Attack', group: 'StatChange' },
  ChangeLevel: { label: 'Level', group: 'StatChange' },
  ChangeColor: { label: 'Color changed', group: 'StatChange' },
  AddColor: { label: 'Color added', group: 'StatChange' },
  // Immunities / protection
  CannotBeDestroyed: { label: 'Cannot be deleted', group: 'Immunity' },
  CannotBeDestroyedByBattle: { label: 'Cannot be deleted in battle', group: 'Immunity' },
  CannotBeDestroyedByEffect: { label: 'Cannot be deleted by effects', group: 'Immunity' },
  CannotBeRemoved: { label: 'Cannot be removed', group: 'Immunity' },
  CannotBeReturnedToDeck: { label: 'Cannot be returned to deck', group: 'Immunity' },
  CannotBeReturnedToHand: { label: 'Cannot be returned to hand', group: 'Immunity' },
  CannotBeTrashedByEffect: { label: 'Cannot be trashed by effects', group: 'Immunity' },
  CannotBeDeDigivolved: { label: 'Cannot be de-digivolved', group: 'Immunity' },
  CannotBeSelectedByEffect: { label: 'Cannot be chosen by effects', group: 'Immunity' },
  CannotBeAffected: { label: "Unaffected by opponent's effects", group: 'Immunity' },
  ImmunityToOpponentEffects: { label: "Unaffected by opponent's effects", group: 'Immunity' },
  ImmuneFromDPMinus: { label: 'Immune to DP reduction', group: 'Immunity' },
  // Restrictions
  CannotAttack: { label: "Can't attack", group: 'Restriction' },
  CannotAttackPlayer: { label: "Can't attack players", group: 'Restriction' },
  CannotBlock: { label: "Can't block", group: 'Restriction' },
  CannotCounter: { label: "Can't counter", group: 'Restriction' },
  CannotSuspend: { label: "Can't suspend", group: 'Restriction' },
  CannotUnsuspend: { label: "Can't unsuspend", group: 'Restriction' },
  CannotDigivolve: { label: "Can't digivolve", group: 'Restriction' },
  CannotActivateOnPlayEffects: { label: "Can't activate [On Play] effects", group: 'Restriction' },
  CannotActivateMainEffects: { label: "Can't activate [Main] effects", group: 'Restriction' },
  CannotActivateWhenDigivolvingEffects: {
    label: "Can't activate [When Digivolving] effects",
    group: 'Restriction',
  },
  CannotActivateWhenAttackingEffects: {
    label: "Can't activate [When Attacking] effects",
    group: 'Restriction',
  },
  CannotActivateSecurityEffects: { label: "Can't activate security effects", group: 'Restriction' },
  // Attack grants / mandates
  MayAttack: { label: 'May attack', group: 'Other' },
  ForceAttack: { label: 'Must attack', group: 'Restriction' },
};

// ─── Board Layout ──────────────────────────────────────────────────

export const MAX_BATTLE_AREA_SLOTS = FIELD_SLOTS;  // 14
export const MAX_HAND_SIZE = 30;
export const MEMORY_MIN = -10;
export const MEMORY_MAX = 10;

// ─── Pagination ────────────────────────────────────────────────────

export const CARDS_PER_PAGE = 40;

// ─── Desktop Graphics ──────────────────────────────────────────────
//
// Fixed list of supported desktop window sizes, matching DCGO's preset
// list exactly. The Graphics Settings page renders these in order; the
// CanvasScaler reads the current selection from `useUiStore` and applies
// it via Tauri's window API.

export interface ResolutionPreset {
  width: number;
  height: number;
}

export const RESOLUTION_PRESETS: readonly ResolutionPreset[] = [
  { width: 1024, height: 576 },
  { width: 1280, height: 720 },
  { width: 1600, height: 900 },
  { width: 1920, height: 1080 },
  { width: 2560, height: 1440 },
  { width: 3440, height: 1440 },
  { width: 3840, height: 2160 },
  { width: 5160, height: 2160 },
] as const;

/** Default startup size for first-launch users. Matches the second
 *  preset and the Tauri default window size so there is no resize flash
 *  between pre-React window creation and post-React preset application. */
export const DEFAULT_PRESET: ResolutionPreset = { width: 1280, height: 720 };

/** The internal canvas the game UI authors against. Every preset is just
 *  this size scaled uniformly by `min(w/1920, h/1080)`. Do NOT change
 *  without re-auditing every fixed-pixel size in the game board CSS. */
export const DESIGN_CANVAS: ResolutionPreset = { width: 1920, height: 1080 };

/** Height (real, unscaled window pixels) of the custom desktop title bar
 *  that replaces the native window caption (decorations are off in
 *  tauri.conf.json). The CanvasScaler reserves this much vertical space at
 *  the top so the scaled game canvas sits *below* the bar rather than
 *  behind it. MUST match the `--titlebar-h` value in TitleBar.css
 *  (26px caption + 22px menu strip). */
export const TITLEBAR_HEIGHT = 48;
