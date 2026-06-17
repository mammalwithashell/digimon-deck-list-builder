import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

const navigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useNavigate: () => navigate };
});

const createAiStarterGame = vi.fn(async (_arg: unknown) => ({ game_id: 'g99', seed: null, aiDeckName: 'X' }));
vi.mock('@/features/play/playApi', async () => {
  const { STARTER_DECKS } = await vi.importActual<typeof import('@/features/play/starterDecks.generated')>(
    '@/features/play/starterDecks.generated',
  );
  return {
    listStarterDecks: async () => STARTER_DECKS,
    createAiStarterGame: (arg: unknown) => createAiStarterGame(arg),
  };
});
vi.mock('@/api/gameApi', () => ({ normalizeSeedInput: (s: string | null) => s }));

import { StarterDeckSelectPage } from './StarterDeckSelectPage';

beforeEach(() => {
  navigate.mockClear();
  createAiStarterGame.mockClear();
});

describe('StarterDeckSelectPage', () => {
  it('lists the 6 starter decks', async () => {
    render(
      <MemoryRouter>
        <StarterDeckSelectPage />
      </MemoryRouter>,
    );
    await waitFor(() => expect(screen.getByText('Starter Deck Gaia Red')).toBeInTheDocument());
    expect(screen.getByText('Starter Deck Cocytus Blue')).toBeInTheDocument();
    expect(screen.getAllByRole('button', { name: /Starter Deck/ })).toHaveLength(6);
  });

  it('launches a game with the selected deck', async () => {
    render(
      <MemoryRouter>
        <StarterDeckSelectPage />
      </MemoryRouter>,
    );
    await waitFor(() => screen.getByText('Starter Deck Cocytus Blue'));
    fireEvent.click(screen.getByRole('button', { name: /Cocytus Blue/ }));
    fireEvent.click(screen.getByRole('button', { name: /FACE THE AI/i }));
    await waitFor(() => expect(createAiStarterGame).toHaveBeenCalledTimes(1));
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    expect((createAiStarterGame.mock.calls[0]![0] as { deck: { name: string } }).deck.name).toBe(
      'Starter Deck Cocytus Blue',
    );
    await waitFor(() => expect(navigate).toHaveBeenCalledWith('/game/g99'));
  });
});
