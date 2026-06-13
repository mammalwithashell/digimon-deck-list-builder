import { useState } from 'react';
import { useUpdater } from './useUpdater';
import { describeUpdateStatus } from './updaterView';
import type { UpdaterStatus } from './types';
import './updater.css';

/** Auto-surfaced update UI, mounted once at the app root.
 *
 *  Detection is pull-based ([`useUpdater`] → `updater_check_info`), which
 *  fixes the historical emit-before-listener race that silently dropped
 *  the forced-update modal. A *required* update shows a blocking modal; a
 *  merely *available* update shows a dismissible toast → modal. In-app
 *  install failures are shown in-place, always alongside the manual
 *  "download" fallback. Detection errors with no version info are left to
 *  the launcher's UpdaterStatusCard so we don't pop an empty modal. */
export function UpdaterBridge() {
  const u = useUpdater();
  const [dismissed, setDismissed] = useState(false);
  const [modalOpen, setModalOpen] = useState(false);

  if (!u.active) return null;

  const required = u.info?.update_required === true;
  const available = u.info?.update_available === true;
  const isError = u.status === 'error';

  // Nothing actionable to surface (idle/checking/up-to-date, or a bare
  // detection error the launcher card already owns).
  if (!required && !available) return null;

  const displayStatus: UpdaterStatus = isError ? 'error' : required ? 'forced' : 'available';
  const display = describeUpdateStatus(displayStatus, u.info, u.error);

  // Collapsed toast for a non-forced available update.
  if (available && !required && !modalOpen && !dismissed && !isError) {
    return (
      <button
        className="updater-toast"
        onClick={() => setModalOpen(true)}
        aria-label="Update available"
      >
        Update available: v{u.info?.latest_version}
      </button>
    );
  }
  if (available && !required && !modalOpen && (dismissed || isError)) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      className={`updater-modal${required ? ' updater-modal--forced' : ''}`}
    >
      <div className="updater-modal-card">
        <h2>{display.headline}</h2>
        <p>{display.detail}</p>
        {u.info?.notes && !required && !isError ? (
          <pre className="updater-notes">{u.info.notes}</pre>
        ) : null}
        {u.error ? (
          <p className="updater-error" role="alert">
            {u.error}
          </p>
        ) : null}
        <div className="updater-modal-actions">
          {display.showInstall ? (
            <button disabled={u.installing} onClick={() => void u.installInApp()}>
              {u.installing ? 'Installing…' : 'Install & restart'}
            </button>
          ) : null}
          {display.showDownload ? (
            <button
              className="updater-secondary"
              disabled={u.installing}
              onClick={() => void u.openDownloadPage()}
            >
              Download manually
            </button>
          ) : null}
          {!required ? (
            <button
              className="updater-tertiary"
              disabled={u.installing}
              onClick={() => {
                setModalOpen(false);
                setDismissed(true);
              }}
            >
              Later
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}
