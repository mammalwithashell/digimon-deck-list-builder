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

export interface DeckFolder {
  id: string;
  owner_id: string;
  name: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

export interface DeckSummary {
  id: string;
  name: string;
  description: string;
  game_mode: string;
  is_valid: boolean;
  is_public: boolean;
  is_pinned: boolean;
  folder_id: string | null;
  card_count: number;
  main_count: number;
  egg_count: number;
  tags: string[];
  meta_tier?: string | null;
  meta_archetype?: string | null;
  colors?: string[];
  highest_level?: number | null;
  created_at: string;
  updated_at: string;
}

export interface DeckResponse {
  id: string;
  owner_id: string;
  folder_id?: string | null;
  name: string;
  description: string;
  game_mode: string;
  main_deck: string[];
  egg_deck: string[];
  main_deck_alt_arts?: boolean[];
  egg_deck_alt_arts?: boolean[];
  commander_id: string | null;
  is_valid: boolean;
  validation_errors: string[];
  is_public: boolean;
  is_pinned: boolean;
  tags: string[];
  meta_tier?: string | null;
  meta_archetype?: string | null;
  created_at: string;
  updated_at: string;
}

/** One deck format from the engine registry (`GET /decks/formats` /
 *  `rust_list_formats`). The frontend never hardcodes this list. */
export interface DeckFormat {
  id: string;
  name: string;
  description: string;
  deck_size: number;
  egg_max: number;
  /** "all" | "common_uncommon" | "eden_anomaly" */
  rarity_policy: string;
  singleton: boolean;
  default_max_copies: number;
  playable: boolean;
}

/** Per-card legality under a format (`GET /decks/card-legality` /
 *  `rust_card_legality`). */
export interface CardLegality {
  legal: boolean;
  max_copies: number;
  reason: string | null;
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
