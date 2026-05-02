import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ActionTraceTicker } from './ActionTraceTicker';
import type { ActionTrace } from '@/types/game';

const makeTrace = (overrides: Partial<ActionTrace> = {}): ActionTrace => ({
  actor: 'human',
  playerId: 1,
  actionId: 42,
  decoded: {
    actionId: 42,
    playerId: 1,
    phase: 'Main',
    kind: 'play',
    label: 'Play Agumon',
    sourceZone: 'hand',
    sourceIndex: 0,
    targetZone: 'battle',
    targetIndex: 0,
    cardId: 'BT1-010',
    cardName: 'Agumon',
  },
  tensorSummary: null,
  ...overrides,
});

describe('ActionTraceTicker', () => {
  it('renders the latest decoded action', () => {
    render(
      <ActionTraceTicker
        traces={[
          makeTrace({ decoded: { ...makeTrace().decoded, label: 'Play Agumon' } }),
          makeTrace({ decoded: { ...makeTrace().decoded, label: 'Attack security' } }),
        ]}
      />,
    );

    expect(screen.getByText('Attack security')).toBeInTheDocument();
    expect(screen.queryByText('Play Agumon')).not.toBeInTheDocument();
  });

  it('labels agent_trained as AGENT TRAINED', () => {
    render(<ActionTraceTicker traces={[makeTrace({ actor: 'agent_trained' })]} />);

    expect(screen.getByText('AGENT TRAINED')).toBeInTheDocument();
  });
});
