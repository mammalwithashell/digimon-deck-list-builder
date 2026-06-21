import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { useDeckBuilderStore } from '@/stores/deckBuilderStore';

// The builder fires several data effects on mount; stub them to no-op resolves
// so the component renders without real network calls.
vi.mock('@/api/deckApi', () => ({
  listFormats: vi.fn(async () => []),
  cardLegalityBulk: vi.fn(async () => ({})),
  listTestedCards: vi.fn(async () => new Set<string>()),
  listCardDatabase: vi.fn(async () => []),
  validateDeckRaw: vi.fn(async () => ({ valid: true, errors: [], warnings: [] })),
}));
vi.mock('@/api/digimonCardApi', () => ({
  searchCards: vi.fn(async () => []),
  getCardById: vi.fn(async () => null),
}));

const saveBuilderDeck = vi.fn(async () => ({ id: 'saved-x' }));
vi.mock('@/features/deck-builder/deckBuilderAdapter', () => ({
  getBuilderDeck: vi.fn(async () => ({ id: 'd', name: 'D', main_deck: [], egg_deck: [] })),
  saveBuilderDeck: () => saveBuilderDeck(),
}));

import { DeckBuilderWorkbench } from './DeckBuilderPage';

function renderEmbedded(props: Partial<Parameters<typeof DeckBuilderWorkbench>[0]> = {}) {
  return render(
    <MemoryRouter>
      <DeckBuilderWorkbench embedded initialDeckId={null} onClose={vi.fn()} onSaved={vi.fn()} {...props} />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  saveBuilderDeck.mockClear();
  useDeckBuilderStore.getState().clearDeck();
});

describe('DeckBuilderWorkbench embedded mode', () => {
  it('shows overlay chrome (CANCEL) instead of page navigation (HOME/LIBRARY/QUIT)', () => {
    renderEmbedded();
    expect(screen.getByRole('button', { name: '← CANCEL' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'DONE' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'HOME' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'LIBRARY' })).toBeNull();
    expect(screen.queryByRole('button', { name: 'QUIT' })).toBeNull();
  });

  it('saving invokes onSaved with the persisted id instead of navigating', async () => {
    const onSaved = vi.fn();
    renderEmbedded({ onSaved });
    // SAVE is gated on isDirty; mark dirty and wait for the re-render to enable it
    // before clicking (clicking a disabled button is a no-op).
    useDeckBuilderStore.setState({ isDirty: true });
    await waitFor(() => expect(screen.getByRole('button', { name: 'SAVE' })).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'SAVE' }));
    await waitFor(() => expect(onSaved).toHaveBeenCalledWith('saved-x'));
    expect(saveBuilderDeck).toHaveBeenCalledTimes(1);
  });
});
