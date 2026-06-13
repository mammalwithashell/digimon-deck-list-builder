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
      securityCount={0}
      opponentSecurityCount={0}
      battleArea={[]}
      onAction={vi.fn()}
      localPlayer={1}
      {...overrides}
    />,
  );
}

function makeTrashSelection(zoneOwner?: number): PendingSelection {
  return {
    phase: GamePhase.SelectTrash,
    selectingPlayer: 1,
    validIndices: [130, 131],
    isOptional: false,
    prompt: "Return 1 card from your opponent's trash to the bottom of the deck",
    kind: 'Trash',
    zoneOwner,
  } as PendingSelection;
}

describe('SelectionPanel trash selection zone owner', () => {
  function renderTrashPanel(zoneOwner: number | undefined, onAction = vi.fn()) {
    const mask = new Array(2192).fill(0);
    mask[130] = 1;
    mask[131] = 1;
    render(
      <SelectionPanel
        currentPhase={GamePhase.SelectTrash}
        pendingSelection={makeTrashSelection(zoneOwner)}
        actionMask={mask}
        handIds={[]}
        trashIds={['ST1-02', 'ST1-03']}
        opponentTrashIds={['EX11-012', 'BT21-029']}
        securityCount={0}
        opponentSecurityCount={0}
        battleArea={[]}
        onAction={onAction}
        localPlayer={1}
      />,
    );
    return onAction;
  }

  it("renders the OPPONENT's trash when zoneOwner is the opponent (EX11-012 Medusamon)", () => {
    const onAction = renderTrashPanel(2);
    expect(screen.getByTitle('EX11-012')).toBeInTheDocument();
    expect(screen.getByTitle('BT21-029')).toBeInTheDocument();
    expect(screen.queryByTitle('ST1-02')).not.toBeInTheDocument();
    fireEvent.click(screen.getByTitle('EX11-012'));
    expect(onAction).toHaveBeenCalledWith(130);
  });

  it('renders the own trash when zoneOwner is absent (legacy) or self', () => {
    renderTrashPanel(undefined);
    expect(screen.getByTitle('ST1-02')).toBeInTheDocument();
    expect(screen.queryByTitle('EX11-012')).not.toBeInTheDocument();
  });
});

function makeSecuritySelection(zoneOwner?: number): PendingSelection {
  return {
    phase: GamePhase.SelectSecurity,
    selectingPlayer: 1,
    validIndices: [],
    isOptional: true,
    prompt: "You may trash any 1 of your opponent's security cards",
    kind: 'Security',
    zoneOwner,
  } as PendingSelection;
}

describe('SelectionPanel security selection zone owner', () => {
  // Regression for BT24-018 Styracomon: an opponent-security prompt rendered
  // against the local player's stack (action ids 40-49) instead of the
  // opponent's (50-59), so the modal showed nothing selectable.
  it("renders the OPPONENT's security positions (ids 50+) when zoneOwner is the opponent", () => {
    const mask = new Array(2192).fill(0);
    mask[50] = 1; // opponent security position 0 valid
    mask[51] = 1; // opponent security position 1 valid
    const onAction = vi.fn();
    render(
      <SelectionPanel
        currentPhase={GamePhase.SelectSecurity}
        pendingSelection={makeSecuritySelection(2)}
        actionMask={mask}
        handIds={[]}
        trashIds={[]}
        securityCount={5}
        opponentSecurityCount={3}
        battleArea={[]}
        onAction={onAction}
        localPlayer={1}
      />,
    );
    // 3 face-down opponent-security positions rendered.
    expect(screen.getAllByText('DCG')).toHaveLength(3);
    // Clicking the first (valid) position fires the opponent-security action id.
    fireEvent.click(screen.getAllByText('DCG')[0]!);
    expect(onAction).toHaveBeenCalledWith(50);
  });

  it('renders the OWN security positions (ids 40+) when zoneOwner is self', () => {
    const mask = new Array(2192).fill(0);
    mask[40] = 1;
    const onAction = vi.fn();
    render(
      <SelectionPanel
        currentPhase={GamePhase.SelectSecurity}
        pendingSelection={makeSecuritySelection(1)}
        actionMask={mask}
        handIds={[]}
        trashIds={[]}
        securityCount={2}
        opponentSecurityCount={4}
        battleArea={[]}
        onAction={onAction}
        localPlayer={1}
      />,
    );
    // Own stack has 2 cards → 2 positions, not the opponent's 4.
    expect(screen.getAllByText('DCG')).toHaveLength(2);
    fireEvent.click(screen.getAllByText('DCG')[0]!);
    expect(onAction).toHaveBeenCalledWith(40);
  });
});

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
