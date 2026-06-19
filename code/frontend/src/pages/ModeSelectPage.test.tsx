import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

const navigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useNavigate: () => navigate };
});
vi.mock('@/features/play/playApi', () => ({ listFormats: async () => [] }));

import { ModeSelectPage } from './ModeSelectPage';
import { usePlayFlowStore } from '@/features/play/playFlowStore';

beforeEach(() => {
  navigate.mockClear();
  sessionStorage.clear();
  usePlayFlowStore.getState().reset();
});

describe('ModeSelectPage', () => {
  it('offers an AI Starter Deck tile and routes it to the starter picker', async () => {
    render(
      <MemoryRouter>
        <ModeSelectPage />
      </MemoryRouter>,
    );
    const tile = await screen.findByRole('button', { name: /AI STARTER DECK/i });
    fireEvent.click(tile);
    // Format grid hidden in AI-starter mode.
    expect(screen.queryByRole('region', { name: 'Formats' })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /ENTER/i }));
    await waitFor(() => expect(navigate).toHaveBeenCalledWith('/play/ai-starter'));
  });
});
