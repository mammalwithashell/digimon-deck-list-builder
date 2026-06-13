import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import type { UpdateInfo, UpdaterStatus } from './types';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

/** The desktop bundle can be loaded in a plain browser (dev workflow:
 *  `npm run dev:desktop` + Playwright). There the Tauri IPC hooks aren't
 *  injected, so `invoke()`/plugin calls would throw. Detect via the
 *  Tauri-injected global and treat the updater as inert when absent. */
function inTauriRuntime(): boolean {
  if (typeof window === 'undefined') return false;
  return Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}

export interface UpdaterController {
  status: UpdaterStatus;
  info: UpdateInfo | null;
  error: string | null;
  installing: boolean;
  /** True when running inside the Tauri desktop runtime (updater live). */
  active: boolean;
  /** Re-run detection (pull-based — no event race). */
  refresh: () => Promise<void>;
  /** Download + install in-app via the updater plugin, then relaunch. */
  installInApp: () => Promise<void>;
  /** Open the canonical download page in the default browser (fallback). */
  openDownloadPage: () => Promise<void>;
}

/** Single source of truth for updater UI. Detection is pull-based:
 *  it `invoke`s the Rust `updater_check_info` command, so there is no
 *  emit-before-listener race. The in-app install still uses the updater
 *  plugin; failures are captured in `error` and the browser fallback
 *  ([`openDownloadPage`]) is always available. */
export function useUpdater(): UpdaterController {
  const active = IS_DESKTOP && inTauriRuntime();
  const [status, setStatus] = useState<UpdaterStatus>(active ? 'checking' : 'idle');
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const mounted = useRef(true);

  const refresh = useCallback(async () => {
    if (!active) return;
    setStatus('checking');
    setError(null);
    try {
      const next = await invoke<UpdateInfo>('updater_check_info');
      if (!mounted.current) return;
      setInfo(next);
      setStatus(
        next.update_required ? 'forced' : next.update_available ? 'available' : 'uptodate',
      );
    } catch (err) {
      if (!mounted.current) return;
      setError(String(err));
      setStatus('error');
    }
  }, [active]);

  const installInApp = useCallback(async () => {
    if (!active) return;
    setInstalling(true);
    setError(null);
    try {
      const update = await check();
      if (!update) {
        if (!mounted.current) return;
        // Detection said an update exists but the plugin found none —
        // surface it and steer the user to the manual download.
        setError('The updater service found no installable update.');
        setStatus('error');
        setInstalling(false);
        return;
      }
      await update.downloadAndInstall();
      await relaunch();
    } catch (err) {
      if (!mounted.current) return;
      setError(`In-app update failed: ${String(err)}`);
      setStatus('error');
      setInstalling(false);
    }
  }, [active]);

  const openDownloadPage = useCallback(async () => {
    if (!active) return;
    try {
      await invoke('updater_open_download_page');
    } catch (err) {
      if (!mounted.current) return;
      setError(`Couldn't open the download page: ${String(err)}`);
    }
  }, [active]);

  useEffect(() => {
    mounted.current = true;
    void refresh();
    return () => {
      mounted.current = false;
    };
  }, [refresh]);

  return { status, info, error, installing, active, refresh, installInApp, openDownloadPage };
}
