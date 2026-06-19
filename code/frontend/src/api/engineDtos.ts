// DTOs returned by the native desktop game engine bridge.

// ─── DTO types ────────────────────────────────────────────────────────

export interface EvoCostDto {
  color: number;
  level: number;
  cost: number;
}

export interface KeywordBreakdownDto {
  innate: string[];
  gained: string[];
}

export interface InheritedEffectDto {
  source_index: number;
  card_id: string;
  card_name: string;
  text: string;
}

export interface PermanentModifierDto {
  type: string;
  value: number;
  expiry: string;
  source_card_id: string | null;
}

export interface DpBreakdownDto {
  base: number | null;
  temporary: number;
  total: number | null;
}

export interface CardDto {
  card_id: string;
  card_name: string;
  card_kind: 'Digimon' | 'Tamer' | 'Option' | 'DigiEgg' | 'Dual';
  level: number | null;
  dp: number | null;
  play_cost: number;
  colors: string[];
  /** Printed digivolution costs. Absent on older engine builds. */
  evo_costs?: EvoCostDto[];
}

export interface SourceCardDto {
  card_id: string;
  card_name: string;
  colors: string[];
  /** This source's printed main effect text. Absent on older engine builds. */
  main_effect_text?: string;
  /** This source's printed inherited effect text. */
  inherited_effect_text?: string;
}

export interface PermanentDto {
  field_index: number;
  top_card: CardDto;
  effective_dp: number | null;
  is_suspended: boolean;
  stack_size: number;
  turn_played: number;
  /** Non-top digivolution sources, bottom→top (matches encode_source_select). */
  sources?: SourceCardDto[];
  /** Active keyword display names. Absent on older engine builds. */
  keywords?: string[];
  keyword_breakdown?: KeywordBreakdownDto;
  security_attack_modifier?: number;
  linked_card_ids?: string[];
  main_effect_text?: string;
  inherited_effects?: InheritedEffectDto[];
  modifiers?: PermanentModifierDto[];
  dp_breakdown?: DpBreakdownDto;
}

export interface PlayerDto {
  id: number;
  hand: CardDto[];
  battle_area: PermanentDto[];
  breeding: PermanentDto | null;
  deck_count: number;
  /** Cards remaining in the digitama (egg) deck. Absent on older engine builds. */
  egg_deck_count?: number;
  trash_count: number;
  /** Full trash contents (public zone). Empty/absent on older engine builds. */
  trash?: CardDto[];
  security_count: number;
  is_eliminated: boolean;
}

export interface EffectChoiceDto {
  index: number;
  label: string;
  action_id: number;
  /** Card whose effect this branch resolves — the chooser renders its image. */
  card_id?: string | null;
  card_name?: string | null;
}

export interface PendingSelectionDto {
  phase: string;
  selecting_player: number;
  valid_action_ids: number[];
  is_optional: boolean;
  prompt: string;
  /** Engine `SelectionKind` discriminant (e.g. "OwnField" / "OppField"). */
  kind?: string;
  /** Whose zone Hand/Trash action ids index into (engine 0/1). Absent =
   *  the selecting player's own zone. */
  zone_owner?: number | null;
  effect_choices?: EffectChoiceDto[];
}

export interface RevealedCardDto {
  card_id: string;
  owner: number;
}

export interface GameStateDto {
  turn_count: number;
  turn_player: number;
  /** Engine seat that takes the first turn (0 = local human / player1). */
  first_player: number;
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
