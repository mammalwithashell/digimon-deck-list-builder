// Native desktop game API.
//
// Gameplay is now routed directly through Tauri `invoke()` to the in-process
// engine. There is no hosted gameplay fallback in this module.
//
// The engine DTO carries less detail than the full frontend `GameState`
// (no DP breakdowns, keyword metadata, source lists, etc.). The translator
// below fills the richer frontend shape with safe defaults so the store and
// components render without errors; unmodeled affordances (keyword chips,
// DP tooltips) will appear empty until the engine surfaces them.

import { invoke } from '@tauri-apps/api/core';
import { httpJson, isInTauriRuntime } from './engineRuntime';
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
} from './engineDtos';

// ─── Engine response envelopes ────────────────────────────────────────

interface CreateGameCommandResponse {
  game_id: string;
  state: GameStateDto;
  action_mask: number[];
}

interface ActionCommandResponse {
  state: GameStateDto;
  action_mask: number[];
  is_game_over: boolean;
  logs: string[];
  events: GameEvent[];
  action_context: Record<string, unknown>;
  action_traces?: ActionTraceDto[];
}

interface StepCommandResponse {
  state: GameStateDto;
  action_mask: number[];
  logs: string[];
  events: GameEvent[];
  is_human_turn: boolean;
  is_game_over: boolean;
  action_traces?: ActionTraceDto[];
}

interface SurrenderCommandResponse {
  state: GameStateDto;
  action_mask: number[];
  logs: string[];
  events: GameEvent[];
  is_game_over: boolean;
  surrendered_by: number;
}

interface DecodedActionDto {
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

interface TensorSummaryDto {
  player_id: number;
  profile_id: string;
  profile_version: number;
  tensor_size: number;
  mask_size: number;
  legal_action_count: number;
  card_id_slot_count: number;
  scalar_slot_count: number;
  turn_count: number;
  phase: string;
  memory: number;
  tensor_head: number[];
}

interface ActionTraceDto {
  actor: string;
  player_id: number;
  action_id: number;
  decoded: DecodedActionDto;
  tensor_summary?: TensorSummaryDto | null;
}

// ─── Frontend-facing response shapes ─────────────────────────────────

export type PlayerKind = 'human' | 'greedy' | 'trained';

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

// Maps the engine phase string to the frontend `GamePhase` enum. The engine's
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
    case 'SelectPermutation':
      return GamePhase.SelectPermutation;
    case 'SelectUnion':
      return GamePhase.SelectUnion;
    case 'SelectBudgeted':
      return GamePhase.SelectBudgeted;
    case 'SelectBreedingPermanent':
      return GamePhase.SelectBreedingPermanent;
    case 'SelectPlayOrder':
      return GamePhase.SelectPlayOrder;
    default:
      return GamePhase.Main;
  }
}

const CARD_KIND_INDEX: Record<CardDto['card_kind'], number> = {
  Digimon: 0,
  Tamer: 1,
  Option: 2,
  DigiEgg: 3,
  Dual: 2,
};

function mapCardKind(cardKind: CardDto['card_kind'] | string): number {
  return CARD_KIND_INDEX[cardKind as CardDto['card_kind']] ?? 2;
}

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
      cardKind: mapCardKind(c.card_kind),
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
    throw new Error('Engine returned a game state with no players');
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
    revealedCards: (dto.revealed_cards ?? []).map((rc) => ({
      cardId: rc.card_id,
      owner: rc.owner,
    })),
    pendingSelection: dto.pending_selection
      ? {
          phase: mapPhase(dto.pending_selection.phase),
          validIndices: dto.pending_selection.valid_action_ids,
          isOptional: dto.pending_selection.is_optional,
          prompt: dto.pending_selection.prompt,
          // Engine is 0-based (player_id 0/1); frontend convention is
          // 1-based (1 = "you", 2 = "opponent"), and PromptBar's
          // `localPlayer` is hardcoded to 1. Without this +1, every
          // engine-0 pending selection would render as "Waiting for
          // opponent…" when it's actually the user's turn.
          selectingPlayer: dto.pending_selection.selecting_player + 1,
          // EffectChoice branches need to thread through with their actual
          // engine `action_id`s; the frontend's broken `EFFECT_CHOICE_START`
          // range scan can't find them otherwise.
          effectChoices: dto.pending_selection.effect_choices?.map((c) => ({
            index: c.index,
            cardId: `effect-${c.index}`,
            cardName: c.label,
            label: c.label,
            // Pass engine action_id through so SelectionPanel can dispatch
            // it directly without recomputing from index.
            actionId: c.action_id,
          })),
        }
      : null,
    pendingAttack: null,
  };
}

export function toTensorSummary(summary: TensorSummaryDto): TensorSummary {
  return {
    playerId: summary.player_id,
    profileId: summary.profile_id,
    profileVersion: summary.profile_version,
    tensorSize: summary.tensor_size,
    maskSize: summary.mask_size,
    legalActionCount: summary.legal_action_count,
    cardIdSlotCount: summary.card_id_slot_count,
    scalarSlotCount: summary.scalar_slot_count,
    turnCount: summary.turn_count,
    phase: summary.phase,
    memory: summary.memory,
    tensorHead: summary.tensor_head,
  };
}

