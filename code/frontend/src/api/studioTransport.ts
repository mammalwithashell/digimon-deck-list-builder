/**
 * Typed transport seam for the Board-State Studio (add-board-state-studio).
 *
 * All Studio UI components and Zustand stores communicate exclusively through
 * this interface so the MVP (`HttpStudioTransport`) can later be paired with a
 * desktop `TauriStudioTransport` without refactoring UI components.
 */

import type { GameState, PendingSelection } from '@/types/game';

export type SeatControllerType = 'human' | 'model' | 'greedy' | 'random';
export type SearchRegime = 'open_list' | 'closed_list';

export interface SeatConfig {
  type: SeatControllerType;
  model_id?: string | null;
  simulations: number;
  worlds: number;
  regime: SearchRegime;
  seed: number;
}

export interface StudioFieldStack {
  stack: string[];
  is_suspended: boolean;
  turn_played: number;
  linked_cards?: string[];
}

export interface StudioZoneSpec {
  hand?: string[];
  deck_top?: string[];
  egg_deck?: string[];
  security?: Array<string | { card_id: string; face_up: boolean }>;
  face_up_security?: number[];
  trash?: string[];
  field?: StudioFieldStack[];
  breeding?: string[] | null;
}

export interface StudioBoardPayload {
  deck1: string[];
  deck2: string[];
  seed: number;
  memory: number;
  phase: string;
  turn: number;
  first_player: 1 | 2;
  zones: Record<string, StudioZoneSpec>;
}

export interface PolicyActionScore {
  action_id: number;
  label: string;
  prob: number;
}

export interface SearchPlanScore {
  actions: number[];
  labels: string[];
  visits: number;
  share: number;
  q_value: number;
  prior_prob: number;
}

export interface ExpectationResult {
  kind: string;
  plan: number[];
  passed: boolean;
  observed_share: number;
  detail: string;
}

export interface StepEvalSnapshot {
  mode: 'policy_only' | 'search';
  regime: string;
  viewer: number;
  legal_count: number;
  policy_value: number;
  policy_actions: PolicyActionScore[];
  search_value: number | null;
  simulations_run: number;
  root_plans: SearchPlanScore[];
  root_q: Array<{ action_id: number; label: string; q_value: number }>;
  chosen_action_id: number;
  chosen_plan: number[];
  chosen_label: string;
  expectation_results: ExpectationResult[];
}

export interface RecordedStep {
  step_index: number;
  action_id: number;
  actor: number;
  controller: string;
  label: string;
  phase_before: string;
  phase_after: string;
  memory_after: number;
  turn_after: number;
  events: Array<Record<string, unknown>>;
  eval?: StepEvalSnapshot;
}

export interface StudioNamedLine {
  id: string;
  name: string;
  role: 'target' | 'avoid' | 'alternative';
  actions: number[];
}

export interface StudioExpectation {
  kind: 'visits' | 'avoid' | 'choose';
  plan: number[];
  min_share?: number;
  label?: string;
}

export interface StudioEngineAssertion {
  kind: string;
  value?: number;
  player?: number;
  field_index?: number;
  card_id?: string;
  zone?: string;
  action_id?: number;
  event_type?: string;
  expected?: boolean;
}

export interface StudioRlGoal {
  mode: 'playout_to_win' | 'bounded_horizon';
  max_turns: number | null;
  max_actions: number | null;
  win_assertions: StudioEngineAssertion[];
}

export interface StudioSessionResponse {
  session_id: string;
  game_id: string;
  board: StudioBoardPayload;
  seats: Record<string, SeatConfig>;
  source: Record<string, unknown>;
  timeline: RecordedStep[];
  lines: StudioNamedLine[];
  validation_error: string | null;
  state: GameState;
  internal_state: Record<string, unknown>;
  action_mask: number[];
  legal_actions?: Array<{ action_id: number; label: string }>;
  is_game_over: boolean;
  current_player: number;
  pending_selection: PendingSelection | null;
  last_step?: RecordedStep;
  eval?: StepEvalSnapshot | null;
  opaque_warning?: string | null;
}

export interface DeckTemplate {
  id: string;
  name: string;
  source: string;
  cards: string[];
}

export interface StudioCardEntry {
  card_id: string;
  name: string;
  card_kind: number; // 0=Digimon, 1=Tamer, 2=Option, 3=DigiEgg
  level: number | null;
  colors: number[]; // 0=Red,1=Blue,2=Yellow,3=Green,4=White,5=Black,6=Purple
  dp: number | null;
  play_cost: number | null;
  traits: string[];
  effect_text: string;
  inherited_text: string;
  security_text: string;
  implemented: boolean;
}

export interface LocalModelInfo {
  id: string;
  name: string;
  bytes: number;
}

export interface LibraryItem {
  item_key: string;
  kind: 'puzzle' | 'search_exam' | 'scenario' | 'dcgo_jsonl' | 'recording';
  id: string;
  title: string;
  folder: string;
  tags: string[];
  auto_tags?: string[];
  user_tags?: string[];
  ref: string;
  source_root?: string;
  my_deck?: string | null;
  opp_deck?: string | null;
  finish_type?: 'natural' | 'concede' | 'unfinished' | null;
  divergence_status?: string | null;
  turns?: number;
  seat?: number;
  lines_count?: number;
  expectations_count?: number;
  rl_goal_mode?: string;
  total_steps?: number;
  bytes?: number;
}

export interface StudioLibraryResponse {
  folders: string[];
  items: LibraryItem[];
  analysis_status?: {
    analyzed: number;
    total: number;
    running: boolean;
  };
}

export interface ReplayLogEntry {
  step_number: number;
  player_id: number | null;
  action_id: number;
  label: string;
  phase_before: string;
  phase_after: string;
  memory_before: number;
  memory_after: number;
  turn_number: number;
}

