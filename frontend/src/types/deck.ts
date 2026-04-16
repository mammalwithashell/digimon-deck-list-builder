import type { DigimonCardData } from './cards';

export interface DeckEntry {
  cardId: string;
  /** Whether this entry represents the alternate-art printing of the card.
   *  Entries with the same `cardId` but different `isAltArt` are kept as
   *  separate rows so the user can see both variants in their deck list,
   *  while the 4-per-card limit sums across both. */
  isAltArt: boolean;
  count: number;
  cardData?: DigimonCardData;
}

export interface DeckData {
  id?: string;
  name: string;
  mainDeck: DeckEntry[];
  eggDeck: DeckEntry[];
  gameMode: string;
}

export interface DeckSummary {
  id: string;
  name: string;
  game_mode: string;
  is_valid: boolean;
  is_public: boolean;
  card_count: number;
  created_at: string;
  updated_at: string;
}

export interface DeckValidationError {
  field: string;
  message: string;
}

export interface DeckValidationResult {
  valid: boolean;
  errors: DeckValidationError[];
  warnings: DeckValidationError[];
}
