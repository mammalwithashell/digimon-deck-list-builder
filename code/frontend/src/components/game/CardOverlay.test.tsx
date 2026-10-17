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
    // CardEffectText renders the [Your Turn] timing as a badge of its own.
    expect(screen.getByText(/This Digimon gets \+1000 DP\./)).toBeInTheDocument();
  });

  it('linked cards show their link effect text under a Link Effect label', () => {
    const text = '＜Raid＞ (When this Digimon attacks, you may change the attack target to 1 of your '
      + "opponent's unsuspended Digimon with the highest DP.)";
    render(
      <CardOverlay
        permanent={makePerm({
          linkedCardIds: ['BT21-009', 'ST22-08'],
          linkEffects: [{ linkIndex: 0, cardId: 'BT21-009', cardName: 'Gatchmon', text }],
        })}
        onClose={vi.fn()}
      />,
    );
    const linked = screen.getAllByTestId('linked-card');
    expect(linked).toHaveLength(2);
    expect(within(linked[0]!).getByText('Link Effect')).toBeInTheDocument();
    // CardEffectText splits keywords and timings into their own elements.
    expect(linked[0]).toHaveTextContent(/change the attack target to 1 of your opponent's unsuspended/);
    expect(within(linked[1]!).queryByText('Link Effect')).not.toBeInTheDocument();
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

describe('CardOverlay arena-style inspector', () => {
  it('covers the half of the board opposite the inspected card', () => {
    const { rerender } = render(<CardOverlay permanent={makePerm()} onClose={vi.fn()} side="own" />);
    expect(screen.getByRole('dialog')).toHaveAttribute('data-half', 'upper');
    rerender(<CardOverlay permanent={makePerm()} onClose={vi.fn()} side="opponent" />);
    expect(screen.getByRole('dialog')).toHaveAttribute('data-half', 'lower');
  });

  it('shows the DP change from base at a glance, with the breakdown on demand', () => {
    const perm = makePerm({
      dp: 6000,
      dpBreakdown: {
        base: 4000,
        sources: [{ cardId: 'ST1-02', cardName: 'Koromon', value: 1000 }],
        temporary: 1000,
        total: 6000,
      },
    });
    render(<CardOverlay permanent={perm} onClose={vi.fn()} />);
    expect(screen.getByTestId('inspector-dp')).toHaveTextContent('6,000 DP+2,000');
    expect(screen.queryByTestId('dp-breakdown')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /DP breakdown/ }));
    expect(screen.getByTestId('dp-breakdown')).toHaveTextContent('Koromon +1,000');
  });

  it('marks granted keywords', () => {
    const perm = makePerm({
      keywords: ['Blocker', 'Rush'],
      keywordBreakdown: { innate: ['Blocker'], gained: ['Rush'] },
    });
    render(<CardOverlay permanent={perm} onClose={vi.fn()} />);
    expect(screen.getByTitle('Granted by an effect')).toHaveTextContent('Rush');
    expect(screen.getByTitle('Printed')).toHaveTextContent('Blocker');
  });

  it('renders the actions it is given and runs them on click', () => {
    const onAttack = vi.fn();
    const onEffect = vi.fn();
    render(
      <CardOverlay
        permanent={makePerm()}
        onClose={vi.fn()}
        actions={[
          { key: 'attack', label: 'Attack', tone: 'attack', onClick: onAttack },
          { key: 'effect-1004', label: 'Draw 1', title: '[Main] Draw 1', tone: 'effect', onClick: onEffect },
        ]}
      />,
    );
    fireEvent.click(screen.getByTestId('inspector-action-attack'));
    fireEvent.click(screen.getByTestId('inspector-action-effect-1004'));
    expect(onAttack).toHaveBeenCalledOnce();
    expect(onEffect).toHaveBeenCalledOnce();
    expect(screen.getByTestId('inspector-action-effect-1004')).toHaveAttribute('title', '[Main] Draw 1');
  });

  it('closes on a press outside the panel but not inside it', () => {
    const onClose = vi.fn();
    render(
      <>
        <div data-testid="elsewhere" />
        <CardOverlay permanent={makePerm()} onClose={onClose} />
      </>,
    );
    fireEvent.pointerDown(screen.getByRole('dialog'));
    expect(onClose).not.toHaveBeenCalled();
    fireEvent.pointerDown(screen.getByTestId('elsewhere'));
    expect(onClose).toHaveBeenCalledOnce();
  });
});
