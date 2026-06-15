import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { ActionBar } from './ActionBar';
import { GamePhase, type DecodedAction } from '@/types/game';

const ZERO_MASK = new Array(2192).fill(0);

function eff(over: Partial<DecodedAction> = {}): DecodedAction {
  return {
    actionId: 1012,
    playerId: 0,
    phase: 'Main',
    kind: 'field_effect',
    label: 'Activate CresGarurumon: Digi-Burst 2',
    sourceZone: 'battle',
    sourceIndex: 1,
    targetZone: null,
    targetIndex: null,
    cardId: 'ST6-13',
    cardName: 'CresGarurumon',
    effectName:
      '[Main] Digi-Burst 2: play 1 purple level 3 Digimon from trash free',
    ...over,
  };
}

function baseProps(
  over: Partial<Parameters<typeof ActionBar>[0]> = {},
): Parameters<typeof ActionBar>[0] {
  return {
    phase: GamePhase.Main,
    actionMask: ZERO_MASK,
    onAction: vi.fn(),
    isGameOver: false,
    ...over,
  };
}

describe('ActionBar activatable effects (engine-decoded)', () => {
  it('names a field [Main] effect by card + short effect tag', () => {
    render(<ActionBar {...baseProps({ decodedActions: [eff()] })} />);
    expect(
      screen.getByText('CresGarurumon: Digi-Burst 2'),
    ).toBeInTheDocument();
  });

  it('submits the decoded action id on click', () => {
    const onAction = vi.fn();
    render(
      <ActionBar
        {...baseProps({ decodedActions: [eff({ actionId: 1012 })], onAction })}
      />,
    );
    fireEvent.click(screen.getByTestId('action-effect-1012'));
    expect(onAction).toHaveBeenCalledWith(1012);
  });

  it('disambiguates duplicate card names with a slot, unique names without', () => {
    render(
      <ActionBar
        {...baseProps({
          decodedActions: [
            eff({ actionId: 1012, sourceIndex: 1 }),
            eff({ actionId: 1042, sourceIndex: 4 }),
          ],
        })}
      />,
    );
    expect(
      screen.getByText('CresGarurumon (slot 1): Digi-Burst 2'),
    ).toBeInTheDocument();
    expect(
      screen.getByText('CresGarurumon (slot 4): Digi-Burst 2'),
    ).toBeInTheDocument();
  });

  it('falls back to the card name when the effect has no usable tag', () => {
    render(
      <ActionBar {...baseProps({ decodedActions: [eff({ effectName: null })] })} />,
    );
    expect(screen.getByText('CresGarurumon')).toBeInTheDocument();
  });

  it('renders a trash [Main] effect by card name, not a phantom battle slot', () => {
    render(
      <ActionBar
        {...baseProps({
          decodedActions: [
            eff({
              actionId: 1150,
              kind: 'trash_effect',
              sourceZone: 'trash',
              sourceIndex: 0,
              cardId: 'ST6-02',
              cardName: 'DemiDevimon',
              effectName: '[Main] Return this card to hand',
            }),
          ],
        })}
      />,
    );
    // Engine action id 1150 round-trips; named by the trash card.
    expect(screen.getByTestId('action-effect-1150')).toBeInTheDocument();
    expect(screen.getByText('DemiDevimon: Return this card to hand')).toBeInTheDocument();
    // The old broken decode ("Effect 15:0") must never appear.
    expect(screen.queryByText(/Effect 15:0/)).not.toBeInTheDocument();
  });

  it('falls back to mask-derived rendering when no decoded list is provided', () => {
    const canActivateEffect = new Map<number, Set<number>>([[1, new Set([2])]]);
    render(<ActionBar {...baseProps({ canActivateEffect })} />);
    expect(screen.getByTestId('action-effect-1-2')).toBeInTheDocument();
  });
});