export function toDecodedAction(action: DecodedActionDto): DecodedAction {
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

export function toActionTrace(trace: ActionTraceDto): ActionTrace {
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
  traces: ActionTraceDto[] | undefined,
): ActionTrace[] | undefined {
  return traces?.map(toActionTrace);
}

// ─── Commands (mirror gameApi.ts exports) ─────────────────────────────

export async function createGame(
  params: CreateGameParams,
): Promise<CreateGameResponse> {
  // Browser-dev path: hit the FastAPI `/games` endpoint. The Rust
  // engine's `to_ui_json()` already returns the camelCase `GameState`
  // shape so no DTO translation is needed; the response slots directly
  // into the store.
  if (!isInTauriRuntime()) {
    const kinds = params.player_kinds ?? deriveKinds(params);
    const p1Human = kinds ? kinds[0] === 'human' : false;
    const p2Human = kinds ? kinds[1] === 'human' : false;
    const body = {
      deck1: params.deck1 ?? [],
      deck2: params.deck2 ?? [],
      deck1_raw: params.deck1_raw,
      deck2_raw: params.deck2_raw,
      player1_type: p1Human ? 'human' : 'agent',
      player2_type: p2Human ? 'human' : 'agent',
      player1_policy: 'greedy',
      player2_policy: 'greedy',
    };
    const httpResp = await httpJson<{
      game_id: string;
      state: GameState;
      action_mask: number[];
    }>('/games', { method: 'POST', body });
    return {
      game_id: httpResp.game_id,
      state: httpResp.state,
      action_mask: httpResp.action_mask,
    };
  }

  // Desktop path: Tauri invoke into the in-process Rust engine.
  // Engine accepts optional `player_kinds` / `player_model_ids`.
  // If the caller passed those directly, forward verbatim. Otherwise fall
  // back to deriving them from the legacy string-typed fields so existing
  // call sites work without modification.
  const kinds = params.player_kinds ?? deriveKinds(params);
  const modelIds = params.player_model_ids ?? [null, null];
  const resp = await invoke<CreateGameCommandResponse>('rust_create_game', {
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

export async function createVsAgentGame(params: {
  modelId: string;
  userDeck: { main_deck: string[]; egg_deck: string[] };
  opponentDeck: { main_deck: string[]; egg_deck: string[] };
}): Promise<{ game_id: string }> {
  const resp = await createGame({
    deck1: [...params.userDeck.egg_deck, ...params.userDeck.main_deck],
    deck2: [...params.opponentDeck.egg_deck, ...params.opponentDeck.main_deck],
    player_kinds: ['human', 'trained'],
    player_model_ids: [null, params.modelId],
  });
  return { game_id: resp.game_id };
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
  gameId: string,
  action: number,
): Promise<ActionResponse> {
  if (!isInTauriRuntime()) {
    const httpResp = await httpJson<{
      state: GameState;
      action_mask: number[];
      is_game_over: boolean;
      logs: string[];
      events: GameEvent[];
      action_context?: Record<string, unknown>;
    }>(`/games/${gameId}/actions`, { method: 'POST', body: { action } });
    return {
      state: httpResp.state,
      action_mask: httpResp.action_mask,
      is_game_over: httpResp.is_game_over,
      logs: httpResp.logs,
      events: httpResp.events,
      action_context: httpResp.action_context,
      // Browser-dev path doesn't currently surface action_traces — those
      // are produced by the Tauri command's per-step `explain_action`
      // call. Acceptable trade-off; the ticker stays empty in browser
      // mode but the game itself works.
      action_traces: [],
    };
  }
  const resp = await invoke<ActionCommandResponse>('rust_submit_action', {
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

export async function stepGame(gameId: string): Promise<StepResponse> {
  if (!isInTauriRuntime()) {
    const httpResp = await httpJson<{
      state: GameState;
      action_mask: number[];
      is_game_over: boolean;
      logs: string[];
      events: GameEvent[];
      is_human_turn: boolean;
    }>(`/games/${gameId}/steps`, { method: 'POST' });
    return {
      state: httpResp.state,
      action_mask: httpResp.action_mask,
      logs: httpResp.logs,
      events: httpResp.events,
      is_human_turn: httpResp.is_human_turn,
      is_game_over: httpResp.is_game_over,
      action_traces: [],
    };
  }
  const resp = await invoke<StepCommandResponse>('rust_step_game');
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
  const summary = await invoke<TensorSummaryDto | null>(
    'rust_get_board_tensor_summary',
    { playerId },
  );
  if (!summary) {
    throw new Error('Engine returned no board tensor summary');
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
  const resp = await invoke<SurrenderCommandResponse>('rust_surrender', {
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
