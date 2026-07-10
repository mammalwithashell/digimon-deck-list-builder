import '@testing-library/jest-dom/vitest';
import { render, fireEvent, createEvent } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { PermanentSlot } from './PermanentSlot';
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
