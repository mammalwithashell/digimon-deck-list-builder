import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SelectionPanel } from './SelectionPanel';
import { GamePhase, type PendingSelection } from '@/types/game';

function makeEffectChoiceSelection(): PendingSelection {
  return {
    phase: GamePhase.SelectEffectChoice,
    selectingPlayer: 1,
    validIndices: [30, 31],
    isOptional: false,
    prompt: 'Choose which triggered effect to resolve next (2 pending)',
    kind: 'EffectChoice',
    effectChoices: [
      {
        index: 0,
        cardId: 'ST6-03',
        cardName: 'Gabumon',
        label: 'ST6-03 slot 0 (mandatory)',
        actionId: 30,
      },
      {
        index: 1,
        cardId: 'ST6-06',
        cardName: 'Garurumon',
        label: 'ST6-06 slot 0 (mandatory)',
        actionId: 31,
      },
    ],
  } as PendingSelection;
}

function renderPanel(overrides: Partial<Parameters<typeof SelectionPanel>[0]> = {}) {
  const mask = new Array(2192).fill(0);
  mask[30] = 1;
  mask[31] = 1;
  return render(
    <SelectionPanel
      currentPhase={GamePhase.SelectEffectChoice}
      pendingSelection={makeEffectChoiceSelection()}
      actionMask={mask}
      handIds={[]}
      trashIds={[]}
      securityIds={[]}
      battleArea={[]}
      onAction={vi.fn()}
      localPlayer={1}
      {...overrides}
    />,
  );
}

describe('SelectionPanel effect choice (trigger-order chooser)', () => {
  it('renders the real source card for each effect choice', () => {
    renderPanel();
    expect(screen.getByTitle('ST6-03')).toBeInTheDocument();
    expect(screen.getByTitle('ST6-06')).toBeInTheDocument();
  });

  it('right-clicking a choice card opens the enlarged inspect (DCGO parity)', () => {
    const onInspectCard = vi.fn();
    renderPanel({ onInspectCard });
    const evt = fireEvent.contextMenu(screen.getByTitle('ST6-03'));
    expect(onInspectCard).toHaveBeenCalledWith('ST6-03');
    expect(evt).toBe(false); // native menu suppressed
  });
});
