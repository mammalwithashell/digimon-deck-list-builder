import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { StudioPage } from './StudioPage';
import { setStudioTransport } from '@/api/studioApi';
import type {
  StudioLibraryResponse,
  StudioSessionResponse,
  StudioTransport,
  StepEvalSnapshot,
} from '@/api/studioTransport';
import { useStudioStore } from '@/stores/studioStore';
import { GamePhase } from '@/types/game';

function makeMockState() {
  const emptyPlayer = (id: number) => ({
    id,
    name: `Player ${id}`,
    memory: 0,
    handCount: 1,
    handIds: ['ST1-03'],
    handCards: [],
    securityCount: 1,
    securityIds: ['ST1-03'],
    securityFaceUp: [false],
    deckCount: 40,
    eggDeckCount: 4,
    battleAreaCount: 0,
    battleArea: [],
    breedingArea: null,
    trashIds: [],
  });
  return {
    turnCount: 1,
    currentPhase: GamePhase.Main,
    currentPlayer: 1,
    firstPlayer: 1,
    memoryGauge: 2,
    isGameOver: false,
    winner: null,
    player1: emptyPlayer(1),
    player2: emptyPlayer(2),
    revealedCards: [],
    pendingSelection: null,
    pendingAttack: null,
  };
}

function makeMockSession(overrides: Partial<StudioSessionResponse> = {}): StudioSessionResponse {
  return {
    session_id: 'sess-1',
    game_id: 'sess-1',
    board: {
      deck1: ['ST1-03'],
      deck2: ['ST1-03'],
      seed: 0,
      memory: 2,
      phase: 'Main',
      turn: 1,
      first_player: 1,
      zones: {},
    },
    seats: {
      '1': { type: 'human', simulations: 0, worlds: 1, regime: 'open_list', seed: 0 },
      '2': { type: 'human', simulations: 0, worlds: 1, regime: 'open_list', seed: 0 },
    },
    source: { kind: 'composed' },
    timeline: [],
    lines: [],
    validation_error: null,
    state: makeMockState(),
    internal_state: {},
    action_mask: Array.from({ length: 100 }, (_, i) => (i === 0 || i === 62 ? 1 : 0)),
    is_game_over: false,
    current_player: 1,
    pending_selection: null,
    ...overrides,
  };
}

const MOCK_EVAL: StepEvalSnapshot = {
  mode: 'search',
  regime: 'open_list',
  viewer: 1,
  legal_count: 2,
  policy_value: 0.412,
  policy_actions: [{ action_id: 0, label: 'Play ST1-03', prob: 0.72 }],
  search_value: 0.655,
  simulations_run: 64,
  root_plans: [
    {
      actions: [0],
      labels: ['Play ST1-03'],
      visits: 52,
      share: 0.8125,
      q_value: 0.68,
      prior_prob: 0.72,
    },
  ],
  root_q: [{ action_id: 0, label: 'Play ST1-03', q_value: 0.68 }],
  chosen_action_id: 0,
  chosen_plan: [0],
  chosen_label: 'Play ST1-03',
  expectation_results: [
    { kind: 'visits', plan: [0], passed: true, observed_share: 0.8125, detail: 'observed 81.2% vs >= 10.0%' },
  ],
};

