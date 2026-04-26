import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import type { ForceUpdatePayload } from './types';

const IS_DESKTOP = import.meta.env.VITE_BUILD_TARGET === 'desktop';

type AvailableUpdate = {
  version: string;
  date: string | null;
  body: string | null;
};

export function UpdaterBridge() {
  const [forced, setForced] = useState<ForceUpdatePayload | null>(null);
  const [available, setAvailable] = useState<AvailableUpdate | null>(null);
  const [installing, setInstalling] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);

  useEffect(() => {
    if (!IS_DESKTOP) return;

    // 1. Listen for Rust-side force-update signal (min_version guard).
    const unlistenForce = listen<ForceUpdatePayload>('updater:force-update', (e) => {
      setForced(e.payload);
      setModalOpen(true);
    });

    // 2. Normal background check on mount.
    (async () => {
      try {
        const update = await check();
        if (update) {
          setAvailable({
            version: update.version,
            date: update.date ?? null,
            body: update.body ?? null,
          });
        }
      } catch (err) {
        console.warn('updater check failed:', err);
      }
    })();

    return () => {
      unlistenForce.then((fn) => fn());
    };
  }, []);

  async function applyUpdate() {
    setInstalling(true);
    try {
      const update = await check();
      if (!update) {
        setInstalling(false);
        return;
      }
      await update.downloadAndInstall();
      await relaunch();
    } catch (err) {
      console.error('update install failed:', err);
      setInstalling(false);
      alert(`Update failed: ${err}. You can download the latest installer manually.`);
    }
  }

  if (!IS_DESKTOP) return null;

  if (forced) {
    return (
      <div role="dialog" aria-modal="true" className="updater-force-modal">
        <div className="updater-modal-card">
          <h2>Update required</h2>
          <p>
            This version is no longer supported (floor: {forced.min_version}). Please
            update to {forced.version} to continue.
          </p>
          <button disabled={installing} onClick={applyUpdate}>
            {installing ? 'Updating…' : 'Update now'}
          </button>
        </div>
      </div>
    );
  }

  if (available && !modalOpen) {
    return (
      <button
        className="updater-toast"
        onClick={() => setModalOpen(true)}
        aria-label="Update available"
      >
        Update available: {available.version}
      </button>
    );
  }

  if (available && modalOpen) {
    return (
      <div role="dialog" aria-modal="true" className="updater-modal">
        <div className="updater-modal-card">
          <h2>Update to {available.version}</h2>
          {available.body ? <pre className="updater-notes">{available.body}</pre> : null}
          <div className="updater-modal-actions">
            <button disabled={installing} onClick={applyUpdate}>
              {installing ? 'Installing…' : 'Install and restart'}
            </button>
            <button disabled={installing} onClick={() => setModalOpen(false)}>
              Later
            </button>
          </div>
        </div>
      </div>
    );
  }

  return null;
}
