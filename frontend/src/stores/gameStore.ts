import { create } from 'zustand';
import type {
  GameState,
  GameEvent,
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
  events: GameEvent[];
  activeEffectSlot: number | null;
  activeEffectPlayer: number | null;
  playerLabels: Record<number, string>;

  // Actions
  setGameId: (id: string | null) => void;
  setGameState: (state: GameState) => void;
  setActionMask: (mask: number[]) => void;
  setPlayerLabels: (labels: Record<number, string>) => void;
  selectAttacker: (slot: number | null) => void;
  setHoveredCard: (cardId: string | null) => void;
  appendLogs: (newLogs: string[]) => void;
  clearLogs: () => void;
  appendEvents: (newEvents: GameEvent[]) => void;
  clearEvents: () => void;
  setActiveEffect: (slot: number | null, player: number | null) => void;
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
  events: [],
  activeEffectSlot: null,
  activeEffectPlayer: null,
  playerLabels: { 1: 'Player 1', 2: 'Player 2' },
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
  setPlayerLabels: (labels) => set({ playerLabels: labels }),
  selectAttacker: (slot) => set({ selectedAttacker: slot }),
  setHoveredCard: (cardId) => set({ hoveredCard: cardId }),
  appendLogs: (newLogs) =>
    set((s) => ({ logs: [...s.logs, ...newLogs] })),
  clearLogs: () => set({ logs: [] }),
  appendEvents: (newEvents) =>
    set((s) => ({ events: [...s.events, ...newEvents] })),
  clearEvents: () => set({ events: [] }),
  setActiveEffect: (slot, player) =>
    set({ activeEffectSlot: slot, activeEffectPlayer: player }),
  reset: () => set(initialState),
}));
