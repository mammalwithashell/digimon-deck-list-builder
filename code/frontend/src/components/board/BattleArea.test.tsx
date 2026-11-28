import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { BattleArea } from './BattleArea';
import { __uiStoreInternals, useUiStore } from '@/stores/uiStore';
import type { PermanentInfo } from '@/types/game';

function makeDigimon(overrides: Partial<PermanentInfo> = {}): PermanentInfo {
  return {
    topCardId: 'BT1-010',
    topCardName: 'Agumon',
    dp: 2000,
    level: 3,
    isSuspended: false,
    sourceCount: 0,
    keywords: [],
    keywordBreakdown: { innate: [], gained: [] },
    securityAttackModifier: 0,
    linkedCardIds: [],
    sources: [],
    mainEffectText: '',
    inheritedEffects: [],
    modifiers: [],
    dpBreakdown: { base: 2000, sources: [], temporary: 0, total: 2000 },
    turnPlayed: 1,
    colors: [0],
    ...overrides,
  };
}

function makeTamer(overrides: Partial<PermanentInfo> = {}): PermanentInfo {
  return {
    topCardId: 'BT1-085',
    topCardName: 'Tai Kamiya',
    dp: 0,
    level: 0,
    isSuspended: false,
    sourceCount: 0,
    keywords: [],
    keywordBreakdown: { innate: [], gained: [] },
    securityAttackModifier: 0,
    linkedCardIds: [],
    sources: [],
    mainEffectText: '',
    inheritedEffects: [],
    modifiers: [],
    dpBreakdown: { base: 0, sources: [], temporary: 0, total: 0 },
    turnPlayed: 1,
    colors: [0],
    ...overrides,
  };
}

function makeToken(overrides: Partial<PermanentInfo> = {}): PermanentInfo {
  return makeDigimon({
    topCardId: 'TOKEN_PAISHU',
    topCardName: 'Paishu Token',
    dp: 6000,
    level: 0,
    dpBreakdown: { base: 6000, sources: [], temporary: 0, total: 6000 },
    ...overrides,
  });
}

describe('uiStore battleLanes preference', () => {
  const KEY = __uiStoreInternals.BATTLE_LANES_STORAGE_KEY;

  beforeEach(() => {
    window.localStorage.clear();
    useUiStore.setState({ battleLanes: 'two' });
  });

  it('defaults to two when nothing is persisted', () => {
    expect(__uiStoreInternals.loadPersistedBattleLanes()).toBe('two');
    expect(useUiStore.getState().battleLanes).toBe('two');
  });

  it('hydrates one from localStorage and ignores invalid values', () => {
    window.localStorage.setItem(KEY, 'one');
    expect(__uiStoreInternals.loadPersistedBattleLanes()).toBe('one');

    window.localStorage.setItem(KEY, 'three');
    expect(__uiStoreInternals.loadPersistedBattleLanes()).toBe('two');
  });

  it('persists battleLanes changes via setBattleLanes', () => {
    useUiStore.getState().setBattleLanes('one');
    expect(useUiStore.getState().battleLanes).toBe('one');
    expect(window.localStorage.getItem(KEY)).toBe('one');

    useUiStore.getState().setBattleLanes('two');
    expect(useUiStore.getState().battleLanes).toBe('two');
    expect(window.localStorage.getItem(KEY)).toBe('two');
  });
});

