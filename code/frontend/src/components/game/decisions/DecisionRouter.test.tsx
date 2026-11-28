import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DecisionRouter } from './DecisionRouter';
import { useResolutionPlanStore } from '@/stores/resolutionPlanStore';
import { usePeekStore } from '@/stores/peekStore';
import { GamePhase, type PendingSelection, type PermanentInfo } from '@/types/game';

function makePerm(topCardId: string, topCardName: string, sources: { cardId: string; cardName: string }[]): PermanentInfo {
  return {
    topCardId,
    topCardName,
    dp: 6000,
    level: 4,
    isSuspended: false,
    sourceCount: sources.length,
    keywords: [],
    keywordBreakdown: { innate: [], gained: [] },
    securityAttackModifier: 0,
    linkedCardIds: [],
    sources: [
      {
        cardId: topCardId,
        cardName: topCardName,
        isTop: true,
        optState: 0,
        dpContribution: 0,
        mainEffectText: '',
        inheritedEffectText: '',
        colors: [0],
      },
      ...sources.map((s) => ({
        cardId: s.cardId,
        cardName: s.cardName,
        isTop: false,
        optState: 0,
        dpContribution: 0,
        mainEffectText: '',
        inheritedEffectText: '',
        colors: [0],
      })),
    ],
    mainEffectText: '',
    inheritedEffects: [],
    modifiers: [],
    dpBreakdown: { base: 6000, sources: [], temporary: 0, total: 6000 },
    turnPlayed: 1,
    colors: [0],
  };
}

