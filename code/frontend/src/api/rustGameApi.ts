// Rust-engine adapter for the game API.
//
// Implements the same function surface as `gameApi.ts` but dispatches
// through Tauri `invoke()` to the in-process `digimon-engine` crate.
// Selected at build time via `VITE_BUILD_TARGET=desktop` — see `gameApi.ts`.
//
// The Rust `GameStateDto` carries less detail than the Python `GameState`
// (no DP breakdowns, keyword metadata, source lists, etc.). The translator
// below fills the richer frontend shape with safe defaults so the store and
// components render without errors; unmodeled affordances (keyword chips,
// DP tooltips) will appear empty until the Rust engine surfaces them.

import { invoke } from '@tauri-apps/api/core';
import type {
  ActionTrace,
  DecodedAction,
  GameEvent,
  GameState,
  PermanentInfo,
  PlayerState,
  TensorSummary,
} from '@/types/game';
import { GamePhase } from '@/types/game';
import type {
  CardDto,
  GameStateDto,
  PermanentDto,
  PlayerDto,
} from './rustEngine';

// ─── Rust-side response envelopes ────────────────────────────────────

interface RustCreateGameResponse {
  game_id: string;
  state: GameStateDto;
  action_mask: number[];
}

interface RustActionResponse {
  state: GameStateDto;
  action_mask: number[];
  is_game_over: boolean;
  logs: string[];
  events: GameEvent[];
  action_context: Record<string, unknown>;
  action_traces?: RustActionTrace[];
}

interface RustStepResponse {
  state: GameStateDto;
  action_mask: number[];
  logs: string[];
  events: GameEvent[];
  is_human_turn: boolean;
  is_game_over: boolean;
  action_traces?: RustActionTrace[];
}

interface RustSurrenderResponse {
  state: GameStateDto;
  action_mask: number[];
  logs: string[];
  events: GameEvent[];
  is_game_over: boolean;
  surrendered_by: number;
}

interface RustDecodedAction {
  action_id: number;
  player_id: number;
  phase: string;
  kind: DecodedAction['kind'];
  label: string;
  source_zone: DecodedAction['sourceZone'];
  source_index: number | null;
  target_zone: DecodedAction['targetZone'];
  target_index: number | null;
  card_id: string | null;
  card_name: string | null;
}

interface RustTensorSummary {
  player_id: number;
  tensor_size: number;
  mask_size: number;
  legal_action_count: number;
  turn_count: number;
  phase: string;
  memory: number;
  tensor_head: number[];
}

interface RustActionTrace {
  actor: string;
  player_id: number;
  action_id: number;
  decoded: RustDecodedAction;
  tensor_summary?: RustTensorSummary | null;
}

// ─── Frontend-facing response shapes (must mirror gameApi.ts) ────────

type PlayerKind = 'human' | 'greedy' | 'trained';

interface CreateGameParams {
  deck1?: string[];
  deck2?: string[];
  deck1_raw?: string;
  deck2_raw?: string;
  player1_type?: string;
  player2_type?: string;
  player1_policy?: string;
  player2_policy?: string;
  agent_action_delay_ms?: number;
  player_kinds?: PlayerKind[];
  player_model_ids?: (string | null)[];
}

interface CreateGameResponse {
  game_id: string;
  state: GameState;
  action_mask: number[];
  recording_metadata?: Record<string, unknown>;
  player_labels?: Record<number, string>;
}

interface ActionResponse {
  state: GameState;
  action_mask: number[];
  is_game_over: boolean;
  logs?: string[];
  events?: GameEvent[];
  action_context?: Record<string, unknown>;
  action_traces?: ActionTrace[];
}

interface StepResponse {
  state: GameState;
  action_mask: number[];
  logs: string[];
  events?: GameEvent[];
  is_human_turn: boolean;
  is_game_over: boolean;
  action_traces?: ActionTrace[];
}

interface SurrenderResponse {
  state: GameState;
  action_mask: number[];
  logs: string[];
  events?: GameEvent[];
  is_game_over: boolean;
  surrendered_by: number;
}

// ─── DTO → frontend state translation ────────────────────────────────

