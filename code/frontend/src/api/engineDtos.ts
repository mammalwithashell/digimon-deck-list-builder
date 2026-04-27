// DTOs returned by the native desktop game engine bridge.

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