describe('BattleArea two-lane layout', () => {
  beforeEach(() => {
    window.localStorage.clear();
    useUiStore.setState({ battleLanes: 'two' });
  });

  it('places Digimon & Tokens in front row and Tamers & Delay Options in back row, ordered front-above-back for player', () => {
    const digimon = makeDigimon({ topCardId: 'BT1-010', topCardName: 'Agumon' });
    const tamer = makeTamer({ topCardId: 'BT1-085', topCardName: 'Tai Kamiya' });
    const token = makeToken();
    const delayOption = makeTamer({
      topCardId: 'P-035',
      topCardName: 'Red Memory Boost!',
      dp: 0,
      level: null,
    });

    const { container } = render(
      <BattleArea permanents={[tamer, digimon, delayOption, token]} isOpponent={false} />,
    );

    const area = screen.getByTestId('battle-area-you');
    expect(area).toHaveClass('hs-battle-area', 'hs-battle-area--laned');
    expect(area).toHaveAttribute('data-battle-lanes', 'two');

    const frontLane = screen.getByTestId('battle-lane-you-front');
    const backLane = screen.getByTestId('battle-lane-you-back');
    expect(frontLane).toHaveClass('hs-battle-lane', 'hs-battle-lane--front');
    expect(backLane).toHaveClass('hs-battle-lane', 'hs-battle-lane--back');

    // Front row renders ABOVE back row on player's side (closest to midline)
    const lanes = Array.from(container.querySelectorAll('.hs-battle-lane'));
    expect(lanes).toEqual([frontLane, backLane]);

    // Slots 1 (Agumon) and 3 (Token) in front; slots 0 (Tai) and 2 (Delay) in back
    expect(frontLane.querySelector('[data-testid="field-slot-p1-1"]')).toBeInTheDocument();
    expect(frontLane.querySelector('[data-testid="field-slot-p1-3"]')).toBeInTheDocument();
    expect(frontLane.querySelector('[data-testid="field-slot-p1-0"]')).toBeNull();

    expect(backLane.querySelector('[data-testid="field-slot-p1-0"]')).toBeInTheDocument();
    expect(backLane.querySelector('[data-testid="field-slot-p1-2"]')).toBeInTheDocument();
    expect(backLane.querySelector('[data-testid="field-slot-p1-1"]')).toBeNull();
  });

  it('mirrors row order for opponent so back row renders ABOVE front row (front row closest to midline)', () => {
    const digimon = makeDigimon({ topCardId: 'BT1-010', topCardName: 'Agumon' });
    const tamer = makeTamer({ topCardId: 'BT1-085', topCardName: 'Tai Kamiya' });

    const { container } = render(
      <BattleArea permanents={[digimon, tamer]} isOpponent />,
    );

    const area = screen.getByTestId('battle-area-opp');
    expect(area).toHaveClass('hs-battle-area', 'hs-battle-area--laned');

    const frontLane = screen.getByTestId('battle-lane-opp-front');
    const backLane = screen.getByTestId('battle-lane-opp-back');

    const lanes = Array.from(container.querySelectorAll('.hs-battle-lane'));
    expect(lanes).toEqual([backLane, frontLane]);

    expect(frontLane.querySelector('[data-testid="field-slot-p2-0"]')).toBeInTheDocument();
    expect(backLane.querySelector('[data-testid="field-slot-p2-1"]')).toBeInTheDocument();
  });

  it('omits empty row strips when only Digimon or only Tamers are present', () => {
    const digimon = makeDigimon();
    const { rerender } = render(
      <BattleArea permanents={[digimon]} isOpponent={false} />,
    );

    expect(screen.getByTestId('battle-lane-you-front')).toBeInTheDocument();
    expect(screen.queryByTestId('battle-lane-you-back')).toBeNull();

    const tamer = makeTamer();
    rerender(<BattleArea permanents={[tamer]} isOpponent={false} />);
    expect(screen.queryByTestId('battle-lane-you-front')).toBeNull();
    expect(screen.getByTestId('battle-lane-you-back')).toBeInTheDocument();

    rerender(<BattleArea permanents={[]} isOpponent={false} />);
    expect(screen.queryByTestId('battle-lane-you-front')).toBeNull();
    expect(screen.queryByTestId('battle-lane-you-back')).toBeNull();
    expect(screen.getByText('Battle Area')).toBeInTheDocument();
  });

  it('places the Play Here drop mat in the front row when dragging a playable card', () => {
    const tamer = makeTamer();
    render(
      <BattleArea
        permanents={[tamer]}
        isOpponent={false}
        isDraggingHandCard
        canPlayDragged
      />,
    );

    const frontLane = screen.getByTestId('battle-lane-you-front');
    const backLane = screen.getByTestId('battle-lane-you-back');
    expect(frontLane).toHaveTextContent('Play Here');
    expect(backLane).not.toHaveTextContent('Play Here');
  });

  it('preserves duplicate Tamer layering, stack badges, unstacking on multi-target, and click/inspect handlers', () => {
    const tamer1 = makeTamer({ topCardId: 'BT1-085', isSuspended: false });
    const tamer2 = makeTamer({ topCardId: 'BT1-085', isSuspended: true });
    const digimon = makeDigimon({ topCardId: 'BT1-010' });
    const onSlotClick = vi.fn();
    const onSlotInspect = vi.fn();

    const { container, rerender } = render(
      <BattleArea
        permanents={[tamer1, tamer2, digimon]}
        isOpponent={false}
        dragValidDropSlots={new Set([2])}
        isDraggingHandCard
        onSlotClick={onSlotClick}
        onSlotInspect={onSlotInspect}
      />,
    );

    const backLane = screen.getByTestId('battle-lane-you-back');
    const frontLane = screen.getByTestId('battle-lane-you-front');

    // Duplicate Tamers cluster in back lane with stack badge and behind card
    expect(backLane.querySelector('[data-testid="tamer-stack-badge-0"]')).toHaveTextContent('×2');
    expect(backLane.querySelectorAll('.hs-dup-stack__behind')).toHaveLength(1);

    // Digivolve soft highlight remains on front-row Digimon
    expect(frontLane.querySelector('[data-testid="field-slot-p1-2"]')).toHaveClass(
      'ib-drop-slot--digivolve-soft',
    );

    // Click and right-click inspect work on back-row stacked Tamer and front-row Digimon
    fireEvent.click(backLane.querySelector('[data-testid="field-slot-p1-0"]')!);
    expect(onSlotClick).toHaveBeenCalledWith(0);
    fireEvent.contextMenu(frontLane.querySelector('.ib-permanent-slot')!);
    expect(onSlotInspect).toHaveBeenCalledWith(2);

    // Unstack when 2+ duplicate copies are independently targetable
    rerender(
      <BattleArea
        permanents={[tamer1, tamer2, digimon]}
        isOpponent={false}
        highlightedSlots={new Set([0, 1])}
      />,
    );
    expect(container.querySelector('[data-testid="tamer-stack-badge-0"]')).toBeNull();
    expect(backLane.querySelector('[data-testid="field-slot-p1-0"]')).toBeInTheDocument();
    expect(backLane.querySelector('[data-testid="field-slot-p1-1"]')).toBeInTheDocument();
  });

  it('renders a single combined row without lane wrappers when battleLanes is one', () => {
    useUiStore.getState().setBattleLanes('one');

    const digimon = makeDigimon();
    const tamer = makeTamer();
    render(<BattleArea permanents={[digimon, tamer]} isOpponent={false} />);

    const area = screen.getByTestId('battle-area-you');
    expect(area).toHaveClass('hs-battle-area');
    expect(area).not.toHaveClass('hs-battle-area--laned');
    expect(area).toHaveAttribute('data-battle-lanes', 'one');
    expect(screen.queryByTestId('battle-lane-you-front')).toBeNull();
    expect(screen.queryByTestId('battle-lane-you-back')).toBeNull();
    expect(screen.getByTestId('field-slot-p1-0')).toBeInTheDocument();
    expect(screen.getByTestId('field-slot-p1-1')).toBeInTheDocument();
  });
});
