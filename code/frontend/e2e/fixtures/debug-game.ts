import { type Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const API_BASE = process.env.API_BASE ?? 'http://localhost:8000';

// Rust-backed `/debug` router (add-ui-scenario-test-substrate). Replaces the
// legacy engine_py_legacy debug routes. A game staged here registers in the
// server's active-games store and is driven through the live `/games`
// routes (the staged game shares RustHeadlessGame's play surface).

export interface DebugGameOptions {
  deck1: string[];
  deck2: string[];
  player1_type?: string;
  player2_type?: string;
  first_player?: number;
  skip_shuffle?: boolean;
  starting_hand1?: string[];
  starting_hand2?: string[];
  auto_mulligan?: string;
  initial_memory?: number;
  seed?: number;
  phase?: string;
  turn?: number;
  zones?: Record<string, unknown>;
}

/** Normalized internal-state: the raw engine shape plus `hand_p1`/`hand_p2`
 *  convenience accessors the specs read. */
export interface DebugInternalState {
  memory: number;
  turn_count: number;
  current_phase: string;
  current_player: number;
  game_over: boolean;
  winner_id: number | null;
  players: Array<{
    player_id: number;
    hand: string[];
    deck_count: number;
    security_count: number;
    trash: string[];
    battle_area: Array<{ stack: string[]; is_suspended: boolean; turn_played: number }>;
    breeding: string[] | null;
  }>;
  hand_p1: string[];
  hand_p2: string[];
}

export interface AssertionResult {
  kind: string;
  passed: boolean;
  message: string;
}

export interface DebugGameHandle {
  gameId: string;
  sendAction: (actionId: number) => Promise<any>;
  stepGame: () => Promise<any>;
  setMemory: (memory: number) => Promise<void>;
  injectCard: (playerId: number, cardId: string, zone: string) => Promise<void>;
  placeOnField: (
    playerId: number,
    stack: string[],
    isSuspended?: boolean,
    turnPlayed?: number,
  ) => Promise<void>;
  getInternalState: () => Promise<DebugInternalState>;
  /** Legal action ids from the live mask (replaces the removed GET /games/{id}/actions). */
  getActions: () => Promise<number[]>;
  /** Evaluate a fixture's engine assertions server-side. */
  evaluate: (assertions: unknown[]) => Promise<{ all_passed: boolean; results: AssertionResult[] }>;
  delete: () => Promise<void>;
}

function attachHandle(page: Page, gameId: string): DebugGameHandle {
  return {
    gameId,
    async sendAction(actionId: number) {
      const r = await page.request.post(`${API_BASE}/games/${gameId}/actions`, {
        data: { action: actionId },
      });
      return r.json();
    },
    async stepGame() {
      const r = await page.request.post(`${API_BASE}/games/${gameId}/steps`);
      return r.json();
    },
    async setMemory(memory: number) {
      await page.request.post(`${API_BASE}/debug/games/${gameId}/set-memory`, {
        data: { memory },
      });
    },
    async injectCard(playerId: number, cardId: string, zone: string) {
      await page.request.post(`${API_BASE}/debug/games/${gameId}/inject-card`, {
        data: { player_id: playerId, card_id: cardId, zone },
      });
    },
    async placeOnField(playerId, stack, isSuspended = false, turnPlayed = 0) {
      await page.request.post(`${API_BASE}/debug/games/${gameId}/place-on-field`, {
        data: { player_id: playerId, stack, is_suspended: isSuspended, turn_played: turnPlayed },
      });
    },
    async getInternalState() {
      const r = await page.request.get(`${API_BASE}/debug/games/${gameId}/internal-state`);
      const raw = await r.json();
      const byId = (pid: number) =>
        (raw.players ?? []).find((p: any) => p.player_id === pid)?.hand ?? [];
      return { ...raw, hand_p1: byId(1), hand_p2: byId(2) } as DebugInternalState;
    },
    async getActions() {
      const r = await page.request.get(`${API_BASE}/games/${gameId}/action-mask`);
      const data = await r.json();
      const mask: number[] = data.action_mask ?? [];
      return mask.map((b, i) => (b === 1 ? i : -1)).filter((i) => i >= 0);
    },
    async evaluate(assertions: unknown[]) {
      const r = await page.request.post(`${API_BASE}/debug/games/${gameId}/evaluate`, {
        data: { assertions },
      });
      return r.json();
    },
    async delete() {
      await page.request.delete(`${API_BASE}/debug/games/${gameId}`);
    },
  };
}

export async function createDebugGame(
  page: Page,
  options: DebugGameOptions,
): Promise<DebugGameHandle> {
  const resp = await page.request.post(`${API_BASE}/debug/games`, {
    data: {
      player1_type: 'human',
      player2_type: 'human',
      initial_memory: 0,
      ...options,
    },
  });
  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`Failed to create debug game: ${resp.status()} ${body}`);
  }
  const data = await resp.json();
  return attachHandle(page, data.game_id);
}

