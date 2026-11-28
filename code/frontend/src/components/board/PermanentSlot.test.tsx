import '@testing-library/jest-dom/vitest';
import { render, fireEvent, createEvent, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { PermanentSlot, isNonDigimonPermanent, resolveEffectiveDp } from './PermanentSlot';
import { useGameStore } from '@/stores/gameStore';
import type { PermanentInfo } from '@/types/game';

function makePerm(overrides: Partial<PermanentInfo> = {}): PermanentInfo {
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
    turnPlayed: 0,
    colors: [0],
    ...overrides,
  };
}

describe('PermanentSlot right-click inspect', () => {
  it('fires onInspect, prevents the default browser menu, and does not click', () => {
    const onInspect = vi.fn();
    const onClick = vi.fn();
    const { container } = render(
      <PermanentSlot
        perm={makePerm()}
        slotIndex={0}
        isOpponent={false}
        onClick={onClick}
        onInspect={onInspect}
      />,
    );
    const slot = container.querySelector('.ib-permanent-slot');
    expect(slot).toBeTruthy();

    const evt = createEvent.contextMenu(slot!);
    fireEvent(slot!, evt);

    expect(onInspect).toHaveBeenCalledTimes(1);
    expect(evt.defaultPrevented).toBe(true);
    // Right-click must not trigger the play/attack/select left-click path.
    expect(onClick).not.toHaveBeenCalled();
  });

  it('does not bind a context handler when onInspect is absent', () => {
    const onClick = vi.fn();
    const { container } = render(
      <PermanentSlot perm={makePerm()} slotIndex={0} isOpponent onClick={onClick} />,
    );
    const slot = container.querySelector('.ib-permanent-slot');
    const evt = createEvent.contextMenu(slot!);
    fireEvent(slot!, evt);
    // No inspect handler → default not prevented, no click fired.
    expect(evt.defaultPrevented).toBe(false);
    expect(onClick).not.toHaveBeenCalled();
  });
});

describe('PermanentSlot digivolution-card count badge', () => {
  // `sourceCount` is the number of DIGIVOLUTION CARDS (cards under the top
  // card) — the top card itself is not a digivolution card. A Digimon with
  // 2 sources must show "x2", not "x3" (the old badge displayed the total
  // stack size).
  it('shows the digivolution-card count, excluding the top card', () => {
    const { container } = render(
      <PermanentSlot
        perm={makePerm({ sourceCount: 2 })}
        slotIndex={0}
        isOpponent={false}
      />,
    );
    const badge = container.querySelector('.ib-source-count');
    expect(badge).toBeInTheDocument();
    expect(badge).toHaveTextContent('x2');
  });

  it('shows the badge for a single digivolution card', () => {
    const { container } = render(
      <PermanentSlot
        perm={makePerm({ sourceCount: 1 })}
        slotIndex={0}
        isOpponent={false}
      />,
    );
    expect(container.querySelector('.ib-source-count')).toHaveTextContent('x1');
  });

  it('hides the badge when there are no digivolution cards', () => {
    const { container } = render(
      <PermanentSlot
        perm={makePerm({ sourceCount: 0 })}
        slotIndex={0}
        isOpponent={false}
      />,
    );
    expect(container.querySelector('.ib-source-count')).toBeNull();
  });
});

describe('PermanentSlot "choose N" selected badge', () => {
  it('renders the selected check badge only when selected', () => {
    const { queryByTestId, rerender } = render(
      <PermanentSlot perm={makePerm()} slotIndex={0} isOpponent={false} />,
    );
    expect(queryByTestId('permanent-selected-badge')).toBeNull();

    rerender(
      <PermanentSlot perm={makePerm()} slotIndex={0} isOpponent={false} selected />,
    );
    expect(queryByTestId('permanent-selected-badge')).toBeInTheDocument();
  });
});

