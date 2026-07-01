import { describe, expect, it } from 'vitest';
import { isBoardDrivenSelection } from './selectionSurface';
import { GamePhase, type PendingSelection } from '@/types/game';
import { SELECTION } from './constants';
import { SOURCE_SELECT_START } from './sourceSelection';

function pending(overrides: Partial<PendingSelection> = {}): PendingSelection {
  return {
    phase: GamePhase.SelectTarget,
    validIndices: [],
    isOptional: false,
    prompt: '',
    selectingPlayer: 1,
    ...overrides,
  };
}

describe('isBoardDrivenSelection', () => {
  it('is false with no pending selection', () => {
    expect(isBoardDrivenSelection(null, GamePhase.Main)).toBe(false);
    expect(isBoardDrivenSelection(undefined, GamePhase.Main)).toBe(false);
  });

  it('is true for own/opponent field-target picks (single and capped-multi)', () => {
    expect(
      isBoardDrivenSelection(
        pending({ kind: 'OwnField', validIndices: [SELECTION.OWN_FIELD_START] }),
        GamePhase.SelectTarget,
      ),
    ).toBe(true);
    expect(
      isBoardDrivenSelection(
        pending({ kind: 'OppField', validIndices: [SELECTION.OWN_FIELD_START + 2] }),
        GamePhase.SelectBudgeted,
      ),
    ).toBe(true);
  });

  it('is true for AnyField picks', () => {
    expect(
      isBoardDrivenSelection(pending({ kind: 'AnyField' }), GamePhase.SelectTarget),
    ).toBe(true);
  });

  it('is true for DNA material picks (raw field indices in SelectMaterial)', () => {
    expect(
      isBoardDrivenSelection(
        pending({ kind: 'Material', validIndices: [0, 1] }),
        GamePhase.SelectMaterial,
      ),
    ).toBe(true);
  });

  it('is false for a source-select pick in SelectMaterial (rendered in a modal)', () => {
    expect(
      isBoardDrivenSelection(
        pending({ kind: 'Material', validIndices: [SOURCE_SELECT_START] }),
        GamePhase.SelectMaterial,
      ),
    ).toBe(false);
  });

  it('is false for modal selections (hand / trash / reveal / effect-choice)', () => {
    expect(
      isBoardDrivenSelection(pending({ kind: 'Hand' }), GamePhase.SelectHand),
    ).toBe(false);
    expect(
      isBoardDrivenSelection(pending({ kind: 'Trash' }), GamePhase.SelectTrash),
    ).toBe(false);
    expect(
      isBoardDrivenSelection(pending({ kind: 'Reveal' }), GamePhase.SelectReveal),
    ).toBe(false);
  });
});
