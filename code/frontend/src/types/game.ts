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
  dpBreakdown: DpBreakdown;
  turnPlayed: number;
  colors: number[];
}

export interface EffectChoice {
  index: number;
  cardId: string;
  cardName: string;
  label: string;
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
  cardKind: number; // 0=Digimon, 1=Tamer, 2=Option, 3=DigiEgg
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
  source_slot: number | null;
  target_card_id: string | null;
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
  tensorSize: number;
  maskSize: number;
  legalActionCount: number;
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
