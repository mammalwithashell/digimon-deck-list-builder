export enum GamePhase {
  Start = 0,
  Draw = 1,
  Breeding = 2,
  Main = 3,
  End = 4,
  SelectTarget = 5,
  SelectMaterial = 6,
  BlockTiming = 7,
  CounterTiming = 8,
  SelectTrash = 9,
  SelectSource = 10,
  SelectHand = 11,
  SelectReveal = 12,
  SelectEffectChoice = 13,
  SelectSecurity = 14,
  EndOfTurnAction = 15,
  AllianceTiming = 16,
  Mulligan = 17,
  // Additional engine phases that were missing — without these,
  // `mapPhase` defaulted them to `Main`, so `PromptBar` /
  // `SelectionPanel` never dispatched the right UI.
  SelectPermutation = 18,
  SelectUnion = 19,
  SelectBudgeted = 20,
  SelectBreedingPermanent = 21,
  SelectPlayOrder = 22,
}

export interface SourceInfo {
  cardId: string;
  cardName: string | null;
  isTop: boolean;
  optState: number;
  dpContribution: number;
  mainEffectText: string;
  inheritedEffectText: string;
  colors: number[];
}

export interface InheritedEffectInfo {
  sourceIndex: number;
  cardId: string;
  cardName: string | null;
  text: string;
}

export interface KeywordBreakdown {
  innate: string[];
  gained: string[];
}

export interface DpSourceContribution {
  cardId: string;
  cardName: string | null;
  value: number;
}

export interface DpBreakdown {
  base: number | null;
  sources: DpSourceContribution[];
  temporary: number;
  total: number | null;
}

/** A single active modifier on a permanent (immunity, restriction, stat
 * change, etc.), emitted structurally by the engine's `to_ui_json`. The
 * human label and grouping are derived frontend-side from `type` via
 * `MODIFIER_DISPLAY`. */
export interface PermanentModifier {
  /** Stable modifier-type string (e.g. "CannotBeDestroyed", "ChangeDp"). */
  type: string;
  /** Signed magnitude for stat-change modifiers; 0 for flag-style ones. */
  value: number;
  /** When the modifier wears off (e.g. "Permanent", "EndOfTurn"). */
  expiry: string;
  /** Top-card id of the permanent that applied this modifier, if known. */
  sourceCardId: string | null;
}

export interface PermanentInfo {
  topCardId: string | null;
  topCardName: string | null;
  dp: number;
  level: number | null;
  isSuspended: boolean;
  sourceCount: number;
  keywords: string[];
  keywordBreakdown: KeywordBreakdown;
  securityAttackModifier: number;
  linkedCardIds: string[];
  sources: SourceInfo[];
  mainEffectText: string;
  inheritedEffects: InheritedEffectInfo[];
  modifiers: PermanentModifier[];
  dpBreakdown: DpBreakdown;
  turnPlayed: number;
  colors: number[];
}

export interface EffectChoice {
  index: number;
  cardId: string;
  cardName: string;
  label: string;
  /** Concrete engine action ID for this branch. When present, the
   *  SelectionPanel should dispatch this directly rather than computing
   *  from `EFFECT_CHOICE_START + index` (engine uses HAND_EFFECT_START
   *  range, not the frontend's 1000-range — they don't match). */
  actionId?: number;
}

export interface KeywordPrompt {
  keyword: string;
  cardId: string;
  cardName: string;
}

export interface PendingSelection {
  phase: GamePhase;
  validIndices: number[];
  isOptional: boolean;
  prompt: string;
  selectingPlayer: number;
  /**
   * Engine `SelectionKind` (e.g. `OwnField`, `OppField`, `Hand`, `Trash`).
   * For field selections it is the ONLY signal of which player's field the
   * `validIndices` (`OWN_FIELD_START + slot`) refer to — the engine encodes
   * both sides in the same id range. See `utils/selectionTargets.ts`.
   */
  kind?: string;
  /**
   * For `Hand`/`Trash` kinds, the player whose zone `validIndices` index
   * into (UI convention: 1 = you, 2 = opponent). Effects like EX11-012
   * Medusamon prompt YOU to pick from the OPPONENT's trash — without this
   * the panel zips the action ids against the wrong card list. Absent =
   * the selecting player's own zone.
   */
  zoneOwner?: number;
  effectChoices?: EffectChoice[];
  keywordPrompt?: KeywordPrompt;
}

export interface PendingAttack {
  attackerSlot: number;
  targetSlot: number;
}

export interface HandCardInfo {
  cardId: string;
  cardName: string;
  playCost: number;
  level: number | null;
  dp: number | null;
  colors: number[];
  cardKind: number; // 0=Digimon, 1=Tamer, 2=Option/Dual, 3=DigiEgg
  evoCosts: { color: number; level: number; cost: number }[];
}

export interface PlayerState {
  id: number;
  memory: number;
  handCount: number;
  handIds: string[];
  handCards: HandCardInfo[];
  securityCount: number;
  securityIds: string[];
  deckCount: number;
  eggDeckCount: number;
  battleAreaCount: number;
  battleArea: PermanentInfo[];
  breedingArea: PermanentInfo | null;
  trashIds: string[];
}

export interface GameEvent {
  type: string;
  seq: number;
  player: number;
  source_card_id: string | null;
  source_card_name: string | null;
  source_slot: number | null;
  target_card_id: string | null;
  target_card_name: string | null;
  target_slot: number | null;
  meta: Record<string, unknown>;
}

export type ActionActor = 'human' | 'agent_greedy' | 'agent_trained' | string;

export type DecodedActionKind =
  | 'play'
  | 'hand_effect'
  | 'hatch'
  | 'move'
  | 'pass'
  | 'dna_digivolve'
  | 'attack'
  | 'digivolve'
  | 'field_effect'
  | 'trash_effect'
  | 'source_select'
  | 'selection'
  | 'unknown';

export type DecodedActionZone =
  | 'hand'
  | 'battle'
  | 'breeding'
  | 'security'
  | 'trash'
  | 'source'
  | 'revealed'
  | 'effect_choice';

export interface DecodedAction {
  actionId: number;
  playerId: number;
  phase: string;
  kind: DecodedActionKind;
  label: string;
  sourceZone: DecodedActionZone | null;
  sourceIndex: number | null;
  targetZone: DecodedActionZone | null;
  targetIndex: number | null;
  cardId: string | null;
  cardName: string | null;
}

export interface TensorSummary {
  playerId: number;
  profileId: string;
  profileVersion: number;
  tensorSize: number;
  maskSize: number;
  legalActionCount: number;
  cardIdSlotCount: number;
  scalarSlotCount: number;
  turnCount: number;
  phase: string;
  memory: number;
  tensorHead: number[];
}

export interface ActionTrace {
  actor: ActionActor;
  playerId: number;
  actionId: number;
  decoded: DecodedAction;
  tensorSummary: TensorSummary | null;
}

export interface GameState {
  turnCount: number;
  currentPhase: GamePhase;
  currentPlayer: number;
  memoryGauge: number;
  isGameOver: boolean;
  winner: number | null;
  player1: PlayerState;
  player2: PlayerState;
  revealedCards: { cardId: string; owner: number }[];
  pendingSelection: PendingSelection | null;
  pendingAttack: PendingAttack | null;
}
