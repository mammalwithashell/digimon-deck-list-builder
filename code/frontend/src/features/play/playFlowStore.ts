import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import type { QueueType } from '@/api/matchmaking';
import type { OpponentMode, PlayFormatId } from './formatCatalog';

interface QueueState {
  ticketId?: string | null;
  roomCode?: string | null;
  gameId?: string | null;
  seat?: 1 | 2 | null;
  seed?: string | null;
}

interface PlayFlowState {
  formatId: PlayFormatId;
  opponentMode: OpponentMode;
  queueType: QueueType;
  deckId: string | null;
  ticketId: string | null;
  roomCode: string | null;
  gameId: string | null;
  seed: string | null;
  /** This client's seat (1 = host, 2 = joiner) in a room or matched game. */
  seat: 1 | 2 | null;
  selectFormat: (formatId: PlayFormatId) => void;
  selectOpponentMode: (mode: OpponentMode) => void;
  selectQueueType: (queueType: QueueType) => void;
  selectDeck: (deckId: string | null) => void;
  setQueue: (state: QueueState) => void;
  clearLaunchState: () => void;
  reset: () => void;
}

const initial = {
  formatId: 'standard' as PlayFormatId,
  opponentMode: 'quick' as OpponentMode,
  queueType: 'casual' as QueueType,
  deckId: null,
  ticketId: null,
  roomCode: null,
  gameId: null,
  seed: null,
  seat: null,
};

export const usePlayFlowStore = create<PlayFlowState>()(
  persist(
    (set) => ({
      ...initial,
      selectFormat: (formatId) => set({ formatId }),
      selectOpponentMode: (opponentMode) => set({ opponentMode }),
      selectQueueType: (queueType) => set({ queueType }),
      selectDeck: (deckId) => set({ deckId }),
      setQueue: ({ ticketId, roomCode, gameId, seat, seed }) =>
        set((state) => ({
          ticketId: ticketId === undefined ? state.ticketId : ticketId,
          roomCode: roomCode === undefined ? state.roomCode : roomCode,
          gameId: gameId === undefined ? state.gameId : gameId,
          seat: seat === undefined ? state.seat : seat,
          seed: seed === undefined ? state.seed : seed,
        })),
      clearLaunchState: () =>
        set({ ticketId: null, roomCode: null, gameId: null, seat: null, seed: null }),
      reset: () => set(initial),
    }),
    {
      name: 'in-between-play-flow',
      storage: {
        getItem: (name) => {
          const value = sessionStorage.getItem(name);
          return value ? JSON.parse(value) : null;
        },
        setItem: (name, value) => sessionStorage.setItem(name, JSON.stringify(value)),
        removeItem: (name) => sessionStorage.removeItem(name),
      },
    },
  ),
);
