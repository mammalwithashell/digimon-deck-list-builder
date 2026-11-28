import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { TriggerChooserModal } from './TriggerChooserModal';
import { DecisionRouter } from './DecisionRouter';
import { GamePhase, type EffectChoice, type PendingSelection } from '@/types/game';
import { useResolutionPlanStore } from '@/stores/resolutionPlanStore';

describe('TriggerChooserModal & resolution plan safety', () => {
  beforeEach(() => {
    useResolutionPlanStore.getState().clearPlan();
    useResolutionPlanStore.getState().clearPermutationOrder();
  });

  it('resolves the 401a1e14 Hiro Amanokawa + Virus Busters trigger tier without locking up', () => {
    const onAction = vi.fn();
    const mask = new Array(2192).fill(0);
    mask[30] = 1;
    mask[31] = 1;
    const choices: EffectChoice[] = [
      {
        index: 0,
        cardId: 'EX12-066',
        cardName: 'Hiro Amanokawa',
        label: 'EX12-066 slot 1 (mandatory)',
        actionId: 30,
      },
      {
        index: 1,
        cardId: 'EX12-069',
        cardName: 'Virus Busters',
        label: 'EX12-069 slot 1 (optional)',
        actionId: 31,
      },
    ];
    const pending: PendingSelection = {
      phase: GamePhase.SelectEffectChoice,
      selectingPlayer: 1,
      validIndices: [30, 31],
      isOptional: false,
      prompt: 'Choose which triggered effect to resolve next (2 pending)',
      kind: 'TriggerOrder',
      effectChoices: choices,
    };

    render(
      <DecisionRouter
        currentPhase={GamePhase.SelectEffectChoice}
        pendingSelection={pending}
        actionMask={mask}
        firstPlayer={0}
        handIds={[]}
        securityCount={5}
        opponentSecurityCount={5}
        battleArea={[]}
        opponentBattleArea={[]}
        trashIds={[]}
        opponentTrashIds={[]}
        localPlayer={1}
        isGameOver={false}
        onAction={onAction}
      />,
    );

    const resolveBtn = screen.getByTestId('trigger-resolve-confirm');
    expect(resolveBtn).toBeEnabled();
    fireEvent.click(resolveBtn);
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(30);

    // Calling consumeNextAction on the SAME pendingSelection object must NOT
    // prematurely auto-dispatch action 31 before the server returns a new state.
    const premature = useResolutionPlanStore
      .getState()
      .consumeNextAction(pending, mask, 62);
    expect(premature).toBeNull();
  });

  it('clears stale ordered keys when a new trigger tier replaces an older tier', () => {
    const onAction = vi.fn();
    const mask = new Array(2192).fill(0);
    mask[30] = 1;
    mask[31] = 1;

    const tier1: EffectChoice[] = [
      { index: 0, cardId: 'BT1-001', cardName: 'Card A', label: 'A effect', actionId: 30 },
      { index: 1, cardId: 'BT1-002', cardName: 'Card B', label: 'B effect', actionId: 31 },
    ];
    const tier2: EffectChoice[] = [
      { index: 0, cardId: 'EX12-066', cardName: 'Hiro Amanokawa', label: 'EX12-066 slot 1 (mandatory)', actionId: 30 },
      { index: 1, cardId: 'EX12-069', cardName: 'Virus Busters', label: 'EX12-069 slot 1 (optional)', actionId: 31 },
    ];

    const { rerender } = render(
      <TriggerChooserModal
        title="Tier 1"
        choices={tier1}
        actionMask={mask}
        canDecline={false}
        declineActionId={62}
        onAction={onAction}
      />,
    );

    // Explicitly click Card B in tier 1 so local `order` holds tier 1's key
    fireEvent.click(screen.getByText('Card B (BT1-002)'));

    // Now tier 2 arrives in the same mounted component
    rerender(
      <TriggerChooserModal
        title="Tier 2"
        choices={tier2}
        actionMask={mask}
        canDecline={false}
        declineActionId={62}
        onAction={onAction}
      />,
    );

    const resolveBtn = screen.getByTestId('trigger-resolve-confirm');
    expect(resolveBtn).toBeEnabled();
    fireEvent.click(resolveBtn);
    expect(onAction).toHaveBeenCalledWith(30);
  });
});