describe('PermanentSlot live DP and Tamer/Option rules', () => {
  it('resolves live effective DP from dpBreakdown.total when base dp lags', () => {
    const buffed = makePerm({
      dp: 6000,
      dpBreakdown: { base: 6000, sources: [], temporary: 3000, total: 9000 },
    });
    expect(resolveEffectiveDp(buffed)).toBe(9000);
    expect(isNonDigimonPermanent(buffed)).toBe(false);
  });

  it('classifies Tamers & Delay Options as non-Digimon so DP and SA-1 stay hidden', () => {
    const tamer = makePerm({
      topCardId: 'AD1-022',
      topCardName: 'Izzy Izumi & Tai Kamiya',
      dp: 0,
      level: 0,
      securityAttackModifier: -1,
      dpBreakdown: { base: 0, sources: [], temporary: 0, total: 0 },
    });
    expect(isNonDigimonPermanent(tamer)).toBe(true);

    const token = makePerm({
      topCardId: 'TOKEN_PAISHU',
      topCardName: 'Paishu Token',
      dp: 6000,
      level: 0,
      dpBreakdown: { base: 6000, sources: [], temporary: 0, total: 6000 },
    });
    expect(isNonDigimonPermanent(token)).toBe(false);
  });
});

describe('PermanentSlot live keywords & status modifier pills', () => {
  it('distinguishes innate vs gained keywords without 6-char slicing', () => {
    render(
      <PermanentSlot
        perm={makePerm({
          keywords: ['armor_purge', 'alliance'],
          keywordBreakdown: { innate: ['armor_purge'], gained: ['alliance'] },
        })}
        slotIndex={0}
        isOpponent={false}
      />,
    );

    const armorPurge = screen.getByTestId('keyword-badge-armor_purge');
    const alliance = screen.getByTestId('keyword-badge-alliance');
    expect(armorPurge).toHaveTextContent('Armor Purge');
    expect(armorPurge).toHaveAttribute('data-keyword-origin', 'innate');
    expect(alliance).toHaveTextContent('Alliance');
    expect(alliance).toHaveAttribute('data-keyword-origin', 'gained');
  });

  it('surfaces live modifier status pills and hides SA modifier pills on non-Digimon permanents', () => {
    const { rerender } = render(
      <PermanentSlot
        perm={makePerm({
          modifiers: [
            { type: 'CannotUnsuspend', value: 0, expiry: 'EndOfYourTurn', sourceCardId: null },
            { type: 'CannotAttack', value: 0, expiry: 'EndOfTurn', sourceCardId: null },
            { type: 'CannotBlock', value: 0, expiry: 'Permanent', sourceCardId: null },
            { type: 'CannotBeDestroyedByEffect', value: 0, expiry: 'EndOfTurn', sourceCardId: null },
            { type: 'CannotBeBlocked', value: 0, expiry: 'EndOfTurn', sourceCardId: null },
            { type: 'SecurityAttackChange', value: 2, expiry: 'EndOfTurn', sourceCardId: null },
          ],
        })}
        slotIndex={0}
        isOpponent={false}
      />,
    );

    const mods = screen.getByTestId('perm-modifiers');
    expect(mods).toHaveTextContent("Won't Unsuspend");
    expect(mods).toHaveTextContent("Can't Attack");
    expect(mods).toHaveTextContent("Can't Block");
    expect(screen.getByTestId('perm-mod-overflow')).toHaveTextContent('+3');
    expect(screen.getByTestId('perm-mod-overflow')).toHaveAttribute(
      'title',
      expect.stringContaining('Protected'),
    );
    expect(screen.getByTestId('perm-mod-overflow')).toHaveAttribute(
      'title',
      expect.stringContaining('Security Attack +2'),
    );

    // Digimon with <=3 modifiers renders SA +2 pill visibly
    rerender(
      <PermanentSlot
        perm={makePerm({
          modifiers: [
            { type: 'SecurityAttackChange', value: 2, expiry: 'EndOfTurn', sourceCardId: null },
          ],
        })}
        slotIndex={0}
        isOpponent={false}
      />,
    );
    expect(screen.getByTestId('perm-modifiers')).toHaveTextContent('SA +2');

    // Non-Digimon Tamer ignores SecurityAttackChange pills
    rerender(
      <PermanentSlot
        perm={makePerm({
          topCardId: 'BT1-085',
          topCardName: 'Tai Kamiya',
          dp: 0,
          level: 0,
          dpBreakdown: { base: 0, sources: [], temporary: 0, total: 0 },
          modifiers: [
            { type: 'SecurityAttackChange', value: -1, expiry: 'Permanent', sourceCardId: null },
            { type: 'ImmunityToOpponentEffects', value: 0, expiry: 'Permanent', sourceCardId: null },
          ],
        })}
        slotIndex={1}
        isOpponent={false}
      />,
    );

    const tamerMods = screen.getByTestId('perm-modifiers');
    expect(tamerMods).toHaveTextContent('Protected');
    expect(tamerMods).not.toHaveTextContent('SA -1');
  });
});

