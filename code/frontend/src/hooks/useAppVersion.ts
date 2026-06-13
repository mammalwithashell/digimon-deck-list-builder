import { useEffect, useState } from 'react';

/** Build-time version injected by vite (`define`) from tauri.conf.json.
 *  Used as the initial value and as the web-build fallback. */
const BUILD_VERSION = import.meta.env.VITE_APP_VERSION ?? '0.0.0';

function inTauriRuntime(): boolean {
  if (typeof window === 'undefined') return false;
  return Boolean((window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
}

/** The running app's version for display (e.g. the launcher BUILD label).
 *
 *  On desktop, reads the **actual runtime version** via Tauri's
 *  `getVersion()` (sourced from `tauri.conf.json`), so the label always
 *  matches the installed build — fixing the bug where a hardcoded/stale
 *  `VITE_APP_VERSION` left it pinned to an old number after updating. The
 *  build-time injected value seeds the initial render (no wrong-version
 *  flash) and is the fallback in a plain browser. */
export function useAppVersion(): string {
  const [version, setVersion] = useState(BUILD_VERSION);

  useEffect(() => {
    if (!inTauriRuntime()) return;
    let active = true;
    void import('@tauri-apps/api/app')
      .then((m) => m.getVersion())
      .then((v) => {
        if (active && v) setVersion(v);
      })
      .catch(() => {
        /* keep the build-time value if the runtime call fails */
      });
    return () => {
      active = false;
    };
  }, []);

  return version;
}
