import { useCallback } from 'react';
import {
  ACTION,
  ATTACK_TARGETS_PER_SLOT,
  ATTACK_TARGET_SECURITY,
  BREEDING_SLOT,
  DIGIVOLVE_FIELDS_PER_HAND,
} from '@/utils/constants';
import type { ParsedMask } from './useActionMask';

export interface DragData {
  type: 'hand-card' | 'breeding-perm' | 'attacker-perm';
  handIndex?: number;
  slotIndex?: number;
  cardId?: string;
}

export interface DropZoneData {
  type:
    | 'empty-field-slot'
    | 'occupied-field-slot'
    | 'breeding-slot'
    | 'enemy-field-slot'
    | 'enemy-security';
  slotIndex: number;
}

export interface DropResult {
  actionId: number;
}

/**
 * Compute the action ID for a drag-and-drop operation (play, digivolve,
 * move from breeding, or drag-to-attack an enemy Digimon / Security), or null if invalid.
 */
export function useDropZone(parsedMask: ParsedMask) {
  const getDropAction = useCallback(
    (dragData: DragData, dropZone: DropZoneData): number | null => {
      if (dragData.type === 'hand-card' && dragData.handIndex !== undefined) {
        const handIdx = dragData.handIndex;

        if (dropZone.type === 'empty-field-slot') {
          if (parsedMask.canPlayFromHand.has(handIdx)) {
            return handIdx; // action 0-29
          }
        }

        if (dropZone.type === 'occupied-field-slot') {
          const fieldSlots = parsedMask.canDigivolve.get(handIdx);
          if (fieldSlots?.has(dropZone.slotIndex)) {
            return (
              ACTION.DIGIVOLVE_START +
              handIdx * DIGIVOLVE_FIELDS_PER_HAND +
              dropZone.slotIndex
            );
          }
        }

        if (dropZone.type === 'breeding-slot') {
          if (parsedMask.canDigivolveBreeding.has(handIdx)) {
            return (
              ACTION.DIGIVOLVE_START +
              handIdx * DIGIVOLVE_FIELDS_PER_HAND +
              BREEDING_SLOT
            );
          }
        }
      }

      if (dragData.type === 'breeding-perm') {
        if (dropZone.type === 'empty-field-slot') {
          if (parsedMask.canMove) {
            return ACTION.MOVE;
          }
        }
      }

      if (dragData.type === 'attacker-perm' && dragData.slotIndex !== undefined) {
        const attacker = dragData.slotIndex;
        if (dropZone.type === 'enemy-field-slot') {
          const targets = parsedMask.canAttack.get(attacker);
          if (targets?.has(dropZone.slotIndex)) {
            return (
              ACTION.ATTACK_START +
              attacker * ATTACK_TARGETS_PER_SLOT +
              dropZone.slotIndex
            );
          }
        }
        if (dropZone.type === 'enemy-security') {
          if (parsedMask.canAttackSecurity.get(attacker)) {
            return (
              ACTION.ATTACK_START +
              attacker * ATTACK_TARGETS_PER_SLOT +
              ATTACK_TARGET_SECURITY
            );
          }
        }
      }

      return null;
    },
    [parsedMask],
  );

  const isValidDrop = useCallback(
    (dragData: DragData, dropZone: { type: string; slotIndex: number }): boolean => {
      return getDropAction(dragData, dropZone as DropZoneData) !== null;
    },
    [getDropAction],
  );

  return { getDropAction, isValidDrop };
}
