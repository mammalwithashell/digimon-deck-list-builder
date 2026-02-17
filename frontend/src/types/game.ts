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
}

export interface SourceInfo {
  cardId: string;
  optState: number;
  dpContribution: number;
}

export interface PermanentInfo {
  topCardId: string | null;
  topCardName: string | null;
  dp: number;
  level: number | null;
  isSuspended: boolean;
  sourceCount: number;
  keywords: string[];
  securityAttackModifier: number;
  linkedCardIds: string[];
  sources: SourceInfo[];
  turnPlayed: number;
  colors: number[];
}

export interface PendingSelection {
  phase: GamePhase;
  validIndices: number[];
  isOptional: boolean;
  prompt: string;
  selectingPlayer: number;
}

export interface PendingAttack {
  attackerSlot: number;
  targetSlot: number;
}

export interface PlayerState {
  id: number;
  memory: number;
  handCount: number;
  handIds: string[];
  securityCount: number;
  securityIds: string[];
  deckCount: number;
  eggDeckCount: number;
  battleAreaCount: number;
  battleArea: PermanentInfo[];
  breedingArea: PermanentInfo | null;
  trashIds: string[];
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