function makeMockTransport(): StudioTransport {
  const lib: StudioLibraryResponse = {
    folders: ['examon/lethal'],
    items: [
      {
        item_key: 'puzzle:ex13-training',
        kind: 'puzzle',
        id: 'ex13-training',
        title: 'Examon Training Lethal',
        folder: 'examon/lethal',
        tags: ['lethal', 'training'],
        ref: 'qa/puzzles/examon/lethal/ex13-training.json',
      },
      {
        item_key: 'dcgo_jsonl:mod_game2.jsonl',
        kind: 'dcgo_jsonl',
        id: 'mod_game2.jsonl',
        title: 'mod_game2.jsonl',
        folder: 'examon/lethal',
        tags: ['dcgo'],
        ref: 'mod_game2.jsonl',
      },
    ],
  };

  return {
    createSession: vi.fn(async () => makeMockSession()),
    getSession: vi.fn(async () => makeMockSession()),
    updateBoard: vi.fn(async (_id, board) => makeMockSession({ board })),
    updateSeats: vi.fn(async (_id, seats) => makeMockSession({ seats })),
    stepHuman: vi.fn(async () => makeMockSession()),
    stepModel: vi.fn(async () =>
      makeMockSession({
        eval: MOCK_EVAL,
        timeline: [
          {
            step_index: 0,
            action_id: 0,
            actor: 1,
            controller: 'model:cp8.onnx',
            label: 'Play ST1-03',
            phase_before: 'Main',
            phase_after: 'Main',
            memory_after: 0,
            turn_after: 1,
            events: [],
            eval: MOCK_EVAL,
          },
        ],
      }),
    ),
    rewind: vi.fn(async () => makeMockSession()),
    snapshotAsStart: vi.fn(async () => makeMockSession()),
    saveLine: vi.fn(async () => makeMockSession()),
    deleteLine: vi.fn(async () => makeMockSession()),
    loadLine: vi.fn(async () => makeMockSession()),
    evaluateAssertions: vi.fn(async () => ({ all_passed: true, results: [] })),
    listModels: vi.fn(async () => [{ id: 'cp8.onnx', name: 'cp8', bytes: 1024 }]),
    inspectAgent: vi.fn(async () => MOCK_EVAL),
    listDeckTemplates: vi.fn(async () => [{ id: 'blank-filler', name: 'Blank', source: 'builtin', cards: ['ST1-03'] }]),
    listImplementedCards: vi.fn(async () => ['ST1-03', 'EX13-018']),
    listCards: vi.fn(async () => [
      {
        card_id: 'ST1-03',
        name: 'Agumon',
        card_kind: 0,
        level: 3,
        colors: [0],
        dp: 2000,
        play_cost: 3,
        traits: ['Reptile'],
        effect_text: '',
        inherited_text: '[Your Turn] This Digimon gets +1000 DP.',
        security_text: '',
        implemented: true,
      },
      {
        card_id: 'EX13-018',
        name: 'Wingdramon',
        card_kind: 0,
        level: 5,
        colors: [1],
        dp: 7000,
        play_cost: 7,
        traits: ['Sky Dragon'],
        effect_text: '[When Digivolving] Unsuspend this Digimon.',
        inherited_text: '',
        security_text: '',
        implemented: true,
      },
    ]),
    getLibrary: vi.fn(async () => lib),
    createFolder: vi.fn(async (f) => ({ folder: f, folders: ['examon/lethal', f] })),
    renameFolder: vi.fn(async () => ({ folders: [] })),
    deleteFolder: vi.fn(async () => ({ folders: [] })),
    updateLibraryItem: vi.fn(async () => ({ item_key: '', meta: {} })),
    loadReplay: vi.fn(async () => ({
      replay_id: 'rep-1',
      source: { kind: 'dcgo_jsonl', ref: 'mod_game2.jsonl' },
      total_steps: 5,
      current_step: 0,
      is_complete: false,
      is_game_over: false,
      is_paused: false,
      winner_id: null,
      skipped_incomplete_plays: 0,
      has_opaque_placeholders: false,
      opaque_placeholder_zones: {},
      truncation_warning: null,
      divergences: [],
      state: makeMockState(),
    })),
    stepReplay: vi.fn(async () => ({
      replay_id: 'rep-1',
      source: { kind: 'dcgo_jsonl' },
      total_steps: 5,
      current_step: 1,
      is_complete: false,
      is_game_over: false,
      is_paused: false,
      winner_id: null,
      skipped_incomplete_plays: 0,
      has_opaque_placeholders: false,
      opaque_placeholder_zones: {},
      truncation_warning: null,
      divergences: [],
      state: makeMockState(),
    })),
    stepBackReplay: vi.fn(async () => ({
      replay_id: 'rep-1',
      source: { kind: 'dcgo_jsonl' },
      total_steps: 5,
      current_step: 0,
      is_complete: false,
      is_game_over: false,
      is_paused: false,
      winner_id: null,
      skipped_incomplete_plays: 0,
      has_opaque_placeholders: false,
      opaque_placeholder_zones: {},
      truncation_warning: null,
      divergences: [],
      state: makeMockState(),
    })),
    seekReplay: vi.fn(async () => ({
      replay_id: 'rep-1',
      source: { kind: 'dcgo_jsonl' },
      total_steps: 5,
      current_step: 2,
      is_complete: false,
      is_game_over: false,
      is_paused: false,
      winner_id: null,
      skipped_incomplete_plays: 0,
      has_opaque_placeholders: false,
      opaque_placeholder_zones: {},
      truncation_warning: null,
      divergences: [],
      state: makeMockState(),
    })),
    forkReplay: vi.fn(async () => makeMockSession()),
    getPuzzle: vi.fn(async () => ({
      id: 'ex13-training',
      title: 'Examon Training Lethal',
      description: '',
      folder: 'examon/lethal',
      tags: ['lethal'],
      source: { kind: 'composed' },
      decks: { '1': ['ST1-03'], '2': ['ST1-03'] },
      seed: 0,
      seat: 1 as const,
      setup: { kind: 'staged', state: { memory: 2, phase: 'Main', turn: 1, first_player: 1 }, zones: {} },
      lines: [],
      expectations: [],
      assertions: { engine: [], ui: [] },
      rl_goal: { mode: 'playout_to_win' as const, max_turns: null, max_actions: null, win_assertions: [] },
    })),
    savePuzzle: vi.fn(async (doc) => ({ puzzle: doc, path: `qa/puzzles/${doc.id}.json` })),
    deletePuzzle: vi.fn(async (id) => ({ status: 'deleted', id })),
    exportScenario: vi.fn(async (id) => ({ path: `qa/scenarios/${id}.json` })),
    exportSearchExam: vi.fn(async (id) => ({ path: `qa/search-exams/${id}.json` })),
    openSearchExam: vi.fn(async (id) => ({
      session: makeMockSession(),
      meta: { id, title: id, description: '', seat: 1 as const, expectations: [], assertions: [] },
    })),
    openScenario: vi.fn(async (id) => ({
      session: makeMockSession(),
      meta: { id, title: id, description: '', seat: 1 as const, expectations: [], assertions: [] },
    })),
  };
}

