import { create } from 'zustand';
import type {
  GameState,
  GamePhase,
  PlayerState,
  PendingSelection,
  PendingAttack,
} from '@/types/game';

interface GameStore {
  // Game session
  gameId: string | null;

  // Game state
  turnCount: number;
  currentPhase: GamePhase;
  currentPlayer: number;
  memoryGauge: number;
  isGameOver: boolean;
  winner: number | null;
  player1: PlayerState | null;
  player2: PlayerState | null;
  revealedCards: { cardId: string; owner: number }[];
  pendingSelection: PendingSelection | null;
  pendingAttack: PendingAttack | null;

  // UI state
  actionMask: number[];
  selectedAttacker: number | null;
  hoveredCard: string | null;
  logs: string[];

  // Actions
  setGameId: (id: string | null) => void;
  setGameState: (state: GameState) => void;
  setActionMask: (mask: number[]) => void;
  selectAttacker: (slot: number | null) => void;
  setHoveredCard: (cardId: string | null) => void;
  appendLogs: (newLogs: string[]) => void;
  clearLogs: () => void;
  reset: () => void;
}

const initialState = {
  gameId: null,
  turnCount: 0,
  currentPhase: 0 as GamePhase,
  currentPlayer: 0,
  memoryGauge: 0,
  isGameOver: false,
  winner: null,
  player1: null,
  player2: null,
  revealedCards: [],
  pendingSelection: null,
  pendingAttack: null,
  actionMask: [],
  selectedAttacker: null,
  hoveredCard: null,
  logs: [],
};

export const useGameStore = create<GameStore>((set) => ({
  ...initialState,

  setGameId: (id) => set({ gameId: id }),

  setGameState: (state) =>
    set({
      turnCount: state.turnCount,
      currentPhase: state.currentPhase,
      currentPlayer: state.currentPlayer,
      memoryGauge: state.memoryGauge,
      isGameOver: state.isGameOver,
      winner: state.winner,
      player1: state.player1,
      player2: state.player2,
      revealedCards: state.revealedCards,
      pendingSelection: state.pendingSelection,
      pendingAttack: state.pendingAttack,
    }),

  setActionMask: (mask) => set({ actionMask: mask }),
  selectAttacker: (slot) => set({ selectedAttacker: slot }),
  setHoveredCard: (cardId) => set({ hoveredCard: cardId }),
  appendLogs: (newLogs) =>
    set((s) => ({ logs: [...s.logs, ...newLogs] })),
  clearLogs: () => set({ logs: [] }),
  reset: () => set(initialState),
}));
