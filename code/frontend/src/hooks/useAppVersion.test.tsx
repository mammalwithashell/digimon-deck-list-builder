import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useAppVersion } from './useAppVersion';

const getVersion = vi.fn();
vi.mock('@tauri-apps/api/app', () => ({ getVersion: (...a: unknown[]) => getVersion(...a) }));

function Probe() {
  return <div data-testid="v">{useAppVersion()}</div>;
}

describe('useAppVersion', () => {
  beforeEach(() => {
    getVersion.mockReset();
  });
  afterEach(() => {
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it('uses the Tauri runtime version on desktop', async () => {
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    getVersion.mockResolvedValue('0.2.2');
    render(<Probe />);
    expect(await screen.findByText('0.2.2')).toBeInTheDocument();
  });

  it('falls back to the build-time version outside the Tauri runtime', () => {
    render(<Probe />);
    // No __TAURI_INTERNALS__ → runtime call is skipped; shows the build value.
    expect(screen.getByTestId('v')).toHaveTextContent('0.0.0');
    expect(getVersion).not.toHaveBeenCalled();
  });
});
