import { useUpdater } from '@/updater/useUpdater';
import { describeUpdateStatus } from '@/updater/updaterView';

/** Always-visible manual-update entry point in the launcher's right
 *  column. Unlike the auto modal, this is reachable even when nothing is
 *  auto-detected — so a user is never stuck if the in-app updater is
 *  silently failing. Offers an in-app "check / install" path plus a
 *  browser "download" fallback. Renders nothing outside the Tauri
 *  desktop runtime (browser-dev), where the updater is inert. */
export function UpdaterStatusCard() {
  const u = useUpdater();
  if (!u.active) return null;

  const display = describeUpdateStatus(u.status, u.info, u.error);
  const versionLabel = u.info
    ? u.status === 'available' || u.status === 'forced'
      ? `v${u.info.latest_version}`
      : `v${u.info.current_version}`
    : '';

  return (
    <section
      className={`launcher-news launcher-updater launcher-updater--${display.tone}`}
      aria-labelledby="launcher-updater-heading"
    >
      <div className="launcher-panel-head">
        <h2 id="launcher-updater-heading">{display.headline}</h2>
        {versionLabel ? <span>{versionLabel}</span> : null}
      </div>
      {display.detail ? <p className="launcher-updater-detail">{display.detail}</p> : null}
      <div className="launcher-updater-actions">
        <button
          type="button"
          className="launcher-updater-btn"
          disabled={u.status === 'checking' || u.installing}
          onClick={() => void u.refresh()}
        >
          {u.status === 'checking' ? 'Checking…' : 'Check for updates'}
        </button>
        {display.showInstall ? (
          <button
            type="button"
            className="launcher-updater-btn primary"
            disabled={u.installing}
            onClick={() => void u.installInApp()}
          >
            {u.installing ? 'Installing…' : 'Install & restart'}
          </button>
        ) : null}
        {display.showDownload ? (
          <button
            type="button"
            className="launcher-updater-btn"
            disabled={u.installing}
            onClick={() => void u.openDownloadPage()}
          >
            Download
          </button>
        ) : null}
      </div>
    </section>
  );
}
