import '@testing-library/jest-dom/vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { CardDetailOverlay } from './CardDetailOverlay';

describe('CardDetailOverlay', () => {
  it('renders nothing when no card is inspected', () => {
    render(<CardDetailOverlay cardId={null} onClose={vi.fn()} />);
    expect(screen.queryByTestId('card-detail-overlay')).not.toBeInTheDocument();
  });

  it('shows the enlarged card image when a card is inspected', () => {
    render(<CardDetailOverlay cardId="BT24-018" onClose={vi.fn()} />);
    expect(screen.getByTestId('card-detail-overlay')).toBeInTheDocument();
    const img = screen.getByTestId('card-detail-image');
    expect(img).toHaveAttribute('src', expect.stringContaining('BT24-018'));
  });

  it('closes on click anywhere', () => {
    const onClose = vi.fn();
    render(<CardDetailOverlay cardId="BT24-018" onClose={onClose} />);
    fireEvent.click(screen.getByTestId('card-detail-overlay'));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('closes on Escape', () => {
    const onClose = vi.fn();
    render(<CardDetailOverlay cardId="BT24-018" onClose={onClose} />);
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('closes on right-click without opening the native context menu', () => {
    const onClose = vi.fn();
    render(<CardDetailOverlay cardId="BT24-018" onClose={onClose} />);
    const evt = fireEvent.contextMenu(screen.getByTestId('card-detail-overlay'));
    expect(onClose).toHaveBeenCalledTimes(1);
    // contextmenu default (native menu) must be prevented
    expect(evt).toBe(false);
  });
});
