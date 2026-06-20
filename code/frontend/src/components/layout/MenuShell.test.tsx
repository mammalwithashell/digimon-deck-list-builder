import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { MemoryRouter, Routes, Route } from 'react-router-dom';

vi.mock('@/storage/deckStore', () => ({ listDecks: async () => [] }));

import { MenuShell } from './MenuShell';
import { useUiStore } from '@/stores/uiStore';

function renderAt(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route element={<MenuShell />}>
          <Route path="/" element={<div>HOME CONTENT</div>} />
          <Route path="/play" element={<div>PLAY CONTENT</div>} />
        </Route>
      </Routes>
    </MemoryRouter>,
  );
}

beforeEach(() => {
  localStorage.clear();
  useUiStore.getState().setRailCollapsed(false);
});

describe('MenuShell', () => {
  it('renders the rail and the routed outlet content', () => {
    renderAt('/play');
    expect(screen.getByText('PLAY CONTENT')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Play' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Decks/ })).toBeInTheDocument();
  });

  it('marks the active item from the route, not always Home', () => {
    renderAt('/play');
    const current = screen.getByRole('button', { current: 'page' });
    expect(current).toHaveTextContent('Play');
  });

  it('collapses the rail and persists the choice', () => {
    const { container } = renderAt('/');
    const shell = container.querySelector('.menu-shell');
    expect(shell).toHaveAttribute('data-collapsed', 'false');
    fireEvent.click(screen.getByRole('button', { name: /Collapse navigation/i }));
    expect(shell).toHaveAttribute('data-collapsed', 'true');
    expect(localStorage.getItem('desktop.railCollapsed')).toBe('true');
  });
});
