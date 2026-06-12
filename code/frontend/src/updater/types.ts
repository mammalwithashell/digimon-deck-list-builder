/** Update status reported by the `updater_check_info` Tauri command.
 *  Keys are snake_case because the Rust `UpdateInfo` struct is serialized
 *  verbatim (no `rename_all`), matching the historical updater payload. */
export interface UpdateInfo {
  current_version: string;
  latest_version: string;
  min_version: string;
  notes: string | null;
  /** `current < latest` — a newer build exists. */
  update_available: boolean;
  /** `current < min_version` — running build is unsupported (blocking). */
  update_required: boolean;
  /** Direct installer URL for this platform, if the manifest lists one. */
  download_url: string | null;
  /** Browser download page — the manual fallback. */
  download_page_url: string;
}

/** Lifecycle of an update check. `idle` means we're not in a Tauri
 *  runtime (e.g. browser-dev), so the updater is inert. */
export type UpdaterStatus =
  | 'idle'
  | 'checking'
  | 'uptodate'
  | 'available'
  | 'forced'
  | 'error';
