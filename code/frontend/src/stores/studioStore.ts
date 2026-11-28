import { create } from 'zustand';
import { getStudioTransport } from '@/api/studioApi';
import type {
  DeckTemplate,
  LocalModelInfo,
  SeatConfig,
  StepEvalSnapshot,
  StudioBoardPayload,
  StudioCardEntry,
  StudioEngineAssertion,
  StudioExpectation,
  StudioLibraryResponse,
  StudioPuzzleDoc,
  StudioReplayResponse,
  StudioRlGoal,
  StudioSessionResponse,
} from '@/api/studioTransport';
import { useGameStore } from './gameStore';

export type StudioView = 'library' | 'workspace' | 'replay';
export type StudioMode = 'compose' | 'play';
export type PlayoutSpeed = 'slow' | 'normal' | 'fast';

export const PLAYOUT_SPEED_MS: Record<PlayoutSpeed, number> = {
  slow: 1600,
  normal: 800,
  fast: 250,
};

interface StudioState {
  view: StudioView;
  mode: StudioMode;
  flipped: boolean;
  loading: boolean;
  error: string | null;
  statusMessage: string | null;

  library: StudioLibraryResponse | null;
  deckTemplates: DeckTemplate[];
  implementedCardIds: Set<string>;
  cardCatalog: StudioCardEntry[];
  models: LocalModelInfo[];

  session: StudioSessionResponse | null;
  replay: StudioReplayResponse | null;

  // Puzzle authoring metadata
  puzzleId: string;
  puzzleTitle: string;
  puzzleDescription: string;
  puzzleFolder: string;
  puzzleTags: string[];
  puzzleSeat: 1 | 2;
  expectations: StudioExpectation[];
  assertions: StudioEngineAssertion[];
  rlGoal: StudioRlGoal;

  // Playout & inspection
  playoutRunning: boolean;
  playoutSpeed: PlayoutSpeed;
  selectedStepIndex: number | null;
  liveEval: StepEvalSnapshot | null;
  inspectBusy: boolean;
  assertionResults: Array<{ kind: string; passed: boolean; message: string }> | null;

  // Actions
  setView: (view: StudioView) => void;
  setMode: (mode: StudioMode) => void;
  toggleFlipped: () => void;
  setPlayoutRunning: (running: boolean) => void;
  setPlayoutSpeed: (speed: PlayoutSpeed) => void;
  setSelectedStepIndex: (idx: number | null) => void;
  setPuzzleMeta: (patch: Partial<Pick<StudioState, 'puzzleId' | 'puzzleTitle' | 'puzzleDescription' | 'puzzleFolder' | 'puzzleTags' | 'puzzleSeat' | 'rlGoal'>>) => void;
  addExpectation: (exp: StudioExpectation) => void;
  removeExpectation: (idx: number) => void;
  addAssertion: (a: StudioEngineAssertion) => void;
  removeAssertion: (idx: number) => void;
  clearError: () => void;

  loadBootstrap: () => Promise<void>;
  refreshLibrary: () => Promise<void>;
  createBlankSession: () => Promise<void>;
  createSessionFromDecks: (deck1: string[], deck2: string[]) => Promise<void>;
  updateBoard: (board: StudioBoardPayload) => Promise<void>;
  updateSeats: (seats: Record<string, SeatConfig>) => Promise<void>;
  stepHuman: (actionId: number) => Promise<void>;
  stepModel: () => Promise<void>;
  rewindTo: (step: number) => Promise<void>;
  snapshotAsStart: () => Promise<void>;
  saveNamedLine: (name: string, role: 'target' | 'avoid' | 'alternative') => Promise<void>;
  deleteNamedLine: (lineId: string) => Promise<void>;
  loadNamedLine: (lineId: string) => Promise<void>;
  inspectCurrentPosition: (cfg: { model_id: string; simulations: number; worlds: number; regime: 'open_list' | 'closed_list'; seed: number }) => Promise<void>;
  evaluateCurrentAssertions: () => Promise<void>;

  openPuzzle: (puzzleId: string, startMode?: StudioMode) => Promise<void>;
  openSearchExam: (examId: string, startMode?: StudioMode) => Promise<void>;
  openScenario: (scenarioId: string, startMode?: StudioMode) => Promise<void>;
  saveCurrentPuzzle: () => Promise<string | null>;
  exportCurrentScenario: () => Promise<string | null>;
  exportCurrentSearchExam: () => Promise<string | null>;