export interface StudioReplayResponse {
  replay_id: string;
  title?: string;
  item_key?: string | null;
  my_deck?: string | null;
  opp_deck?: string | null;
  finish_type?: 'natural' | 'concede' | 'unfinished' | null;
  divergence_status?: string | null;
  auto_tags?: string[];
  user_tags?: string[];
  tags?: string[];
  source: Record<string, unknown>;
  total_steps: number;
  current_step: number;
  is_complete: boolean;
  is_game_over: boolean;
  is_paused: boolean;
  winner_id: number | null;
  skipped_incomplete_plays: number;
  has_opaque_placeholders: boolean;
  opaque_placeholder_zones: Record<string, string[]>;
  truncation_warning: string | null;
  divergences: Array<Record<string, unknown>>;
  step_log?: ReplayLogEntry[];
  state: GameState;
  step_result?: Record<string, unknown> | null;
}

export interface StudioPuzzleDoc {
  schema?: 'studio-puzzle-v1';
  id: string;
  title: string;
  description: string;
  folder: string;
  tags: string[];
  source: Record<string, unknown>;
  decks: Record<string, string[]>;
  seed: number;
  seat: 1 | 2;
  setup: Record<string, unknown>;
  lines: StudioNamedLine[];
  expectations: StudioExpectation[];
  assertions: { engine: StudioEngineAssertion[]; ui: unknown[] };
  rl_goal: StudioRlGoal;
}

export interface StudioTransport {
  // Sessions
  createSession(req: {
    board?: StudioBoardPayload;
    blank?: boolean;
    source?: Record<string, unknown>;
    seats?: Record<string, SeatConfig>;
  }): Promise<StudioSessionResponse>;
  getSession(sessionId: string): Promise<StudioSessionResponse>;
  updateBoard(sessionId: string, board: StudioBoardPayload): Promise<StudioSessionResponse>;
  updateSeats(sessionId: string, seats: Record<string, SeatConfig>): Promise<StudioSessionResponse>;
  stepHuman(sessionId: string, actionId: number): Promise<StudioSessionResponse>;
  stepModel(sessionId: string, expectations?: StudioExpectation[]): Promise<StudioSessionResponse>;
  rewind(sessionId: string, targetStep: number): Promise<StudioSessionResponse>;
  snapshotAsStart(sessionId: string): Promise<StudioSessionResponse>;
  saveLine(
    sessionId: string,
    req: { id?: string; name: string; role: 'target' | 'avoid' | 'alternative'; actions?: number[] },
  ): Promise<StudioSessionResponse>;
  deleteLine(sessionId: string, lineId: string): Promise<StudioSessionResponse>;
  loadLine(sessionId: string, lineId: string): Promise<StudioSessionResponse>;
  evaluateAssertions(
    gameId: string,
    assertions: StudioEngineAssertion[],
  ): Promise<{ all_passed: boolean; results: Array<{ kind: string; passed: boolean; message: string }> }>;

  // Agent inspection
  listModels(): Promise<LocalModelInfo[]>;
  inspectAgent(
    sessionId: string,
    req: {
      model_id: string;
      simulations: number;
      worlds: number;
      regime: SearchRegime;
      seed: number;
      top_k?: number;
      expectations?: StudioExpectation[];
    },
  ): Promise<StepEvalSnapshot>;

  // Catalogs
  listDeckTemplates(): Promise<DeckTemplate[]>;
  listImplementedCards(): Promise<string[]>;
  listCards(): Promise<StudioCardEntry[]>;

  // Library
  getLibrary(): Promise<StudioLibraryResponse>;
  createFolder(folder: string): Promise<{ folder: string; folders: string[] }>;
  renameFolder(oldFolder: string, newFolder: string): Promise<{ folders: string[] }>;
  deleteFolder(folder: string): Promise<{ folders: string[] }>;
  updateLibraryItem(req: {
    item_key: string;
    folder?: string;
    tags?: string[];
    title_override?: string;
  }): Promise<{ item_key: string; meta: Record<string, unknown> }>;

  // Replays
  loadReplay(req: {
    source: 'recording_id' | 'watch_queue' | 'native_json' | 'dcgo_jsonl';
    ref?: string;
    content?: string;
    verify?: boolean;
  }): Promise<StudioReplayResponse>;
  stepReplay(replayId: string): Promise<StudioReplayResponse>;
  stepBackReplay(replayId: string): Promise<StudioReplayResponse>;
  seekReplay(replayId: string, step: number): Promise<StudioReplayResponse>;
  forkReplay(replayId: string): Promise<StudioSessionResponse>;

  // Puzzles & Exporters
  getPuzzle(puzzleId: string): Promise<StudioPuzzleDoc>;
  savePuzzle(doc: StudioPuzzleDoc): Promise<{ puzzle: StudioPuzzleDoc; path: string }>;
  deletePuzzle(puzzleId: string): Promise<{ status: string; id: string }>;
  exportScenario(puzzleId: string, lineId?: string): Promise<{ path: string }>;
  exportSearchExam(puzzleId: string): Promise<{ path: string }>;
  openSearchExam(examId: string): Promise<{
    session: StudioSessionResponse;
    meta: {
      id: string;
      title: string;
      description: string;
      seat: 1 | 2;
      expectations: StudioExpectation[];
      assertions: StudioEngineAssertion[];
      search?: Record<string, unknown>;
    };
  }>;
  openScenario(scenarioId: string): Promise<{
    session: StudioSessionResponse;
    meta: {
      id: string;
      title: string;
      description: string;
      seat: 1 | 2;
      expectations: StudioExpectation[];
      assertions: StudioEngineAssertion[];
    };
  }>;
}
