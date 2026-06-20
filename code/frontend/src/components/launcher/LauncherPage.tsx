import { useEffect, useMemo, useState } from 'react';
import { fetchPatchNotes, type PatchNotesResponse } from '@/api/patchNotesApi';
import { useAuthStore } from '@/stores/authStore';
import * as deckStore from '@/storage/deckStore';
import type { DeckSummary } from '@/types/deck';
import { LauncherActions } from './LauncherActions';
import { LauncherDeckPanel } from './LauncherDeckPanel';
import { LauncherNewsPanel } from './LauncherNewsPanel';
import { UpdaterStatusCard } from './UpdaterStatusCard';
import { buildDeckRows, summarizeLatestRelease } from './launcherData';
import './launcher.css';

interface LauncherState {
  decks: DeckSummary[];
  patchNotes: PatchNotesResponse | null;
  loaded: boolean;
}

const initialState: LauncherState = {
  decks: [],
  patchNotes: null,
  loaded: false,
};

/**
 * Launcher home content. The surrounding chrome (nav rail, topbar, status
 * bar) is now provided by the shared `MenuShell` (add-desktop-menu-shell);
 * this component renders only the hero + side panels into the shell's
 * content area.
 */
export function LauncherPage() {
  const user = useAuthStore((state) => state.user);
  const [state, setState] = useState<LauncherState>(initialState);

  useEffect(() => {
    let active = true;
    async function load() {
      const [decksResult, patchResult] = await Promise.allSettled([
        deckStore.listDecks(),
        fetchPatchNotes(),
      ]);
      if (!active) return;
      setState({
        decks: decksResult.status === 'fulfilled' ? decksResult.value : [],
        patchNotes: patchResult.status === 'fulfilled' ? patchResult.value : null,
        loaded: true,
      });
    }
    void load();
    return () => {
      active = false;
    };
  }, []);

  const deckRows = useMemo(() => buildDeckRows(state.decks), [state.decks]);
  const releaseSummary = useMemo(
    () => summarizeLatestRelease(state.patchNotes),
    [state.patchNotes],
  );

  return (
    <div className="launcher-home">
      <section className="launcher-hero" aria-labelledby="launcher-heading">
        <div className="launcher-welcome">// WELCOME BACK, {(user?.username ?? 'GUEST').toUpperCase()}</div>
        <h1 id="launcher-heading" className="launcher-title">
          PICK UP<br />WHERE YOU<br /><em>LEFT OFF.</em>
        </h1>
        <div className="launcher-tagline">PLAY ANONYMOUSLY · NO ACCOUNT REQUIRED</div>
        <LauncherActions />
      </section>
      <aside className="launcher-right-column" aria-label="Launcher details">
        <LauncherDeckPanel decks={deckRows} loaded={state.loaded} />
        <UpdaterStatusCard />
        <LauncherNewsPanel release={releaseSummary} />
      </aside>
    </div>
  );
}
