import '@testing-library/jest-dom/vitest';
import { render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { CardOverlay } from './CardOverlay';
import type { PermanentInfo } from '@/types/game';

function makePerm(overrides: Partial<PermanentInfo> = {}): PermanentInfo {
  return {
    topCardId: 'BT24-018',
    topCardName: 'Styracomon',
    dp: 4000,
    level: 7,
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
    dpBreakdown: { base: 4000, sources: [], temporary: 0, total: 4000 },
    turnPlayed: 1,
    colors: [0],
    ...overrides,
  };
}

describe('CardOverlay active-modifier section', () => {
  it('renders grouped, labelled modifiers from structured data', () => {
    const perm = makePerm({
      modifiers: [
        { type: 'CannotBeDestroyed', value: 0, expiry: 'Permanent', sourceCardId: null },
        { type: 'ChangeDp', value: 3000, expiry: 'EndOfTurn', sourceCardId: 'BT24-018' },
        { type: 'CannotSuspend', value: 0, expiry: 'Permanent', sourceCardId: null },
      ],
    });
    render(<CardOverlay permanent={perm} onClose={vi.fn()} />);

    const section = screen.getByTestId('active-modifiers');
    expect(within(section).getByText('Cannot be deleted')).toBeInTheDocument();
    expect(within(section).getByText('DP +3,000')).toBeInTheDocument();
    expect(within(section).getByText("Can't suspend")).toBeInTheDocument();
    // Non-permanent expiry shows a hint.
    expect(within(section).getByText('(until end of turn)')).toBeInTheDocument();
  });

  it('tolerates an unmapped modifier type under a humanized Other label', () => {
    const perm = makePerm({
      modifiers: [
        { type: 'SomeFutureModifier', value: 0, expiry: 'Permanent', sourceCardId: null },
      ],
    });
    render(<CardOverlay permanent={perm} onClose={vi.fn()} />);
    const section = screen.getByTestId('active-modifiers');
    expect(within(section).getByText('Some Future Modifier')).toBeInTheDocument();
  });

  it('omits the section entirely when there are no modifiers', () => {
    render(<CardOverlay permanent={makePerm()} onClose={vi.fn()} />);
    expect(screen.queryByTestId('active-modifiers')).not.toBeInTheDocument();
  });
});
