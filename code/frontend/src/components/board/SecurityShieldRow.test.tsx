import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SecurityShieldRow } from './SecurityShieldRow';

describe('SecurityShieldRow security stack & face-up display', () => {
  it('renders sleeve card backs for face-down security tiles', () => {
    render(
      <SecurityShieldRow
        count={3}
        isOpponent={false}
        securityIds={[null, null, null]}
        securityFaceUp={[false, false, false]}
      />,
    );

    expect(screen.getByTestId('security-facedown-card-0')).toBeInTheDocument();
    expect(screen.getByTestId('security-facedown-card-2')).toBeInTheDocument();
    expect(screen.getAllByTestId('card-back-sleeve')).toHaveLength(3);
  });

  it('renders face-up security cards directly in the dock with eye & [Security] badge and inspects on click', () => {
    const onInspectCard = vi.fn();
    render(
      <SecurityShieldRow
        count={5}
        isOpponent={false}
        securityIds={['EX12-069', null, null, null, null]}
        securityFaceUp={[true, false, false, false, false]}
        onInspectCard={onInspectCard}
      />,
    );

    const directCard = screen.getByTestId('security-faceup-card-0');
    expect(directCard).toHaveTextContent('[Security]');
    expect(directCard).toHaveAttribute(
      'title',
      'Face-up security #1: EX12-069 [Security] — Click to inspect',
    );
    fireEvent.click(directCard);
    expect(onInspectCard).toHaveBeenCalledWith('EX12-069');

    const eyeBtn = screen.getByTestId('own-security-eye');
    expect(eyeBtn).toHaveTextContent('Face-Up: EX12-069');
    fireEvent.click(eyeBtn);
    expect(onInspectCard).toHaveBeenCalledTimes(2);
  });

  it('opens a face-up card popover when multiple opponent security cards are face up', () => {
    const onInspectCard = vi.fn();
    render(
      <SecurityShieldRow
        count={3}
        isOpponent
        securityIds={['EX12-069', null, 'BT24-010']}
        securityFaceUp={[true, false, true]}
        onInspectCard={onInspectCard}
      />,
    );

    expect(screen.getByTestId('security-faceup-card-0')).toBeInTheDocument();
    expect(screen.getByTestId('security-facedown-card-1')).toBeInTheDocument();
    expect(screen.getByTestId('security-faceup-card-2')).toBeInTheDocument();

    const eyeBtn = screen.getByTestId('opp-security-eye');
    expect(eyeBtn).toHaveTextContent('2 Face-Up');
    fireEvent.click(eyeBtn);

    const popover = screen.getByTestId('faceup-security-popover');
    expect(popover).toBeInTheDocument();
    fireEvent.click(within(popover).getByTitle('BT24-010'));
    expect(onInspectCard).toHaveBeenCalledWith('BT24-010');
  });

  it('shows targetable highlight and attack CTA when isAttackTarget is true', () => {
    const onAttackSecurity = vi.fn();
    const { rerender } = render(
      <SecurityShieldRow
        count={2}
        isOpponent
        isAttackTarget
        onAttackSecurity={onAttackSecurity}
      />,
    );

    const target = screen.getByTestId('security-attack-target');
    expect(target).toHaveClass('is-attack-target');
    expect(screen.getByTestId('security-attack-cta')).toHaveTextContent('Attack Security');
    fireEvent.click(target);
    expect(onAttackSecurity).toHaveBeenCalledTimes(1);

    rerender(
      <SecurityShieldRow
        count={0}
        isOpponent
        isAttackTarget
        onAttackSecurity={onAttackSecurity}
      />,
    );
    expect(screen.getByTestId('security-attack-cta')).toHaveTextContent('Attack Player');
  });
});
