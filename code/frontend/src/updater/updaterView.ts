import type { UpdateInfo, UpdaterStatus } from './types';

/** Presentational description of the current updater state — pure, so it
 *  can be unit-tested without a Tauri runtime. Both the auto modals and
 *  the launcher's manual-update card render from this. */
export interface UpdaterDisplay {
  headline: string;
  detail: string;
  tone: 'neutral' | 'available' | 'forced' | 'error';
  /** Offer the in-app "install & restart" button. */
  showInstall: boolean;
  /** Offer the "download manually" (browser) link. Always available in a
   *  Tauri runtime so the user is never stranded by a broken in-app path. */
  showDownload: boolean;
}

export function describeUpdateStatus(
  status: UpdaterStatus,
  info: UpdateInfo | null,
  error: string | null,
): UpdaterDisplay {
  switch (status) {
    case 'idle':
      return {
        headline: 'Updates',
        detail: 'Update checks run in the desktop app.',
        tone: 'neutral',
        showInstall: false,
        showDownload: false,
      };
    case 'checking':
      return {
        headline: 'Checking for updates…',
        detail: info ? `Current build: v${info.current_version}.` : '',
        tone: 'neutral',
        showInstall: false,
        showDownload: true,
      };
    case 'uptodate':
      return {
        headline: 'Up to date',
        detail: info ? `You're on the latest version (v${info.current_version}).` : '',
        tone: 'neutral',
        showInstall: false,
        showDownload: true,
      };
    case 'available':
      return {
        headline: 'Update available',
        detail: info
          ? `v${info.latest_version} is ready to install (you're on v${info.current_version}).`
          : '',
        tone: 'available',
        showInstall: true,
        showDownload: true,
      };
    case 'forced':
      return {
        headline: 'Update required',
        detail: info
          ? `v${info.current_version} is no longer supported. Update to v${info.latest_version} to continue.`
          : '',
        tone: 'forced',
        showInstall: true,
        showDownload: true,
      };
    case 'error':
      return {
        headline: 'Update check failed',
        // In-app install shares the failing path, so steer to the manual
        // download instead of offering a button likely to fail again.
        detail: error
          ? `${error} You can download the latest version manually.`
          : 'You can download the latest version manually.',
        tone: 'error',
        showInstall: false,
        showDownload: true,
      };
  }
}
