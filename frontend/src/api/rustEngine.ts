// Frontend adapter for the Rust-native digimon engine.
//
// These commands run entirely in-process via Tauri (no HTTP sidecar). They
// expose the digimon-engine crate directly. See src-tauri/src/engine_commands.rs
// for the backend implementation.
//
// Only available inside the Tauri desktop app. For hosted web, use gameApi.ts.

import { invoke } from '@tauri-apps/api/core';

// ─── DTO types ────────────────────────────────────────────────────────

export interface CardDto {
  card_id: string;
  card_name: string;
  card_kind: 'Digimon' | 'Tamer' | 'Option' | 'DigiEgg';
  level: number | null;
  dp: number | null;
  play_cost: number;
  colors: string[];
}

export interface PermanentDto {
  field_index: number;
  top_card: CardDto;
  effective_dp: number | null;
  is_suspended: boolean;
  stack_size: number;
  turn_played: number;
}

export interface PlayerDto {
  id: number;
  hand: CardDto[];
  battle_area: PermanentDto[];
  breeding: PermanentDto | null;
  deck_count: number;
  trash_count: number;
  security_count: number;
  is_eliminated: boolean;
}

export interface GameStateDto {
  turn_count: number;
  turn_player: number;
  current_phase: string;
  memory: number;
  game_over: boolean;
  winner: number | null;
  players: PlayerDto[];
  /** Player expected to make the next mulligan decision, or null if done. */
  mulligan_current_player: number | null;
  /** Whether each player has used their one redraw. Indexed by player id. */
  mulligan_used: boolean[];
}

export type AttackResult =
  | 'Invalid'
  | 'AttackerWins'
  | 'DefenderWins'
  | 'MutualDestruction'
  | 'SecurityCheckSurvived'
  | 'AttackerDeletedBySecurity'
  | 'GameWon';

export interface AttackResultDto {
  result: AttackResult;
  state: GameStateDto;
}

// ─── Commands ─────────────────────────────────────────────────────────

/** Create a new 2-player game pre-loaded with test cards. Returns initial state. */
export async function createTestGame(): Promise<GameStateDto> {
  return invoke<GameStateDto>('create_test_game');
}

/** Read the current game state. */
export async function getRustGameState(): Promise<GameStateDto> {
  return invoke<GameStateDto>('get_rust_game_state');
}

/** Play a card from a player's hand. */
export async function rustPlayCard(
  playerId: number,
  handIndex: number,
): Promise<GameStateDto> {
  return invoke<GameStateDto>('rust_play_card', {
    playerId,
    handIndex,
  });
}

/** Attack an opposing Digimon. */
export async function rustAttackDigimon(
  attackerPlayer: number,
  attackerIndex: number,
  defenderPlayer: number,
  defenderIndex: number,
): Promise<AttackResultDto> {
  return invoke<AttackResultDto>('rust_attack_digimon', {
    attackerPlayer,
    attackerIndex,
    defenderPlayer,
    defenderIndex,
  });
}

/** Attack the opposing player (security check). */
export async function rustAttackPlayer(
  attackerPlayer: number,
  attackerIndex: number,
  defenderPlayer: number,
): Promise<AttackResultDto> {
  return invoke<AttackResultDto>('rust_attack_player', {
    attackerPlayer,
    attackerIndex,
    defenderPlayer,
  });
}

/** End the current turn (without passing memory). */
export async function rustEndTurn(): Promise<GameStateDto> {
  return invoke<GameStateDto>('rust_end_turn');
}

/** Pass turn (memory -> -3, then end turn). */
export async function rustPassTurn(): Promise<GameStateDto> {
  return invoke<GameStateDto>('rust_pass_turn');
}

/** Hatch: move top egg to breeding area. */
export async function rustHatch(playerId: number): Promise<GameStateDto> {
  return invoke<GameStateDto>('rust_hatch', { playerId });
}

/** Move from breeding to battle area. */
export async function rustMoveFromBreeding(
  playerId: number,
): Promise<GameStateDto> {
  return invoke<GameStateDto>('rust_move_from_breeding', { playerId });
}

/**
 * Apply a mulligan decision for the currently-deciding player.
 * `keep = true` keeps the opening hand; `keep = false` shuffles the hand
 * back into the deck and redraws. Only valid while `mulligan_current_player`
 * is non-null in the game state.
 */
export async function rustMulliganDecide(
  keep: boolean,
): Promise<GameStateDto> {
  return invoke<GameStateDto>('rust_mulligan_decide', { keep });
}
