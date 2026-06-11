import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useState } from 'react';
import { CardOverlay } from './CardOverlay';
import { CardDetailOverlay } from './CardDetailOverlay';
import type { PermanentInfo, SourceInfo } from '@/types/game';

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

function makeSource(overrides: Partial<SourceInfo> = {}): SourceInfo {
  return {
    cardId: 'ST1-02',
    cardName: 'Koromon',
    isTop: false,
    optState: 0,
    dpContribution: 0,
    mainEffectText: '',
    inheritedEffectText: '[Your Turn] This Digimon gets +1000 DP.',
    colors: [0],
    ...overrides,
  };
}

describe('CardOverlay right-click inspect (DCGO CardInfo -> CardDetail)', () => {
  it('right-clicking the top card image opens the enlarged inspect without closing the panel', () => {
    const onClose = vi.fn();
    const onInspectCard = vi.fn();
    render(<CardOverlay permanent={makePerm()} onClose={onClose} onInspectCard={onInspectCard} />);
    const evt = fireEvent.contextMenu(screen.getByTitle('BT24-018'));
    expect(onInspectCard).toHaveBeenCalledWith('BT24-018');
    expect(onClose).not.toHaveBeenCalled();
    expect(evt).toBe(false); // native context menu suppressed
  });

  it('right-clicking a source card image opens that source enlarged', () => {
    const onInspectCard = vi.fn();
    render(
      <CardOverlay
        permanent={makePerm({ sources: [makeSource()] })}
        onClose={vi.fn()}
        onInspectCard={onInspectCard}
      />,
    );
    fireEvent.contextMenu(screen.getByTitle('ST1-02'));
    expect(onInspectCard).toHaveBeenCalledWith('ST1-02');
  });

  it('source entries show the source card inherited effect text', () => {
    render(
      <CardOverlay permanent={makePerm({ sources: [makeSource()] })} onClose={vi.fn()} />,
    );
    expect(screen.getByText('[Your Turn] This Digimon gets +1000 DP.')).toBeInTheDocument();
  });
});

describe('Escape layering between stack inspector and enlarged detail', () => {
  function Harness({ onInspectorClose }: { onInspectorClose: () => void }) {
    const [detail, setDetail] = useState<string | null>('BT24-018');
    return (
      <>
        <CardOverlay permanent={makePerm()} onClose={onInspectorClose} />
        <CardDetailOverlay cardId={detail} onClose={() => setDetail(null)} />
      </>
    );
  }

  it('Escape closes only the enlarged detail when both are open', () => {
    const onInspectorClose = vi.fn();
    render(<Harness onInspectorClose={onInspectorClose} />);
    expect(screen.getByTestId('card-detail-overlay')).toBeInTheDocument();

    fireEvent.keyDown(window, { key: 'Escape' });
    // Detail closed, inspector untouched.
    expect(screen.queryByTestId('card-detail-overlay')).not.toBeInTheDocument();
    expect(onInspectorClose).not.toHaveBeenCalled();

    // A second Escape (detail now closed) closes the inspector.
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onInspectorClose).toHaveBeenCalledTimes(1);
  });
});
