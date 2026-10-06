// Pure helpers for the lab-game deck stager (LabSetupPage).
//
// A staged deck is TOP-FIRST: index 0 is the first card drawn. Lab games
// skip the mulligan, so the main deck deals deterministically:
//   [0..5)  opening hand
//   [5..10) security — index 9 lands on top and is checked first
//   [10..)  draws, in order (the first player skips its turn-1 draw)
// The egg deck is likewise top-first: index 0 hatches first.

export const HAND_SIZE = 5;
export const SECURITY_SIZE = 5;

export interface StagedCard {
  /** Stable key for React lists / drag-and-drop (decks repeat card ids). */
  uid: string;
  cardId: string;
}

export interface SeatStaging {
  deckId: string | null;
  deckName: string;
  main: StagedCard[];
  eggs: StagedCard[];
}

export type SlotZone = 'hand' | 'security' | 'draw';

export interface SlotLabel {
  zone: SlotZone;
  /** Short label, e.g. "HAND", "SEC 1" (first checked), "DRAW 3". */
  text: string;
}

export function mainSlotLabel(index: number): SlotLabel {
  if (index < HAND_SIZE) return { zone: 'hand', text: 'HAND' };
  if (index < HAND_SIZE + SECURITY_SIZE) {
    // Index 9 is the top of the security stack, checked 1st.
    return { zone: 'security', text: `SEC ${HAND_SIZE + SECURITY_SIZE - index}` };
  }
  return { zone: 'draw', text: `DRAW ${index - HAND_SIZE - SECURITY_SIZE + 1}` };
}

let uidCounter = 0;

export function toStaged(cardIds: string[]): StagedCard[] {
  return cardIds.map((cardId) => ({ uid: `c${uidCounter++}`, cardId }));
}

/** Move one entry from `from` to `to` (both indices into the original list). */
export function moveCard<T>(list: T[], from: number, to: number): T[] {
  if (from === to || from < 0 || from >= list.length) return list;
  const clamped = Math.max(0, Math.min(list.length - 1, to));
  const next = list.slice();
  const moved = next.splice(from, 1);
  next.splice(clamped, 0, ...moved);
  return next;
}

/** Fisher–Yates shuffle (non-mutating). `random` is injectable for tests. */
export function shuffled<T>(list: T[], random: () => number = Math.random): T[] {
  const next = list.slice();
  for (let i = next.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    const tmp = next[i] as T;
    next[i] = next[j] as T;
    next[j] = tmp;
  }
  return next;
}

/** The single top-first list `rust_create_lab_game` expects. Eggs and main
 *  cards are routed by kind engine-side, keeping relative order. */
export function toEngineDeck(seat: SeatStaging): string[] {
  return [...seat.eggs.map((c) => c.cardId), ...seat.main.map((c) => c.cardId)];
}

/** Why a staged seat can't start, or null if it can. The engine deals 5 hand
 *  + 5 security up front, and an empty deck on a draw loses the game. */
export function seatProblem(seat: SeatStaging): string | null {
  if (!seat.deckId) return 'Pick a deck';
  const needed = HAND_SIZE + SECURITY_SIZE;
  if (seat.main.length < needed) {
    return `Main deck needs at least ${needed} cards (has ${seat.main.length})`;
  }
  return null;
}
