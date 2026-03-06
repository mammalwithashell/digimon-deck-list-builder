import {
  ACTION,
  FIELD_SLOTS,
  BREEDING_SLOT,
  ATTACK_TARGETS_PER_SLOT,
  ATTACK_TARGET_SECURITY,
  DIGIVOLVE_FIELDS_PER_HAND,
  EFFECTS_PER_SOURCE,
  SOURCES_PER_FIELD,
} from './constants';

export function decodeAction(id: number): string {
  if (id >= ACTION.PLAY_START && id <= ACTION.PLAY_END) {
    return `Play hand card ${id}`;
  }
  if (id >= ACTION.TRASH_START && id <= ACTION.TRASH_END) {
    return `Trash hand card ${id - ACTION.TRASH_START}`;
  }
  if (id === ACTION.HATCH) return 'Hatch from egg deck';
  if (id === ACTION.MOVE) return 'Move from breeding area';
  if (id === ACTION.PASS) return 'Pass / Decline';
  if (id >= ACTION.DNA_START && id <= ACTION.DNA_END) {
    return `DNA Digivolve with hand card ${id - ACTION.DNA_START}`;
  }

  if (id >= ACTION.ATTACK_START && id <= ACTION.ATTACK_END) {
    const offset = id - ACTION.ATTACK_START;
    const attacker = Math.floor(offset / ATTACK_TARGETS_PER_SLOT);
    const target = offset % ATTACK_TARGETS_PER_SLOT;
    if (target === ATTACK_TARGET_SECURITY) {
      return `Slot ${attacker} attacks security`;
    }
    if (target < FIELD_SLOTS) {
      return `Slot ${attacker} attacks opponent slot ${target}`;
    }
    return `Slot ${attacker} attack (target ${target})`;
  }

  if (id >= ACTION.DIGIVOLVE_START && id <= ACTION.DIGIVOLVE_END) {
    const offset = id - ACTION.DIGIVOLVE_START;
    const hand = Math.floor(offset / DIGIVOLVE_FIELDS_PER_HAND);
    const field = offset % DIGIVOLVE_FIELDS_PER_HAND;
    if (field === BREEDING_SLOT) {
      return `Digivolve hand ${hand} onto breeding area`;
    }
    return `Digivolve hand ${hand} onto slot ${field}`;
  }

  if (id >= ACTION.EFFECT_START && id <= ACTION.EFFECT_END) {
    const offset = id - ACTION.EFFECT_START;
    const source = Math.floor(offset / EFFECTS_PER_SOURCE);
    const effectIdx = offset % EFFECTS_PER_SOURCE;
    return `Activate effect ${effectIdx} on source ${source}`;
  }

  if (id >= ACTION.SOURCE_START && id <= ACTION.SOURCE_END) {
    const offset = id - ACTION.SOURCE_START;
    const field = Math.floor(offset / SOURCES_PER_FIELD);
    const sourceIdx = offset % SOURCES_PER_FIELD;
    return `Select source ${sourceIdx} on slot ${field}`;
  }

  return `Unknown action ${id}`;
}
