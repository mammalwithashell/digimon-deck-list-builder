// ─── Action Space Ranges (2120 discrete actions) ───────────────────

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
  SOURCE_START: 2000,
  SOURCE_END: 2167,
} as const;

// Board layout constants
export const FIELD_SLOTS = 14;
export const BREEDING_SLOT = FIELD_SLOTS;  // 14
export const MAX_SOURCES = 11;

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
  TRASH_START: 130,
  TRASH_END: 179,
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

// ─── Board Layout ──────────────────────────────────────────────────

export const MAX_BATTLE_AREA_SLOTS = FIELD_SLOTS;  // 14
export const MAX_HAND_SIZE = 30;
export const MEMORY_MIN = -10;
export const MEMORY_MAX = 10;

// ─── Pagination ────────────────────────────────────────────────────

export const CARDS_PER_PAGE = 40;
