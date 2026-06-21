import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';

// Stub the heavy builder: expose the props the overlay passes so we can assert
// wiring and drive the close/save callbacks without mounting the real builder.
vi.mock('@/pages/DeckBuilderPage', () => ({
  DeckBuilderWorkbench: (props: {
    embedded?: boolean;
    initialDeckId?: string | null;
    onClose?: () => void;
    onSaved?: (id: string) => void;
  }) => (
    <div
      data-testid="workbench"
      data-embedded={String(props.embedded)}
      data-deckid={props.initialDeckId ?? ''}
    >
      <button type="button" onClick={() => props.onClose?.()}>stub-close</button>
      <button type="button" onClick={() => props.onSaved?.('saved-9')}>stub-save</button>
    </div>
  ),
}));

import { DeckEditOverlay } from './DeckEditOverlay';

describe('DeckEditOverlay', () => {
  beforeEach(() => {
    useDeckBuilderStore.setState({ isDirty: false });
    vi.restoreAllMocks();
  });

  it('renders nothing when closed', () => {
    const { container } = render(
      <DeckEditOverlay isOpen={false} deckId="d1" onClose={vi.fn()} onSaved={vi.fn()} />,
    );
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByTestId('workbench')).toBeNull();
  });

  it('renders nothing when open without a deck id', () => {
    render(<DeckEditOverlay isOpen deckId={null} onClose={vi.fn()} onSaved={vi.fn()} />);
    expect(screen.queryByTestId('workbench')).toBeNull();
  });

  it('renders the embedded workbench for the deck when open', () => {
    render(<DeckEditOverlay isOpen deckId="d1" onClose={vi.fn()} onSaved={vi.fn()} />);
    const wb = screen.getByTestId('workbench');
    expect(wb).toHaveAttribute('data-embedded', 'true');
    expect(wb).toHaveAttribute('data-deckid', 'd1');
  });

  it('closes immediately when there are no unsaved changes', () => {
    const onClose = vi.fn();
    render(<DeckEditOverlay isOpen deckId="d1" onClose={onClose} onSaved={vi.fn()} />);
    fireEvent.click(screen.getByText('stub-close'));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('guards close when dirty: blocks on cancel, proceeds on confirm', () => {
    useDeckBuilderStore.setState({ isDirty: true });
    const onClose = vi.fn();
    render(<DeckEditOverlay isOpen deckId="d1" onClose={onClose} onSaved={vi.fn()} />);

    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(false);
    fireEvent.click(screen.getByText('stub-close'));
    expect(onClose).not.toHaveBeenCalled();

    confirmSpy.mockReturnValue(true);
    fireEvent.click(screen.getByText('stub-close'));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('passes the saved id straight through onSaved (no dirty guard)', () => {
    useDeckBuilderStore.setState({ isDirty: true });
    const onSaved = vi.fn();
    render(<DeckEditOverlay isOpen deckId="d1" onClose={vi.fn()} onSaved={onSaved} />);
    fireEvent.click(screen.getByText('stub-save'));
    expect(onSaved).toHaveBeenCalledWith('saved-9');
  });
});
