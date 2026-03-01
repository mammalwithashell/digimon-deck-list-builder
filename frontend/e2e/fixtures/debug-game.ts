import { type Page } from '@playwright/test';

const API_BASE = process.env.API_BASE ?? 'http://localhost:8000';

export interface DebugGameOptions {
  deck1: string[];
  deck2: string[];
  player1_type?: string;
  player2_type?: string;
  player1_policy?: string;
  player2_policy?: string;
  first_player?: number;
  skip_shuffle?: boolean;
  starting_hand1?: string[];
  starting_hand2?: string[];
  auto_mulligan?: string;
  initial_memory?: number;
  agent_action_delay_ms?: number;
}

export interface DebugGameHandle {
  gameId: string;
  sendAction: (actionId: number) => Promise<any>;
  stepGame: () => Promise<any>;
  setMemory: (memory: number) => Promise<void>;
  injectCard: (playerId: number, cardId: string, zone: string) => Promise<void>;
  getInternalState: () => Promise<any>;
  getActions: () => Promise<Record<string, string>>;
}

export async function createDebugGame(
  page: Page,
  options: DebugGameOptions,
): Promise<DebugGameHandle> {
  const resp = await page.request.post(`${API_BASE}/debug/games`, {
    data: {
      player1_type: 'human',
      player2_type: 'human',
      player1_policy: 'greedy',
      player2_policy: 'greedy',
      skip_shuffle: true,
      auto_mulligan: 'keep',
      initial_memory: 0,
      agent_action_delay_ms: 0,
      ...options,
    },
  });
  if (!resp.ok()) {
    const body = await resp.text();
    throw new Error(`Failed to create debug game: ${resp.status()} ${body}`);
  }
  const data = await resp.json();
  const gameId: string = data.game_id;

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
    async getInternalState() {
      const r = await page.request.get(`${API_BASE}/debug/games/${gameId}/internal-state`);
      return r.json();
    },
    async getActions() {
      const r = await page.request.get(`${API_BASE}/games/${gameId}/actions`);
      const data = await r.json();
      return data.actions;
    },
  };
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
