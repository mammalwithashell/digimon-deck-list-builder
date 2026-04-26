export enum CardColor {
  Red = 0,
  Blue = 1,
  Yellow = 2,
  Green = 3,
  White = 4,
  Black = 5,
  Purple = 6,
}

export enum CardKind {
  Digimon = 0,
  Tamer = 1,
  Option = 2,
  DigiEgg = 3,
}

export enum Rarity {
  C = 0,
  U = 1,
  R = 2,
  SR = 3,
  SEC = 4,
  P = 5,
}

/** Card data as returned by digimoncard.io public API */
export interface DigimonCardData {
  name: string;
  type: string;
  color: string;
  stage: string;
  digi_type: string;
  attribute: string;
  level: string;
  play_cost: string;
  evolution_cost: string;
  cardrarity: string;
  artist: string;
  dp: string;
  cardnumber: string;
  maineffect: string;
  soureeffect: string; // Note: API uses this misspelling
  set_name: string;
  card_sets: string[];
  image_url: string;
  color2?: string;
  /** True when this entry represents an alternate-art printing
   *  (served from the CDN under the `_alt` suffix). */
  isAltArt?: boolean;
}

export interface CardFilters {
  name: string;
  color: string[];
  type: string[];
  level: string[];
  cost: string;
  dp: string;
  set: string;
  attribute: string;
  form: string;
  rarity: string;
}

export const DEFAULT_FILTERS: CardFilters = {
  name: '',
  color: [],
  type: [],
  level: [],
  cost: '',
  dp: '',
  set: '',
  attribute: '',
  form: '',
  rarity: '',
};