describe('DecisionRouter Track A Depth flows', () => {
  beforeEach(() => {
    useResolutionPlanStore.setState({ plan: null, permutationQueue: [] });
    usePeekStore.setState({ peeking: false });
  });

  it('renders a docked SelectHand rail by default with Grid View fallback', () => {
    const onHandGridOverride = vi.fn();
    const mask = new Array(2192).fill(0);
    mask[0] = 1;
    mask[62] = 1;

    render(
      <DecisionRouter
        currentPhase={GamePhase.SelectHand}
        pendingSelection={{
          phase: GamePhase.SelectHand,
          selectingPlayer: 1,
          validIndices: [0, 62],
          isOptional: true,
          prompt: 'Trash 1 card from your hand',
        }}
        actionMask={mask}
        firstPlayer={0}
        handIds={['BT1-010', 'BT1-015']}
        securityCount={5}
        opponentSecurityCount={5}
        battleArea={[]}
        opponentBattleArea={[]}
        trashIds={[]}
        opponentTrashIds={[]}
        localPlayer={1}
        isGameOver={false}
        handGridOverride={false}
        onHandGridOverride={onHandGridOverride}
        onAction={vi.fn()}
      />,
    );

    expect(screen.getByTestId('board-decision-rail')).toBeInTheDocument();
    expect(screen.queryByTestId('selection-panel')).not.toBeInTheDocument();
    fireEvent.click(screen.getByTestId('hand-open-grid'));
    expect(onHandGridOverride).toHaveBeenCalledWith(true);
  });

  it('uses two-step SourceHostPicker for multi-host SOURCE_SELECT (Step 1 rail -> Step 2 host stack)', () => {
    const onAction = vi.fn();
    const onSelectSourceHost = vi.fn();
    const mask = new Array(2192).fill(0);
    mask[2000] = 1;
    mask[2012] = 1;

    const battleArea = [
      makePerm('BT1-025', 'WarGreymon', [{ cardId: 'BT1-010', cardName: 'Agumon' }]),
      makePerm('BT1-044', 'MetalGarurumon', [{ cardId: 'BT1-029', cardName: 'Gabumon' }]),
    ];

    const pending: PendingSelection = {
      phase: GamePhase.SelectSource,
      selectingPlayer: 1,
      validIndices: [2000, 2012],
      isOptional: false,
      prompt: 'Play 1 digivolution card from under your Digimon',
    };

    const { rerender } = render(
      <DecisionRouter
        currentPhase={GamePhase.SelectSource}
        pendingSelection={pending}
        actionMask={mask}
        firstPlayer={0}
        handIds={[]}
        securityCount={5}
        opponentSecurityCount={5}
        battleArea={battleArea}
        opponentBattleArea={[]}
        trashIds={[]}
        opponentTrashIds={[]}
        localPlayer={1}
        isGameOver={false}
        selectedSourceHostSlot={null}
        onSelectSourceHost={onSelectSourceHost}
        onAction={onAction}
      />,
    );

    // Step 1: docked rail asks player to click a Digimon on the board
    expect(screen.getByTestId('board-decision-rail')).toBeInTheDocument();
    expect(screen.getByText(/Step 1 of 2/i)).toBeInTheDocument();

    // Step 2: after selecting slot 1 on board, modal shows ONLY slot 1's source card (BT1-029)
    rerender(
      <DecisionRouter
        currentPhase={GamePhase.SelectSource}
        pendingSelection={pending}
        actionMask={mask}
        firstPlayer={0}
        handIds={[]}
        securityCount={5}
        opponentSecurityCount={5}
        battleArea={battleArea}
        opponentBattleArea={[]}
        trashIds={[]}
        opponentTrashIds={[]}
        localPlayer={1}
        isGameOver={false}
        selectedSourceHostSlot={1}
        onSelectSourceHost={onSelectSourceHost}
        onAction={onAction}
      />,
    );

    expect(screen.getByTestId('selection-panel')).toBeInTheDocument();
    expect(screen.getByTitle('BT1-029')).toBeInTheDocument();
    expect(screen.queryByTitle('BT1-010')).not.toBeInTheDocument();

    fireEvent.click(screen.getByTestId('source-change-host'));
    expect(onSelectSourceHost).toHaveBeenCalledWith(null);

    fireEvent.click(screen.getByTitle('BT1-029'));
    expect(onAction).toHaveBeenCalledWith(2012);
  });

  it('routes SelectPermutation with 2+ revealed cards to OrderCardsModal', () => {
    const mask = new Array(2192).fill(0);
    mask[30] = 1;
    mask[31] = 1;

    render(
      <DecisionRouter
        currentPhase={GamePhase.SelectPermutation}
        pendingSelection={{
          phase: GamePhase.SelectPermutation,
          selectingPlayer: 1,
          validIndices: [30, 31],
          isOptional: false,
          prompt: 'Place remaining cards at the bottom of your deck in any order',
        }}
        actionMask={mask}
        firstPlayer={0}
        handIds={[]}
        securityCount={5}
        opponentSecurityCount={5}
        battleArea={[]}
        opponentBattleArea={[]}
        trashIds={[]}
        opponentTrashIds={[]}
        revealedCards={[
          { cardId: 'BT1-010', owner: 1 },
          { cardId: 'BT1-015', owner: 1 },
        ]}
        localPlayer={1}
        isGameOver={false}
        onAction={vi.fn()}
      />,
    );

    expect(screen.getByTestId('order-cards-modal')).toBeInTheDocument();
  });

  it('routes Digi-Xros SelectMaterial with cardChoice to SelectionPanel showing Hand and Battle cards', () => {
    const onAction = vi.fn();
    const mask = new Array(2192).fill(0);
    mask[30] = 1;
    mask[31] = 1;
    mask[62] = 1;

    render(
      <DecisionRouter
        currentPhase={GamePhase.SelectMaterial}
        pendingSelection={{
          phase: GamePhase.SelectMaterial,
          selectingPlayer: 1,
          validIndices: [30, 31, 62],
          isOptional: true,
          kind: 'Material',
          prompt: 'Select DigiXros material',
          cardChoice: {
            prompt_id: '7',
            page_generation: '0',
            page_index: '0',
            page_count: '1',
            rows: [
              {
                action_id: 30,
                operation: 'commit_card',
                zone: 'hand',
                owner: 0,
                carrier_slot: null,
                carrier_identity: null,
                position: '0',
                card_id: 'BT10-009',
                card_registry_index: 10,
                identity_visible: true,
              },
              {
                action_id: 31,
                operation: 'commit_card',
                zone: 'battle_top',
                owner: 0,
                carrier_slot: 0,
                carrier_identity: null,
                position: '0',
                card_id: 'BT10-011',
                card_registry_index: 11,
                identity_visible: true,
              },
            ],
          },
        }}
        actionMask={mask}
        firstPlayer={0}
        handIds={['BT10-009']}
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

    expect(screen.getByTestId('selection-panel')).toBeInTheDocument();
    expect(screen.getByText('Hand 1')).toBeInTheDocument();
    expect(screen.getByText('Battle card 1')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Hand 1').closest('button')!);
    expect(onAction).toHaveBeenCalledWith(30);
  });

  it('routes mutually exclusive EffectChoice to SelectionPanel ("Choose One"), NOT TriggerChooserModal', () => {
    const onAction = vi.fn();
    const mask = new Array(2192).fill(0);
    mask[30] = 1;
    mask[31] = 1;
    mask[62] = 1;

    render(
      <DecisionRouter
        currentPhase={GamePhase.SelectEffectChoice}
        pendingSelection={{
          phase: GamePhase.SelectEffectChoice,
          selectingPlayer: 1,
          validIndices: [30, 31, 62],
          isOptional: true,
          kind: 'EffectChoice',
          prompt: 'Choose 1 of the following effects to activate',
          effectChoices: [
            {
              index: 0,
              cardId: 'EX12-069',
              cardName: 'Virus Busters',
              label: 'Draw 1 card',
              actionId: 30,
            },
            {
              index: 1,
              cardId: 'EX12-069',
              cardName: 'Virus Busters',
              label: 'Gain 1 memory',
              actionId: 31,
            },
          ],
        }}
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

    expect(screen.queryByTestId('trigger-chooser-modal')).not.toBeInTheDocument();
    expect(screen.getByTestId('selection-panel')).toBeInTheDocument();
    expect(screen.getByTestId('exclusive-effect-choices')).toBeInTheDocument();
    expect(screen.getByText('Choose One Effect (Optional)')).toBeInTheDocument();

    fireEvent.click(screen.getByText('Gain 1 memory'));
    expect(onAction).toHaveBeenCalledWith(31);

    fireEvent.click(screen.getByTestId('effect-choice-skip'));
    expect(onAction).toHaveBeenCalledWith(62);
  });
});