// Maps the Rust phase string to the frontend `GamePhase` enum. Rust's
// string set is a superset (e.g. `Start`/`End` are split into `Unsuspend`/
// `Draw`/`EndTurn`); we map everything sensible and fall through to `Main`
// so the UI keeps rendering.
function mapPhase(phaseStr: string): GamePhase {
  switch (phaseStr) {
    case 'Mulligan':
      return GamePhase.Mulligan;
    case 'Unsuspend':
    case 'Draw':
      return GamePhase.Draw;
    case 'Breeding':
      return GamePhase.Breeding;
    case 'Main':
      return GamePhase.Main;
    case 'EndTurn':
    case 'EndOfTurnAction':
      return GamePhase.EndOfTurnAction;
    case 'SelectTarget':
      return GamePhase.SelectTarget;
    case 'SelectMaterial':
      return GamePhase.SelectMaterial;
    case 'SelectTrash':
      return GamePhase.SelectTrash;
    case 'SelectSource':
      return GamePhase.SelectSource;
    case 'SelectHand':
      return GamePhase.SelectHand;
    case 'SelectReveal':
      return GamePhase.SelectReveal;
    case 'SelectSecurity':
      return GamePhase.SelectSecurity;
    case 'EffectChoice':
      return GamePhase.SelectEffectChoice;
    case 'BlockTiming':
      return GamePhase.BlockTiming;
    case 'CounterTiming':
      return GamePhase.CounterTiming;
    case 'AllianceTiming':
      return GamePhase.AllianceTiming;
    default:
      return GamePhase.Main;
  }
}

const CARD_KIND_INDEX: Record<CardDto['card_kind'], number> = {
  Digimon: 0,
  Tamer: 1,
  Option: 2,
  DigiEgg: 3,
};

const COLOR_INDEX: Record<string, number> = {
  Red: 0,
  Blue: 1,
  Yellow: 2,
  Green: 3,
  Black: 4,
  Purple: 5,
  White: 6,
};

function mapColors(colors: string[]): number[] {
  return colors.map((c) => COLOR_INDEX[c] ?? -1).filter((i) => i >= 0);
}

function toPermanentInfo(perm: PermanentDto): PermanentInfo {
  const top = perm.top_card;
  return {
    topCardId: top.card_id,
    topCardName: top.card_name,
    dp: perm.effective_dp ?? 0,
    level: top.level,
    isSuspended: perm.is_suspended,
    sourceCount: perm.stack_size,
    keywords: [],
    keywordBreakdown: { innate: [], gained: [] },
    securityAttackModifier: 0,
    linkedCardIds: [],
    sources: [
      {
        cardId: top.card_id,
        cardName: top.card_name,
        isTop: true,
        optState: 0,
        dpContribution: perm.effective_dp ?? 0,
        mainEffectText: '',
        inheritedEffectText: '',
        colors: mapColors(top.colors),
      },
    ],
    mainEffectText: '',
    inheritedEffects: [],
    dpBreakdown: {
      base: top.dp,
      sources: [],
      temporary: 0,
      total: perm.effective_dp ?? top.dp,
    },
    turnPlayed: perm.turn_played,
    colors: mapColors(top.colors),
  };
}

function toPlayerState(player: PlayerDto, memory: number): PlayerState {
  const battleArea = player.battle_area.map(toPermanentInfo);
  return {
    id: player.id,
    memory,
    handCount: player.hand.length,
    handIds: player.hand.map((c) => c.card_id),
    handCards: player.hand.map((c) => ({
      cardId: c.card_id,
      cardName: c.card_name,
      playCost: c.play_cost,
      level: c.level,
      dp: c.dp,
      colors: mapColors(c.colors),
      cardKind: CARD_KIND_INDEX[c.card_kind] ?? 0,
      evoCosts: [],
    })),
    securityCount: player.security_count,
    securityIds: [],
    deckCount: player.deck_count,
    eggDeckCount: 0,
    battleAreaCount: battleArea.length,
    battleArea,
    breedingArea: player.breeding ? toPermanentInfo(player.breeding) : null,
    trashIds: [],
  };
}

export function dtoToGameState(dto: GameStateDto): GameState {
  const player1: PlayerDto | undefined =
    dto.players.find((p) => p.id === 0) ?? dto.players[0];
  if (!player1) {
    throw new Error('Rust engine returned a game state with no players');
  }
  const player2: PlayerDto =
    dto.players.find((p) => p.id === 1) ?? dto.players[1] ?? player1;
  const memory0 = dto.turn_player === 0 ? dto.memory : -dto.memory;
  return {
    turnCount: dto.turn_count,
    currentPhase: mapPhase(dto.current_phase),
    currentPlayer: dto.turn_player,
    memoryGauge: dto.memory,
    isGameOver: dto.game_over,
    winner: dto.winner,
    player1: toPlayerState(player1, memory0),
    player2: toPlayerState(player2, -memory0),
    revealedCards: [],
    pendingSelection: null,
    pendingAttack: null,
  };
}

export function toTensorSummary(summary: RustTensorSummary): TensorSummary {
  return {
    playerId: summary.player_id,
    tensorSize: summary.tensor_size,
    maskSize: summary.mask_size,
    legalActionCount: summary.legal_action_count,
    turnCount: summary.turn_count,
    phase: summary.phase,
    memory: summary.memory,
    tensorHead: summary.tensor_head,
  };
}