export interface ScenarioFixture {
  id: string;
  title: string;
  readiness: 'expected_pass' | 'blocked_on_card_impl';
  decks: Record<string, string[]>;
  seed?: number;
  state: { memory: number; phase: string; turn?: number; first_player?: number };
  zones?: Record<string, unknown>;
  assertions: { engine?: unknown[]; ui?: unknown[] };
}

/** Load a `qa/scenarios/<name>.json` fixture by file name (no extension). */
export function loadScenario(name: string): ScenarioFixture {
  const here = dirname(fileURLToPath(import.meta.url));
  // e2e/fixtures -> repo qa/scenarios
  const path = join(here, '..', '..', '..', '..', 'qa', 'scenarios', `${name}.json`);
  return JSON.parse(readFileSync(path, 'utf-8')) as ScenarioFixture;
}

/** Stage a scenario fixture via `/debug/games` and return a handle. */
export async function stageScenario(
  page: Page,
  fixture: ScenarioFixture,
): Promise<DebugGameHandle> {
  return createDebugGame(page, {
    deck1: fixture.decks['1'],
    deck2: fixture.decks['2'],
    seed: fixture.seed,
    initial_memory: fixture.state.memory,
    phase: fixture.state.phase,
    turn: fixture.state.turn,
    first_player: fixture.state.first_player ?? 1,
    zones: fixture.zones,
  });
}

// ── Deck Constants ──────────────────────────────────────────────

export const MEDUSA_DECK = [
  // Eggs (4)
  "BT21-001","BT21-001","BT21-001","BT21-001",
  // Lv.3 (12)
  "BT23-005","BT23-005","BT23-005",
  "BT21-008","BT21-008","BT21-008","BT21-008",
  "BT24-008","BT24-008","BT24-008",
  "BT24-011","BT24-011",
  // Lv.4 (10)
  "BT21-017","BT21-017","BT21-017","BT21-017",
  "BT24-012","BT24-012","BT24-012",
  "P-189","P-189",
  "BT24-011",
  // Lv.5 (5)
  "BT21-025",
  "BT24-016","BT24-016","BT24-016","BT24-016",
  // Lv.6 (4)
  "BT24-017","BT24-017","BT24-017","BT24-017",
  // Lv.7 (3)
  "BT24-018","BT24-018","BT24-018",
  // Tamers (6)
  "BT18-087","BT18-087",
  "BT21-081","BT21-081",
  "BT24-082","BT24-082","BT24-082",
  // Options (10)
  "P-035","P-035",
  "P-103","P-103","P-103","P-103",
  "LM-027","LM-027",
  "BT24-089","BT24-089","BT24-089",
];

export const CS_HUDIEMON_DECK = [
  // Eggs (4)
  "BT22-005","BT22-005","BT22-005","BT22-005",
  // Lv.3 (12)
  "BT22-043","BT22-043","BT22-043","BT22-043",
  "BT22-044","BT22-044","BT22-044","BT22-044",
  "BT23-048","BT23-048","BT23-048","BT23-048",
  // Lv.4 (10)
  "BT23-027","BT23-027","BT23-027","BT23-027",
  "BT23-050","BT23-050","BT23-050","BT23-050",
  "BT23-101","BT23-101",
  // Lv.5 (6)
  "BT23-020","BT23-020","BT23-020",
  "BT23-032","BT23-032","BT23-032",
  "BT16-025","BT16-025","BT16-025",
  // Lv.6+ (0)
  // Tamers (6)
  "BT22-089","BT22-089","BT22-089",
  "BT23-081","BT23-081","BT23-081",
  "BT23-090","BT23-090",
  // Options (3)
  "BT22-099","BT22-099","BT22-099",
  // Misc
  "BT16-082","BT16-082","BT16-082","BT16-082",
];
