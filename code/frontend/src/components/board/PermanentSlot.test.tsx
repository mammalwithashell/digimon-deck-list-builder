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
    sourceCount: 1,
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
