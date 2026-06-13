// DTOs returned by the native desktop game engine bridge.

// ─── DTO types ────────────────────────────────────────────────────────

export interface CardDto {
  card_id: string;
  card_name: string;
  card_kind: 'Digimon' | 'Tamer' | 'Option' | 'DigiEgg' | 'Dual';
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

export interface EffectChoiceDto {
  index: number;
  label: string;
  action_id: number;
}

export interface PendingSelectionDto {
  phase: string;
  selecting_player: number;
  valid_action_ids: number[];
  is_optional: boolean;
  prompt: string;
  /** SelectionKind variant string (e.g. "OwnField"/"OppField"). */
  kind?: string;
  effect_choices?: EffectChoiceDto[];
}

export interface RevealedCardDto {
  card_id: string;
  owner: number;
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
  /** `null` outside selection phases; non-null when the engine is parked
   *  on a human-driven choice. Read by SelectionPanel / PromptBar. */
  pending_selection?: PendingSelectionDto | null;
  /** Cards revealed during the most recent effect; rendered by
   *  `RevealedCardsZone`. */
  revealed_cards?: RevealedCardDto[];
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
