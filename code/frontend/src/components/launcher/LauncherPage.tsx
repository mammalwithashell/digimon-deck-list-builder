import { useEffect, useMemo, useState } from 'react';
import { fetchPatchNotes, type PatchNotesResponse } from '@/api/patchNotesApi';
import { listTestedCards } from '@/api/deckApi';
import { isServerHealthy } from '@/api/systemApi';
import { useAppVersion } from '@/hooks/useAppVersion';
import { useAuthStore } from '@/stores/authStore';
import * as deckStore from '@/storage/deckStore';
import type { DeckSummary } from '@/types/deck';
import { LauncherActions } from './LauncherActions';
import { LauncherDeckPanel } from './LauncherDeckPanel';
import { LauncherNewsPanel } from './LauncherNewsPanel';
import { LauncherShell } from './LauncherShell';
import { UpdaterStatusCard } from './UpdaterStatusCard';
import {
  buildDeckRows,
  countDraftDecks,
  formatCardCount,
  summarizeLatestRelease,
} from './launcherData';
import './launcher.css';

interface LauncherState {
  decks: DeckSummary[];
  patchNotes: PatchNotesResponse | null;
  testedCardCount: number | null;
  serverHealthy: boolean;
  loaded: boolean;
}

const initialState: LauncherState = {
  decks: [],
  patchNotes: null,
  testedCardCount: null,
  serverHealthy: false,
  loaded: false,
};

export function LauncherPage() {
  const user = useAuthStore((state) => state.user);
  const appVersion = useAppVersion();
  const [state, setState] = useState<LauncherState>(initialState);

  useEffect(() => {
    let active = true;
    async function load() {
      const [decksResult, patchResult, cardsResult, healthResult] = await Promise.allSettled([
        deckStore.listDecks(),
        fetchPatchNotes(),
        listTestedCards(),
        isServerHealthy(),
      ]);
      if (!active) return;
      setState({
        decks: decksResult.status === 'fulfilled' ? decksResult.value : [],
        patchNotes: patchResult.status === 'fulfilled' ? patchResult.value : null,
        testedCardCount: cardsResult.status === 'fulfilled' ? cardsResult.value.length : null,
        serverHealthy: healthResult.status === 'fulfilled' ? healthResult.value : false,
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
    <LauncherShell
      buildVersion={appVersion}
      cardCountLabel={formatCardCount(state.testedCardCount)}
      deckCount={state.decks.length}
      draftCount={countDraftDecks(state.decks)}
      serverHealthy={state.serverHealthy}
      username={user?.username ?? 'Guest'}
    >
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
    </LauncherShell>
  );
}
