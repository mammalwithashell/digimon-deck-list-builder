import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import type { DeckSummary } from '@/types/deck';

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useNavigate: () => vi.fn() };
});

const deckA: DeckSummary = {
  id: 'deck-a',
  name: 'Alpha',
  game_mode: 'standard',
  main_count: 50,
  egg_count: 5,
  tags: [],
  meta_archetype: null,
  deck_icon_card_id: null,
} as unknown as DeckSummary;

const listDecks = vi.fn(async (): Promise<DeckSummary[]> => [deckA]);
vi.mock('@/api/deckLibraryAdapter', () => ({
  listDecks: () => listDecks(),
  getDeck: vi.fn(async () => ({ id: 'deck-a', main_deck: [], egg_deck: [] })),
}));

vi.mock('@/features/play/formatCatalog', () => ({
  getPlayFormat: () => ({
    id: 'standard',
    name: 'Standard',
    deckLabel: '50 + 5',
    description: 'Standard format',
  }),
  canUseDeckForFormat: () => ({ ok: true, reason: '' }),
}));
vi.mock('@/features/play/playApi', () => ({ createBotGame: vi.fn() }));
vi.mock('@/api/gameApi', () => ({ normalizeSeedInput: (s: string | null) => s }));

// Stub the overlay so we can assert it is opened with the selected deck without
// mounting the real builder.
vi.mock('@/components/deckbuilder/DeckEditOverlay', () => ({
  DeckEditOverlay: ({
    isOpen,
    deckId,
    onSaved,
  }: {
    isOpen: boolean;
    deckId: string | null;
    onSaved: (id: string) => void;
  }) =>
    isOpen ? (
      <div data-testid="edit-overlay" data-deckid={deckId ?? ''}>
        <button type="button" onClick={() => onSaved(deckId!)}>stub-onSaved</button>
      </div>
    ) : null,
}));

import { DeckSelectPage } from './DeckSelectPage';
import { usePlayFlowStore } from '@/features/play/playFlowStore';

function renderPage() {
  return render(
    <MemoryRouter>
      <DeckSelectPage />
    </MemoryRouter>,
  );
}

beforeEach(() => {
  listDecks.mockClear();
  listDecks.mockResolvedValue([deckA]);
  usePlayFlowStore.setState({ formatId: 'standard', opponentMode: 'bot', deckId: null });
});

describe('DeckSelectPage edit-before-queue', () => {
  it('disables EDIT when no deck is selected', async () => {
    listDecks.mockResolvedValueOnce([]);
    renderPage();
    await waitFor(() => expect(listDecks).toHaveBeenCalled());
    expect(screen.getByRole('button', { name: 'EDIT' })).toBeDisabled();
  });

  it('opens the edit overlay for the selected deck', async () => {
    renderPage();
    await waitFor(() => expect(screen.getByRole('button', { name: 'EDIT' })).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'EDIT' }));
    const overlay = screen.getByTestId('edit-overlay');
    expect(overlay).toHaveAttribute('data-deckid', 'deck-a');
  });

  it('refreshes the deck list and closes the overlay after a save', async () => {
    renderPage();
    await waitFor(() => expect(screen.getByRole('button', { name: 'EDIT' })).toBeEnabled());
    fireEvent.click(screen.getByRole('button', { name: 'EDIT' }));
    expect(listDecks).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByText('stub-onSaved'));
    await waitFor(() => expect(listDecks).toHaveBeenCalledTimes(2));
    expect(screen.queryByTestId('edit-overlay')).toBeNull();
  });
});