describe('PermanentSlot linked cards & unobstructed clicks', () => {
  it('renders fanned linked card peeks + link badge while preserving main click and right-click inspect', () => {
    const onClick = vi.fn();
    const onInspect = vi.fn();
    render(
      <PermanentSlot
        perm={makePerm({
          linkedCardIds: ['BT21-009', 'EX11-027'],
          linkEffects: [
            { linkIndex: 0, cardId: 'BT21-009', cardName: 'Gatchmon', text: '<Raid>' },
          ],
        })}
        slotIndex={0}
        isOpponent={false}
        onClick={onClick}
        onInspect={onInspect}
      />,
    );

    expect(screen.getByTestId('perm-link-badge')).toHaveTextContent('2');
    const peeks = screen.getAllByTestId('perm-linked-peek');
    expect(peeks).toHaveLength(2);
    expect(peeks[0]).toHaveAttribute('title', expect.stringContaining('Gatchmon (BT21-009)'));

    // Clicking a linked card peek must still bubble to the host slot's click handler
    fireEvent.click(peeks[0]!);
    expect(onClick).toHaveBeenCalledTimes(1);

    // Right-clicking a linked card peek triggers inspect without firing onClick
    const ctxEvt = createEvent.contextMenu(peeks[0]!);
    fireEvent(peeks[0]!, ctxEvt);
    expect(onInspect).toHaveBeenCalledTimes(1);
    expect(onClick).toHaveBeenCalledTimes(1);
  });
});

describe('PermanentSlot highlight, attack, target, and effect-source states', () => {
  it('exposes distinct state classes and badges for highlighted, selected, targeted, canAttack, and effect-source', () => {
    useGameStore.setState({ activeEffectSlot: 2, activeEffectPlayer: 1 });
    const { container, rerender } = render(
      <PermanentSlot
        perm={makePerm()}
        slotIndex={2}
        isOpponent={false}
        highlighted
        canAttack
      />,
    );

    const slot = container.querySelector('.ib-permanent-slot')!;
    expect(slot).toHaveClass('hs-perm--effect-source');
    expect(slot).toHaveClass('hs-perm--highlighted');
    expect(slot).toHaveClass('hs-perm--can-attack');
    expect(screen.getByTestId('perm-effect-source-badge')).toHaveTextContent('EFFECT');

    useGameStore.setState({ activeEffectSlot: null, activeEffectPlayer: null });
    rerender(
      <PermanentSlot
        perm={makePerm()}
        slotIndex={2}
        isOpponent={false}
        canAttack
      />,
    );
    expect(screen.getByTestId('perm-can-attack-badge')).toHaveTextContent('ATK');

    rerender(
      <PermanentSlot
        perm={makePerm()}
        slotIndex={2}
        isOpponent
        targeted
      />,
    );
    expect(container.querySelector('.ib-permanent-slot')).toHaveClass('hs-perm--targeted');
    expect(screen.getByTestId('perm-targeted-badge')).toHaveTextContent('TARGET');
  });
});
