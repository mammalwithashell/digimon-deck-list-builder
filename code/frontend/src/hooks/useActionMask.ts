import { useMemo } from 'react';
import {
  ACTION,
  ATTACK_TARGETS_PER_SLOT,
  ATTACK_TARGET_SECURITY,
  BREEDING_SLOT,
  DIGIVOLVE_FIELDS_PER_HAND,
  SELECTION,
  MAX_BATTLE_AREA_SLOTS,
} from '@/utils/constants';

export interface ParsedMask {
  canPlayFromHand: Set<number>;
  canTrashFromHand: Set<number>;
  canHatch: boolean;
  canMove: boolean;
  canPass: boolean;
  canDnaDigivolve: Set<number>;
  /** Map<attackerSlot, Set<targetIndex>> */
  canAttack: Map<number, Set<number>>;
  /** Map<handIndex, Set<fieldSlot>> */
  canDigivolve: Map<number, Set<number>>;
  /** Hand indices that can digivolve onto breeding area (virtual slot 14). */
  canDigivolveBreeding: Set<number>;
  /** Map<sourceSlot, Set<effectIdx>> */
  canActivateEffect: Map<number, Set<number>>;
  /** Valid selection indices for selection phases */
  validSelections: Set<number>;
  /** Can attack security */
  canAttackSecurity: Map<number, boolean>;
}

export function useActionMask(mask: number[]): ParsedMask {
  return useMemo(() => {
    const canPlayFromHand = new Set<number>();
    const canTrashFromHand = new Set<number>();
    const canDnaDigivolve = new Set<number>();
    const canAttack = new Map<number, Set<number>>();
    const canAttackSecurity = new Map<number, boolean>();
    const canDigivolve = new Map<number, Set<number>>();
    const canDigivolveBreeding = new Set<number>();
    const canActivateEffect = new Map<number, Set<number>>();
    const validSelections = new Set<number>();

    if (mask.length === 0) {
      return {
        canPlayFromHand,
        canTrashFromHand,
        canHatch: false,
        canMove: false,
        canPass: false,
        canDnaDigivolve,
        canAttack,
        canDigivolve,
        canDigivolveBreeding,
        canActivateEffect,
        validSelections,
        canAttackSecurity,
      };
    }

    // Play from hand (0-29)
    for (let i = ACTION.PLAY_START; i <= ACTION.PLAY_END; i++) {
      if (mask[i] === 1) canPlayFromHand.add(i);
    }

    // Trash from hand (30-59)
    for (let i = ACTION.TRASH_START; i <= ACTION.TRASH_END; i++) {
      if (mask[i] === 1) canTrashFromHand.add(i - ACTION.TRASH_START);
    }

    // DNA Digivolve (63-92)
    for (let i = ACTION.DNA_START; i <= ACTION.DNA_END; i++) {
      if (mask[i] === 1) canDnaDigivolve.add(i - ACTION.DNA_START);
    }

    // Attack (100-399)
    for (let i = ACTION.ATTACK_START; i <= ACTION.ATTACK_END; i++) {
      if (mask[i] !== 1) continue;
      const offset = i - ACTION.ATTACK_START;
      const attacker = Math.floor(offset / ATTACK_TARGETS_PER_SLOT);
      const target = offset % ATTACK_TARGETS_PER_SLOT;
      if (!canAttack.has(attacker)) canAttack.set(attacker, new Set());
      canAttack.get(attacker)!.add(target);
      if (target === ATTACK_TARGET_SECURITY) {
        canAttackSecurity.set(attacker, true);
      }
    }

    // Digivolve (400-999)
    for (let i = ACTION.DIGIVOLVE_START; i <= ACTION.DIGIVOLVE_END; i++) {
      if (mask[i] !== 1) continue;
      const offset = i - ACTION.DIGIVOLVE_START;
      const hand = Math.floor(offset / DIGIVOLVE_FIELDS_PER_HAND);
      const field = offset % DIGIVOLVE_FIELDS_PER_HAND;
      if (field === BREEDING_SLOT) {
        canDigivolveBreeding.add(hand);
        continue;
      }
      if (field >= MAX_BATTLE_AREA_SLOTS) continue;
      if (!canDigivolve.has(hand)) canDigivolve.set(hand, new Set());
      canDigivolve.get(hand)!.add(field);
    }

    // Effects (1000-1999)
    for (let i = ACTION.EFFECT_START; i <= ACTION.EFFECT_END; i++) {
      if (mask[i] !== 1) continue;
      const offset = i - ACTION.EFFECT_START;
      const source = Math.floor(offset / 10);
      const effectIdx = offset % 10;
      if (!canActivateEffect.has(source)) canActivateEffect.set(source, new Set());
      canActivateEffect.get(source)!.add(effectIdx);
    }

    // Selection phases — aggregate all valid indices
    // Hand selections
    for (let i = SELECTION.HAND_START; i <= SELECTION.HAND_END; i++) {
      if (mask[i] === 1) validSelections.add(i);
    }
    // Revealed
    for (let i = SELECTION.REVEALED_START; i <= SELECTION.REVEALED_END; i++) {
      if (mask[i] === 1) validSelections.add(i);
    }
    // Own security
    for (let i = SELECTION.OWN_SECURITY_START; i <= SELECTION.OWN_SECURITY_END; i++) {
      if (mask[i] === 1) validSelections.add(i);
    }
    // Enemy security
    for (let i = SELECTION.ENEMY_SECURITY_START; i <= SELECTION.ENEMY_SECURITY_END; i++) {
      if (mask[i] === 1) validSelections.add(i);
    }
    // Own field
    for (let i = SELECTION.OWN_FIELD_START; i <= SELECTION.OWN_FIELD_END; i++) {
      if (mask[i] === 1) validSelections.add(i);
    }
    // Enemy field
    for (let i = SELECTION.ENEMY_FIELD_START; i <= SELECTION.ENEMY_FIELD_END; i++) {
      if (mask[i] === 1) validSelections.add(i);
    }
    // Trash
    for (let i = SELECTION.TRASH_START; i <= SELECTION.TRASH_END; i++) {
      if (mask[i] === 1) validSelections.add(i);
    }
    // Effect choice
    for (let i = SELECTION.EFFECT_CHOICE_START; i <= SELECTION.EFFECT_CHOICE_END; i++) {
      if (mask[i] === 1) validSelections.add(i);
    }

    return {
      canPlayFromHand,
      canTrashFromHand,
      canHatch: mask[ACTION.HATCH] === 1,
      canMove: mask[ACTION.MOVE] === 1,
      canPass: mask[ACTION.PASS] === 1,
      canDnaDigivolve,
      canAttack,
      canDigivolve,
      canDigivolveBreeding,
      canActivateEffect,
      validSelections,
      canAttackSecurity,
    };
  }, [mask]);
}