describe('StudioPage', () => {
  beforeEach(() => {
    setStudioTransport(makeMockTransport());
    useStudioStore.setState({
      view: 'library',
      mode: 'compose',
      session: null,
      replay: null,
      liveEval: null,
      playoutRunning: false,
      error: null,
      statusMessage: null,
    });
  });

  it('renders Studio Library with folders and tags and launches a Blank Board session', async () => {
    render(<StudioPage />);
    await waitFor(() => {
      expect(screen.getByText('Examon Training Lethal')).toBeTruthy();
      expect(screen.getAllByText('mod_game2.jsonl').length).toBeGreaterThan(0);
    });

    fireEvent.click(screen.getByTestId('studio-new-blank'));
    await waitFor(() => {
      expect(screen.getByTestId('studio-card-search')).toBeTruthy();
      expect(screen.getByTestId('studio-agent-inspector')).toBeTruthy();
    });
  });

  it('inspects policy head and MCTS search values on a staged board', async () => {
    render(<StudioPage />);
    await waitFor(() => expect(screen.getByTestId('studio-new-blank')).toBeTruthy());
    fireEvent.click(screen.getByTestId('studio-new-blank'));

    await waitFor(() => expect(screen.getByTestId('studio-inspect-now')).toBeTruthy());
    fireEvent.click(screen.getByTestId('studio-inspect-now'));

    await waitFor(() => {
      expect(screen.getByText('+0.412')).toBeTruthy();
      expect(screen.getByText('+0.655')).toBeTruthy();
      expect(screen.getAllByText(/Play ST1-03/).length).toBeGreaterThan(0);
    });
  });

  it('searches cards by name and effect text and toggles collapsible color/level/type filters', async () => {
    render(<StudioPage />);
    await waitFor(() => expect(screen.getByTestId('studio-new-blank')).toBeTruthy());
    fireEvent.click(screen.getByTestId('studio-new-blank'));

    await waitFor(() => {
      expect(screen.getByText('Agumon')).toBeTruthy();
      expect(screen.getByText('Wingdramon')).toBeTruthy();
    });

    // Filters start collapsed
    expect(screen.queryByTestId('studio-filters-drawer')).toBeNull();

    // Search by card text
    const searchInput = screen.getByLabelText('Search cards');
    fireEvent.change(searchInput, { target: { value: 'Unsuspend' } });
    expect(screen.getByText('Wingdramon')).toBeTruthy();
    expect(screen.queryByText('Agumon')).toBeNull();

    // Clear search and filter by collapsible Color filter (Red)
    fireEvent.change(searchInput, { target: { value: '' } });
    fireEvent.click(screen.getByTestId('studio-filters-toggle'));
    expect(screen.getByTestId('studio-filters-drawer')).toBeTruthy();
    fireEvent.click(screen.getByText('Red'));
    expect(screen.getByText('Agumon')).toBeTruthy();
    expect(screen.queryByText('Wingdramon')).toBeNull();
  });
});