export function toDecodedAction(action: RustDecodedAction): DecodedAction {
  return {
    actionId: action.action_id,
    playerId: action.player_id,
    phase: action.phase,
    kind: action.kind,
    label: action.label,
    sourceZone: action.source_zone,
    sourceIndex: action.source_index,
    targetZone: action.target_zone,
    targetIndex: action.target_index,
    cardId: action.card_id,
    cardName: action.card_name,
  };
}

export function toActionTrace(trace: RustActionTrace): ActionTrace {
  return {
    actor: trace.actor,
    playerId: trace.player_id,
    actionId: trace.action_id,
    decoded: toDecodedAction(trace.decoded),
    tensorSummary: trace.tensor_summary
      ? toTensorSummary(trace.tensor_summary)
      : null,
  };
}

export function toActionTraces(
  traces: RustActionTrace[] | undefined,
): ActionTrace[] | undefined {
  return traces?.map(toActionTrace);
}

// ─── Commands (mirror gameApi.ts exports) ─────────────────────────────

export async function createGame(
  params: CreateGameParams,
): Promise<CreateGameResponse> {
  // Rust backend accepts optional `player_kinds` / `player_model_ids`.
  // If the caller passed those directly, forward verbatim. Otherwise fall
  // back to deriving them from the legacy string-typed fields the hosted
  // API takes, so existing call sites work without modification.
  const kinds = params.player_kinds ?? deriveKinds(params);
  const modelIds = params.player_model_ids ?? [null, null];
  const resp = await invoke<RustCreateGameResponse>('rust_create_game', {
    deck1: params.deck1 ?? null,
    deck2: params.deck2 ?? null,
    playerKinds: kinds ?? null,
    playerModelIds: modelIds,
  });
  return {
    game_id: resp.game_id,
    state: dtoToGameState(resp.state),
    action_mask: resp.action_mask,
  };
}

function deriveKinds(params: CreateGameParams): PlayerKind[] | null {
  if (!params.player1_type && !params.player2_type) return null;
  return [
    toKind(params.player1_type, params.player1_policy),
    toKind(params.player2_type, params.player2_policy),
  ];
}

function toKind(type: string | undefined, policy: string | undefined): PlayerKind {
  if (type === 'human' || !type) return 'human';
  if (policy === 'trained') return 'trained';
  return 'greedy';
}

export async function sendAction(
  _gameId: string,
  action: number,
): Promise<ActionResponse> {
  const resp = await invoke<RustActionResponse>('rust_submit_action', {
    action,
  });
  return {
    state: dtoToGameState(resp.state),
    action_mask: resp.action_mask,
    is_game_over: resp.is_game_over,
    logs: resp.logs,
    events: resp.events,
    action_context: resp.action_context,
    action_traces: toActionTraces(resp.action_traces),
  };
}

export async function stepGame(_gameId: string): Promise<StepResponse> {
  const resp = await invoke<RustStepResponse>('rust_step_game');
  return {
    state: dtoToGameState(resp.state),
    action_mask: resp.action_mask,
    logs: resp.logs,
    events: resp.events,
    is_human_turn: resp.is_human_turn,
    is_game_over: resp.is_game_over,
    action_traces: toActionTraces(resp.action_traces),
  };
}

export async function getBoardTensorSummary(
  _gameId: string,
  playerId: number,
): Promise<TensorSummary> {
  const summary = await invoke<RustTensorSummary | null>(
    'rust_get_board_tensor_summary',
    { playerId },
  );
  if (!summary) {
    throw new Error('Rust engine returned no board tensor summary');
  }
  return toTensorSummary(summary);
}

export async function getState(_gameId: string): Promise<GameState> {
  const dto = await invoke<GameStateDto>('get_rust_game_state');
  return dtoToGameState(dto);
}

export async function getMask(_gameId: string): Promise<number[]> {
  return invoke<number[]>('rust_get_mask');
}

export async function getLog(_gameId: string): Promise<string[]> {
  return invoke<string[]>('rust_get_log');
}

export async function surrenderGame(
  _gameId: string,
  playerId: number,
): Promise<SurrenderResponse> {
  const resp = await invoke<RustSurrenderResponse>('rust_surrender', {
    playerId,
  });
  return {
    state: dtoToGameState(resp.state),
    action_mask: resp.action_mask,
    logs: resp.logs,
    events: resp.events,
    is_game_over: resp.is_game_over,
    surrendered_by: resp.surrendered_by,
  };
}

export async function deleteGame(_gameId: string): Promise<void> {
  await invoke('rust_delete_game');
}
