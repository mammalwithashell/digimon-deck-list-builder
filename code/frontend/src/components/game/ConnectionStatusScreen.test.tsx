import { describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { ConnectionStatusScreen } from './ConnectionStatusScreen';

describe('ConnectionStatusScreen', () => {
  it('shows a clear failure UI with the error message and a way back on error', () => {
    const onRetry = vi.fn();
    const onLeave = vi.fn();
    render(
      <ConnectionStatusScreen
        state="error"
        error="The game server isn't responding."
        onRetry={onRetry}
        onLeave={onLeave}
      />,
    );

    expect(screen.getByTestId('connection-status-screen')).toBeInTheDocument();
    expect(screen.getByText(/connection failed/i)).toBeInTheDocument();
    expect(screen.getByText("The game server isn't responding.")).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /try again/i }));
    expect(onRetry).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole('button', { name: /back to play/i }));
    expect(onLeave).toHaveBeenCalledTimes(1);
  });

  it('falls back to a generic message when no error detail is provided', () => {
    render(
      <ConnectionStatusScreen
        state="error"
        error={null}
        onRetry={vi.fn()}
        onLeave={vi.fn()}
      />,
    );
    // Some human-readable failure copy is always shown, never a blank panel.
    expect(screen.getByText(/couldn't connect|could not connect|unable to connect/i)).toBeInTheDocument();
  });

  it('shows a connecting state without a retry button or error text', () => {
    render(
      <ConnectionStatusScreen state="connecting" onRetry={vi.fn()} onLeave={vi.fn()} />,
    );
    expect(screen.getByRole('heading', { name: /connecting/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /try again/i })).not.toBeInTheDocument();
    expect(screen.queryByText(/connection failed/i)).not.toBeInTheDocument();
  });

  it('shows a distinct reconnecting state that still offers a way out', () => {
    const onLeave = vi.fn();
    render(
      <ConnectionStatusScreen state="reconnecting" onRetry={vi.fn()} onLeave={onLeave} />,
    );
    expect(screen.getByRole('heading', { name: /reconnecting/i })).toBeInTheDocument();
    // Never a dead end — the player can always leave back to the play screen.
    fireEvent.click(screen.getByRole('button', { name: /back to play/i }));
    expect(onLeave).toHaveBeenCalledTimes(1);
  });
});
