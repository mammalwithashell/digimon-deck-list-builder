const CDN_BASE = 'https://images.digimoncard.io/images/cards';

/**
 * Official Bandai Digimon TCG token card artwork bundled locally under
 * `/cards/tokens/`. Covers both Rust engine synthetic IDs (`TOKEN_PAISHU`,
 * `TOKEN_KOTENKEN`, etc.) and hyphenated / lowercase variants.
 */
const TOKEN_ART_MAP: Record<string, string> = {
  // Currently registered in code/digimon-engine/src/token_registry.rs
  TOKEN_PETRIFICATION: 'TOKEN-Petrification-Token.webp',
  TOKEN_FAMILIAR: 'TOKEN-Familiar-Token.webp',
  TOKEN_ATHO_RENE_POR: 'TOKEN-AthoRenePor-Token.webp',
  TOKEN_HINUKAMUY: 'TOKEN-Hinukamuy-Token.webp',
  TOKEN_KOTENKEN: 'TOKEN-Kotenken.webp',
  TOKEN_PAISHU: 'TOKEN-Paishu.webp',
  TOKEN_DIABOROMON: 'TOKEN-Diaboromon-Token.webp',
  // Additional official Bandai Digimon TCG tokens
  TOKEN_AMON: 'TOKEN-Amon-of-Crimson-Flame.webp',
  TOKEN_AMON_OF_CRIMSON_FLAME: 'TOKEN-Amon-of-Crimson-Flame.webp',
  TOKEN_UMON: 'TOKEN-Umon-of-Blue-Thunder.webp',
  TOKEN_UMON_OF_BLUE_THUNDER: 'TOKEN-Umon-of-Blue-Thunder.webp',
  TOKEN_FUJITSUMON: 'TOKEN-Fujitsumon-Token.webp',
  TOKEN_GYUUKIMON: 'TOKEN-Gyuukimon-Token.webp',
  TOKEN_KOHAGURUMON: 'TOKEN-KoHagurumon-Token.webp',
  TOKEN_PIPE_FOX: 'TOKEN-Pipe-Fox.webp',
  TOKEN_RAPIDMON: 'TOKEN-Rapidmon-Token.webp',
  TOKEN_TAOMON: 'TOKEN-Taomon-Token.webp',
  TOKEN_UKA_NO_MITAMA: 'TOKEN-Uka-no-Mitama.webp',
  TOKEN_VOLEE_ZERDRUCKEN: 'TOKEN-Volee-Zerdrucken.webp',
  TOKEN_WARGROWLMON: 'TOKEN-WarGrowlmon-Token.webp',
};

export function getTokenImageUrl(cardId: string): string | null {
  if (!cardId) return null;
  const upper = cardId.trim().toUpperCase().replace(/[-\s]+/g, '_');
  const direct = TOKEN_ART_MAP[upper];
  if (direct) return `/cards/tokens/${direct}`;
  // Strip trailing "_TOKEN" if present (e.g. TOKEN_PAISHU_TOKEN -> TOKEN_PAISHU)
  const trimmed = upper.replace(/_TOKEN$/, '');
  if (TOKEN_ART_MAP[trimmed]) return `/cards/tokens/${TOKEN_ART_MAP[trimmed]}`;
  return null;
}

export function getCardImageUrl(cardId: string, altArt = false): string {
  const tokenUrl = getTokenImageUrl(cardId);
  if (tokenUrl) return tokenUrl;
  const suffix = altArt ? '_alt' : '';
  return `${CDN_BASE}/${cardId}${suffix}.webp`;
}