  loadReplay: (req: { source: 'recording_id' | 'watch_queue' | 'native_json' | 'dcgo_jsonl'; ref?: string; content?: string; verify?: boolean }) => Promise<void>;
  stepReplay: () => Promise<void>;
  stepBackReplay: () => Promise<void>;
  seekReplay: (step: number) => Promise<void>;
  forkCurrentReplay: () => Promise<void>;
  renameActiveReplay: (title: string) => Promise<void>;
  updateActiveReplayTags: (userTags: string[]) => Promise<void>;
}

function syncGameStoreFromSession(sess: StudioSessionResponse) {
  const gs = useGameStore.getState();
  gs.setGameState(sess.state);
  gs.setActionMask(sess.action_mask);
  gs.setPlayerLabels({ 1: 'Player 1', 2: 'Player 2' });
}

function syncGameStoreFromReplay(rep: StudioReplayResponse) {
  const gs = useGameStore.getState();
  gs.setGameState(rep.state);
  gs.setActionMask([]);
  gs.setPlayerLabels({ 1: 'Player 1', 2: 'Player 2' });
}

export const useStudioStore = create<StudioState>((set, get) => ({
  view: 'library',
  mode: 'compose',
  flipped: false,
  loading: false,
  error: null,
  statusMessage: null,

  library: null,
  deckTemplates: [],
  implementedCardIds: new Set<string>(),
  cardCatalog: [],
  models: [],

  session: null,
  replay: null,

  puzzleId: 'new-puzzle',
  puzzleTitle: 'New Studio Puzzle',
  puzzleDescription: '',
  puzzleFolder: '',
  puzzleTags: [],
  puzzleSeat: 1,
  expectations: [],
  assertions: [],
  rlGoal: {
    mode: 'playout_to_win',
    max_turns: null,
    max_actions: null,
    win_assertions: [],
  },

  playoutRunning: false,
  playoutSpeed: 'normal',
  selectedStepIndex: null,
  liveEval: null,
  inspectBusy: false,
  assertionResults: null,

  setView: (view) => set({ view, playoutRunning: false }),
  setMode: (mode) => set({ mode, playoutRunning: false }),
  toggleFlipped: () => set((s) => ({ flipped: !s.flipped })),
  setPlayoutRunning: (playoutRunning) => set({ playoutRunning }),
  setPlayoutSpeed: (playoutSpeed) => set({ playoutSpeed }),
  setSelectedStepIndex: (selectedStepIndex) => set({ selectedStepIndex }),
  setPuzzleMeta: (patch) => set(patch),
  addExpectation: (exp) => set((s) => ({ expectations: [...s.expectations, exp] })),
  removeExpectation: (idx) =>
    set((s) => ({ expectations: s.expectations.filter((_, i) => i !== idx) })),
  addAssertion: (a) => set((s) => ({ assertions: [...s.assertions, a] })),
  removeAssertion: (idx) =>
    set((s) => ({ assertions: s.assertions.filter((_, i) => i !== idx) })),
  clearError: () => set({ error: null, statusMessage: null }),

  loadBootstrap: async () => {
    set({ loading: true, error: null });
    try {
      const t = getStudioTransport();
      const [library, deckTemplates, impl, cards, models] = await Promise.all([
        t.getLibrary(),
        t.listDeckTemplates(),
        t.listImplementedCards(),
        t.listCards(),
        t.listModels(),
      ]);
      set({
        library,
        deckTemplates,
        implementedCardIds: new Set(impl),
        cardCatalog: cards,
        models,
        loading: false,
      });
    } catch (err) {
      set({ loading: false, error: err instanceof Error ? err.message : String(err) });
    }
  },

  refreshLibrary: async () => {
    try {
      const library = await getStudioTransport().getLibrary();
      set({ library });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  createBlankSession: async () => {
    set({ loading: true, error: null, playoutRunning: false });
    try {
      const sess = await getStudioTransport().createSession({ blank: true });
      syncGameStoreFromSession(sess);
      set({
        session: sess,
        view: 'workspace',
        mode: 'compose',
        puzzleId: 'new-puzzle',
        puzzleTitle: 'New Studio Puzzle',
        puzzleDescription: '',
        puzzleFolder: '',
        puzzleTags: [],
        expectations: [],
        assertions: [],
        assertionResults: null,
        liveEval: null,
        selectedStepIndex: null,
        loading: false,
      });
    } catch (err) {
      set({ loading: false, error: err instanceof Error ? err.message : String(err) });
    }
  },

  createSessionFromDecks: async (deck1, deck2) => {
    set({ loading: true, error: null, playoutRunning: false });
    try {
      const sess = await getStudioTransport().createSession({
        board: {
          deck1,
          deck2,
          seed: 0,
          memory: 0,
          phase: 'Main',
          turn: 1,
          first_player: 1,
          zones: {},
        },
      });
      syncGameStoreFromSession(sess);
      set({
        session: sess,
        view: 'workspace',
        mode: 'compose',
        puzzleId: 'new-puzzle',
        puzzleTitle: 'New Studio Puzzle',
        puzzleDescription: '',
        puzzleFolder: '',
        puzzleTags: [],
        expectations: [],
        assertions: [],
        assertionResults: null,
        liveEval: null,
        selectedStepIndex: null,
        loading: false,
      });
    } catch (err) {
      set({ loading: false, error: err instanceof Error ? err.message : String(err) });
    }
  },

  updateBoard: async (board) => {
    const cur = get().session;
    if (!cur) return;
    set({ error: null });
    try {
      const sess = await getStudioTransport().updateBoard(cur.session_id, board);
      syncGameStoreFromSession(sess);
      set({ session: sess, liveEval: null, selectedStepIndex: null });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  updateSeats: async (seats) => {
    const cur = get().session;
    if (!cur) return;
    try {
      const sess = await getStudioTransport().updateSeats(cur.session_id, seats);
      set({ session: sess });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  stepHuman: async (actionId) => {
    const cur = get().session;
    if (!cur) return;
    set({ error: null });
    try {
      const sess = await getStudioTransport().stepHuman(cur.session_id, actionId);
      syncGameStoreFromSession(sess);
      set({
        session: sess,
        liveEval: null,
        selectedStepIndex: sess.timeline.length > 0 ? sess.timeline.length - 1 : null,
      });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  stepModel: async () => {
    const cur = get().session;
    if (!cur || cur.is_game_over) {
      set({ playoutRunning: false });
      return;
    }
    set({ inspectBusy: true, error: null });
    try {
      const sess = await getStudioTransport().stepModel(cur.session_id, get().expectations);
      syncGameStoreFromSession(sess);
      set({
        session: sess,
        liveEval: sess.eval ?? null,
        selectedStepIndex: sess.timeline.length > 0 ? sess.timeline.length - 1 : null,
        inspectBusy: false,
        playoutRunning: sess.is_game_over ? false : get().playoutRunning,
      });
    } catch (err) {
      set({
        inspectBusy: false,
        playoutRunning: false,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },

  rewindTo: async (step) => {
    const cur = get().session;
    if (!cur) return;
    set({ playoutRunning: false, error: null });
    try {
      const sess = await getStudioTransport().rewind(cur.session_id, step);
      syncGameStoreFromSession(sess);
      set({
        session: sess,
        liveEval: null,
        selectedStepIndex: sess.timeline.length > 0 ? sess.timeline.length - 1 : null,
      });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  snapshotAsStart: async () => {
    const cur = get().session;
    if (!cur) return;
    set({ playoutRunning: false, error: null });
    try {
      const sess = await getStudioTransport().snapshotAsStart(cur.session_id);
      syncGameStoreFromSession(sess);
      set({
        session: sess,
        liveEval: null,
        selectedStepIndex: null,
        statusMessage: 'Captured current board as new puzzle start.',
      });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  saveNamedLine: async (name, role) => {
    const cur = get().session;
    if (!cur) return;
    try {
      const sess = await getStudioTransport().saveLine(cur.session_id, { name, role });
      set({ session: sess, statusMessage: `Saved line "${name}" (${role}).` });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  deleteNamedLine: async (lineId) => {
    const cur = get().session;
    if (!cur) return;
    try {
      const sess = await getStudioTransport().deleteLine(cur.session_id, lineId);
      set({ session: sess });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  loadNamedLine: async (lineId) => {
    const cur = get().session;
    if (!cur) return;
    set({ playoutRunning: false });
    try {
      const sess = await getStudioTransport().loadLine(cur.session_id, lineId);
      syncGameStoreFromSession(sess);
      set({
        session: sess,
        selectedStepIndex: sess.timeline.length > 0 ? sess.timeline.length - 1 : null,
      });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  inspectCurrentPosition: async (cfg) => {
    const cur = get().session;
    if (!cur) return;
    set({ inspectBusy: true, error: null });
    try {
      const res = await getStudioTransport().inspectAgent(cur.session_id, {
        ...cfg,
        expectations: get().expectations,
      });
      set({ liveEval: res, selectedStepIndex: null, inspectBusy: false });
    } catch (err) {
      set({ inspectBusy: false, error: err instanceof Error ? err.message : String(err) });
    }
  },

  evaluateCurrentAssertions: async () => {
    const cur = get().session;
    if (!cur) return;
    try {
      const res = await getStudioTransport().evaluateAssertions(cur.game_id, get().assertions);
      set({ assertionResults: res.results });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  openPuzzle: async (puzzleId, startMode = 'compose') => {
    set({ loading: true, error: null, playoutRunning: false });
    try {
      const t = getStudioTransport();
      const doc = await t.getPuzzle(puzzleId);
      const st = (doc.setup?.state as Record<string, unknown>) ?? {};
      const zones = (doc.setup?.zones as Record<string, unknown>) ?? {};
      const board: StudioBoardPayload = {
        deck1: doc.decks['1'] ?? [],
        deck2: doc.decks['2'] ?? [],
        seed: doc.seed ?? 0,
        memory: Number(st.memory ?? 0),
        phase: String(st.phase ?? 'Main'),
        turn: Number(st.turn ?? 1),
        first_player: (Number(st.first_player ?? doc.seat ?? 1) === 2 ? 2 : 1) as 1 | 2,
        zones: zones as StudioBoardPayload['zones'],
      };
      const sess = await t.createSession({ board, source: doc.source });
      for (const ln of doc.lines ?? []) {
        await t.saveLine(sess.session_id, ln);
      }
      const refreshed = await t.getSession(sess.session_id);
      syncGameStoreFromSession(refreshed);
      set({
        session: refreshed,
        view: 'workspace',
        mode: startMode,
        puzzleId: doc.id,
        puzzleTitle: doc.title,
        puzzleDescription: doc.description ?? '',
        puzzleFolder: doc.folder ?? '',
        puzzleTags: doc.tags ?? [],
        puzzleSeat: doc.seat ?? 1,
        expectations: doc.expectations ?? [],
        assertions: doc.assertions?.engine ?? [],
        rlGoal: doc.rl_goal ?? {
          mode: 'playout_to_win',
          max_turns: null,
          max_actions: null,
          win_assertions: [],
        },
        liveEval: null,
        selectedStepIndex: null,
        loading: false,
      });
    } catch (err) {
      set({ loading: false, error: err instanceof Error ? err.message : String(err) });
    }
  },

  openSearchExam: async (examId, startMode = 'play') => {
    set({ loading: true, error: null, playoutRunning: false });
    try {
      const { session, meta } = await getStudioTransport().openSearchExam(examId);
      syncGameStoreFromSession(session);
      set({
        session,
        view: 'workspace',
        mode: startMode,
        puzzleId: meta.id,
        puzzleTitle: meta.title,
        puzzleDescription: meta.description ?? '',
        puzzleFolder: '',
        puzzleTags: [],
        puzzleSeat: meta.seat ?? 1,
        expectations: meta.expectations ?? [],
        assertions: meta.assertions ?? [],
        liveEval: null,
        selectedStepIndex: null,
        statusMessage: `Loaded search exam "${meta.title}" (${meta.expectations.length} expectation(s)).`,
        loading: false,
      });
    } catch (err) {
      set({ loading: false, error: err instanceof Error ? err.message : String(err) });
    }
  },

  openScenario: async (scenarioId, startMode = 'compose') => {
    set({ loading: true, error: null, playoutRunning: false });
    try {
      const { session, meta } = await getStudioTransport().openScenario(scenarioId);
      syncGameStoreFromSession(session);
      set({
        session,
        view: 'workspace',
        mode: startMode,
        puzzleId: meta.id,
        puzzleTitle: meta.title,
        puzzleDescription: meta.description ?? '',
        puzzleFolder: '',
        puzzleTags: [],
        puzzleSeat: meta.seat ?? 1,
        expectations: meta.expectations ?? [],
        assertions: meta.assertions ?? [],
        liveEval: null,
        selectedStepIndex: null,
        statusMessage: `Loaded scenario "${meta.title}" (${meta.assertions.length} assertion(s)).`,
        loading: false,
      });
    } catch (err) {
      set({ loading: false, error: err instanceof Error ? err.message : String(err) });
    }
  },

  saveCurrentPuzzle: async () => {
    const s = get();
    if (!s.session) return null;
    const doc: StudioPuzzleDoc = {
      schema: 'studio-puzzle-v1',
      id: s.puzzleId,
      title: s.puzzleTitle,
      description: s.puzzleDescription,
      folder: s.puzzleFolder,
      tags: s.puzzleTags,
      source: s.session.source,
      decks: { '1': s.session.board.deck1, '2': s.session.board.deck2 },
      seed: s.session.board.seed,
      seat: s.puzzleSeat,
      setup: {
        kind: 'staged',
        state: {
          memory: s.session.board.memory,
          phase: s.session.board.phase,
          turn: s.session.board.turn,
          first_player: s.session.board.first_player,
        },
        zones: s.session.board.zones,
        prefix_actions: [],
      },
      lines: s.session.lines,
      expectations: s.expectations,
      assertions: { engine: s.assertions, ui: [] },
      rl_goal: s.rlGoal,
    };
    try {
      const res = await getStudioTransport().savePuzzle(doc);
      set({ statusMessage: `Saved puzzle to ${res.path}` });
      void get().refreshLibrary();
      return res.path;
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
      return null;
    }
  },

  exportCurrentScenario: async () => {
    const path = await get().saveCurrentPuzzle();
    if (!path) return null;
    try {
      const res = await getStudioTransport().exportScenario(get().puzzleId);
      set({ statusMessage: `Exported scenario to ${res.path}` });
      void get().refreshLibrary();
      return res.path;
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
      return null;
    }
  },

  exportCurrentSearchExam: async () => {
    const path = await get().saveCurrentPuzzle();
    if (!path) return null;
    try {
      const res = await getStudioTransport().exportSearchExam(get().puzzleId);
      set({ statusMessage: `Exported search exam to ${res.path}` });
      void get().refreshLibrary();
      return res.path;
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
      return null;
    }
  },

  loadReplay: async (req) => {
    set({ loading: true, error: null, playoutRunning: false });
    try {
      const rep = await getStudioTransport().loadReplay(req);
      syncGameStoreFromReplay(rep);
      set({ replay: rep, view: 'replay', loading: false });
    } catch (err) {
      set({ loading: false, error: err instanceof Error ? err.message : String(err) });
    }
  },

  stepReplay: async () => {
    const r = get().replay;
    if (!r) return;
    try {
      const next = await getStudioTransport().stepReplay(r.replay_id);
      syncGameStoreFromReplay(next);
      set({ replay: next });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  stepBackReplay: async () => {
    const r = get().replay;
    if (!r) return;
    try {
      const next = await getStudioTransport().stepBackReplay(r.replay_id);
      syncGameStoreFromReplay(next);
      set({ replay: next });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  seekReplay: async (step) => {
    const r = get().replay;
    if (!r) return;
    try {
      const next = await getStudioTransport().seekReplay(r.replay_id, step);
      syncGameStoreFromReplay(next);
      set({ replay: next });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  forkCurrentReplay: async () => {
    const r = get().replay;
    if (!r) return;
    set({ loading: true, error: null });
    try {
      const sess = await getStudioTransport().forkReplay(r.replay_id);
      syncGameStoreFromSession(sess);
      set({
        session: sess,
        view: 'workspace',
        mode: 'compose',
        puzzleTitle: r.title ? `${r.title} (step ${r.current_step})` : `Forked Replay (step ${r.current_step})`,
        liveEval: null,
        selectedStepIndex: null,
        statusMessage: sess.opaque_warning ?? `Forked replay at step ${r.current_step} into Studio.`,
        loading: false,
      });
    } catch (err) {
      set({ loading: false, error: err instanceof Error ? err.message : String(err) });
    }
  },

  renameActiveReplay: async (title) => {
    const r = get().replay;
    if (!r) return;
    const trimmed = title.trim();
    if (!trimmed) return;
    if (r.item_key) {
      await getStudioTransport().updateLibraryItem({
        item_key: r.item_key,
        title_override: trimmed,
      });
      void get().refreshLibrary();
    }
    set({ replay: { ...r, title: trimmed }, statusMessage: `Renamed replay to "${trimmed}".` });
  },

  updateActiveReplayTags: async (userTags) => {
    const r = get().replay;
    if (!r) return;
    const cleaned = [...new Set(userTags.map((t) => t.trim().toLowerCase()).filter(Boolean))].sort();
    if (r.item_key) {
      await getStudioTransport().updateLibraryItem({
        item_key: r.item_key,
        tags: cleaned,
      });
      void get().refreshLibrary();
    }
    const autoTags = r.auto_tags ?? [];
    const merged = [...new Set([...autoTags, ...cleaned])].sort();
    set({
      replay: { ...r, user_tags: cleaned, tags: merged },
      statusMessage: `Updated replay tags.`,
    });
  },
}));
