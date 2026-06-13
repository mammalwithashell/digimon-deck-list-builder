import '@testing-library/jest-dom/vitest';
import { render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { UpdateInfo } from '@/updater/types';

// Tauri IPC surfaces the updater hook depends on. The bare module paths
// are mocked so the hook's invoke/check calls resolve in jsdom.
const invoke = vi.fn();
const check = vi.fn();
const relaunch = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock('@tauri-apps/plugin-updater', () => ({ check: (...a: unknown[]) => check(...a) }));
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: (...a: unknown[]) => relaunch(...a) }));

const availableInfo: UpdateInfo = {
  current_version: '0.1.0',
  latest_version: '0.2.2',
  min_version: '0.2.0',
  notes: 'fixes',
  update_available: true,
  update_required: false,
  download_url: 'https://example/win-setup.exe',
  download_page_url: 'https://example/download',
};

describe('UpdaterStatusCard', () => {
  beforeEach(() => {
    vi.stubEnv('VITE_BUILD_TARGET', 'desktop');
    (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ = {};
    invoke.mockReset();
    check.mockReset();
    relaunch.mockReset();
  });

  afterEach(() => {
    vi.unstubAllEnvs();
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
  });

  it('surfaces an available update with install + download actions', async () => {
    invoke.mockImplementation((cmd: string) =>
      cmd === 'updater_check_info' ? Promise.resolve(availableInfo) : Promise.resolve(),
    );
    const { UpdaterStatusCard } = await import('./UpdaterStatusCard');
    render(<UpdaterStatusCard />);

    expect(await screen.findByText('Update available')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /install & restart/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^download$/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /check for updates/i })).toBeInTheDocument();
  });

  it('surfaces a detection error with a manual-download fallback (no install button)', async () => {
    invoke.mockImplementation((cmd: string) =>
      cmd === 'updater_check_info'
        ? Promise.reject(new Error('manifest HTTP 500'))
        : Promise.resolve(),
    );
    const { UpdaterStatusCard } = await import('./UpdaterStatusCard');
    render(<UpdaterStatusCard />);

    expect(await screen.findByText('Update check failed')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^download$/i })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /install & restart/i })).not.toBeInTheDocument();
  });

  it('renders nothing outside the Tauri runtime', async () => {
    delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__;
    const { UpdaterStatusCard } = await import('./UpdaterStatusCard');
    const { container } = render(<UpdaterStatusCard />);
    expect(container).toBeEmptyDOMElement();
  });
});
