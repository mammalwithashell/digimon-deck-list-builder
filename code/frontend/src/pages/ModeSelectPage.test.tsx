import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

const navigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useNavigate: () => navigate };
});
vi.mock('@/features/play/playApi', () => ({ listFormats: async () => [] }));
// loadPlayFormats() pulls listFormats from @/api/deckApi — mock the real source
// so the page never fires a network XHR (which jsdom can't complete, leaving a
// dangling socket that hangs worker teardown).
vi.mock('@/api/deckApi', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/deckApi')>()),
  listFormats: async () => [],
}));

import { ModeSelectPage } from './ModeSelectPage';
import { usePlayFlowStore } from '@/features/play/playFlowStore';
import { useThemeStore, type Theme } from '@/design/theme/themeStore';

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

  it.each<Theme>(['dark', 'light'])(
    'frames the opponent and format choosers as titled windows (%s)',
    async (theme) => {
      useThemeStore.getState().setTheme(theme);
      render(
        <MemoryRouter>
          <ModeSelectPage />
        </MemoryRouter>,
      );
      // Wait for the page (and its default format) to settle.
      await screen.findByRole('region', { name: 'Formats' });
      const titles = Array.from(
        document.querySelectorAll('.ds-window__title-text'),
      ).map((el) => el.textContent);
      expect(titles).toContain('OPPONENT');
      expect(titles).toContain('FORMAT');
    },
  );
});
