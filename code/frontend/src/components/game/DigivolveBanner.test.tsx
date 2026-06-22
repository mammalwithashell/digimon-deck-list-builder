import '@testing-library/jest-dom/vitest';
import { render, act } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('@/components/shared/Card', () => ({
  Card: ({ cardId }: { cardId: string }) => (
    <div data-testid="dgv-card">{cardId}</div>
  ),
}));

import { DigivolveBanner } from './DigivolveBanner';
import { useGameStore } from '@/stores/gameStore';
import { useUiStore } from '@/stores/uiStore';

let seq = 0;
function seedDigivolve() {
  seq += 1;
  useGameStore.setState({
    events: [
      {
        type: 'digivolve',
        seq,
        source_card_id: 'BT1-010',
        source_slot: 0,
        player: 1,
        meta: { card_name: 'Greymon' },
      },
    ],
    player1: { battleArea: [{ colors: [0] }] },
    player2: null,
  } as never);
}

beforeEach(() => {
  useGameStore.setState({ events: [] } as never);
});

afterEach(() => {
  vi.useRealTimers();
});

describe('DigivolveBanner motion variants', () => {
  it('plays the full multi-phase cut-in at full motion', () => {
    useUiStore.setState({ motion: 'full' });
    seedDigivolve();
    const { container } = render(<DigivolveBanner />);
    expect(container.querySelector('.dgv-cutin')).toBeInTheDocument();
    expect(container.querySelector('.dgv-cutin__ring')).toBeInTheDocument();
    expect(container.querySelector('.animate-digivolve-banner')).toBeNull();
  });

  it('uses the simple banner at reduced motion', () => {
    useUiStore.setState({ motion: 'reduced' });
    seedDigivolve();
    const { container } = render(<DigivolveBanner />);
    expect(container.querySelector('.animate-digivolve-banner')).toBeInTheDocument();
    expect(container.querySelector('.dgv-cutin')).toBeNull();
  });

  it('shows a static reveal (no animated cut-in) at off motion', () => {
    useUiStore.setState({ motion: 'off' });
    seedDigivolve();
    const { container, getByText } = render(<DigivolveBanner />);
    expect(container.querySelector('.dgv-cutin')).toBeNull();
    expect(container.querySelector('.animate-digivolve-banner')).toBeNull();
    expect(getByText('Digivolve!')).toBeInTheDocument();
  });

  it('renders exactly one cut-in for a single event and is non-interactive', () => {
    useUiStore.setState({ motion: 'full' });
    seedDigivolve();
    const { container } = render(<DigivolveBanner />);
    expect(container.querySelectorAll('.dgv-cutin')).toHaveLength(1);
    // Outer overlay must not intercept board input.
    expect(container.firstElementChild).toHaveClass('pointer-events-none');
  });

  it('auto-dismisses after its show window and clears the timer on unmount', () => {
    vi.useFakeTimers();
    useUiStore.setState({ motion: 'full' });
    seedDigivolve();
    const { container, unmount } = render(<DigivolveBanner />);
    expect(container.querySelector('.dgv-cutin')).toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(1800);
    });
    expect(container.querySelector('.dgv-cutin')).toBeNull();
    expect(() => unmount()).not.toThrow();
  });
});
