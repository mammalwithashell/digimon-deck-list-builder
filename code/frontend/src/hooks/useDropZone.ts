import { useCallback } from 'react';
import { ACTION, DIGIVOLVE_FIELDS_PER_HAND } from '@/utils/constants';
import type { ParsedMask } from './useActionMask';

export interface DragData {
  type: 'hand-card' | 'breeding-perm';
  handIndex?: number;
  cardId?: string;
}

export interface DropResult {
  actionId: number;
}

/**
 * Compute the action ID for a drag-and-drop operation, or null if invalid.
 */
export function useDropZone(parsedMask: ParsedMask) {
  const getDropAction = useCallback(
    (
      dragData: DragData,
      dropZone: { type: 'empty-field-slot' | 'occupied-field-slot' | 'breeding-slot'; slotIndex: number },
    ): number | null => {
      if (dragData.type === 'hand-card' && dragData.handIndex !== undefined) {
        const handIdx = dragData.handIndex;

        if (dropZone.type === 'empty-field-slot') {
          // Play from hand
          if (parsedMask.canPlayFromHand.has(handIdx)) {
            return handIdx; // action 0-29
          }
        }

        if (dropZone.type === 'occupied-field-slot') {
          // Digivolve onto existing permanent
          const fieldSlots = parsedMask.canDigivolve.get(handIdx);
          if (fieldSlots?.has(dropZone.slotIndex)) {
            return ACTION.DIGIVOLVE_START + handIdx * DIGIVOLVE_FIELDS_PER_HAND + dropZone.slotIndex;
          }
        }

        if (dropZone.type === 'breeding-slot') {
          if (parsedMask.canDigivolveBreeding.has(handIdx)) {
            return ACTION.DIGIVOLVE_START + handIdx * DIGIVOLVE_FIELDS_PER_HAND + 12;
          }
        }
      }

      if (dragData.type === 'breeding-perm') {
        if (dropZone.type === 'empty-field-slot') {
          // Move from breeding
          if (parsedMask.canMove) {
            return ACTION.MOVE;
          }
        }
      }

      return null;
    },
    [parsedMask],
  );

  const isValidDrop = useCallback(
    (dragData: DragData, dropZone: { type: string; slotIndex: number }): boolean => {
      return getDropAction(dragData, dropZone as { type: 'empty-field-slot' | 'occupied-field-slot' | 'breeding-slot'; slotIndex: number }) !== null;
    },
    [getDropAction],
  );

  return { getDropAction, isValidDrop };
}
