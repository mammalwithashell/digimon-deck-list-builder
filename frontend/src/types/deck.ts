import type { DigimonCardData } from './cards';

export interface DeckEntry {
  cardId: string;
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
