import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { TrashViewer } from './TrashViewer';

describe('TrashViewer', () => {
  it('right-clicking a trash card inspects it and suppresses the browser menu', () => {
    const onInspect = vi.fn();
    render(
      <TrashViewer
        isOpen
        onClose={() => {}}
        trashIds={['ST2-02', 'ST2-13']}
        ownerLabel="Player 1"
        onInspect={onInspect}
      />,
    );
    const card = screen.getByTitle('ST2-13');
    // contextMenu returns false when a handler called preventDefault().
    const notPrevented = fireEvent.contextMenu(card);
    expect(onInspect).toHaveBeenCalledTimes(1);
    expect(onInspect).toHaveBeenCalledWith('ST2-13');
    expect(notPrevented).toBe(false);
  });

  it('does not bind a context handler (and does not throw) when onInspect is absent', () => {
    render(<TrashViewer isOpen onClose={() => {}} trashIds={['ST2-02']} ownerLabel="P" />);
    const card = screen.getByTitle('ST2-02');
    expect(() => fireEvent.contextMenu(card)).not.toThrow();
    expect(card).toBeInTheDocument();
  });
});
