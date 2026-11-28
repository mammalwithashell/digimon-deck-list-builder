import { describe, expect, it } from 'vitest';
import { getCardImageUrl, getTokenImageUrl } from './cardImages';

describe('getCardImageUrl & official Bandai token art', () => {
  it('resolves all registered Rust engine token IDs to local official token art', () => {
    expect(getTokenImageUrl('TOKEN_PAISHU')).toBe('/cards/tokens/TOKEN-Paishu.webp');
    expect(getTokenImageUrl('TOKEN_KOTENKEN')).toBe('/cards/tokens/TOKEN-Kotenken.webp');
    expect(getTokenImageUrl('TOKEN_PETRIFICATION')).toBe('/cards/tokens/TOKEN-Petrification-Token.webp');
    expect(getTokenImageUrl('TOKEN_FAMILIAR')).toBe('/cards/tokens/TOKEN-Familiar-Token.webp');
    expect(getTokenImageUrl('TOKEN_ATHO_RENE_POR')).toBe('/cards/tokens/TOKEN-AthoRenePor-Token.webp');
    expect(getTokenImageUrl('TOKEN_HINUKAMUY')).toBe('/cards/tokens/TOKEN-Hinukamuy-Token.webp');
    expect(getTokenImageUrl('TOKEN_DIABOROMON')).toBe('/cards/tokens/TOKEN-Diaboromon-Token.webp');
    expect(getCardImageUrl('TOKEN_PAISHU')).toBe('/cards/tokens/TOKEN-Paishu.webp');
  });

  it('falls back to DigimonCard CDN for regular cards', () => {
    expect(getCardImageUrl('AD1-004')).toBe('https://images.digimoncard.io/images/cards/AD1-004.webp');
  });
});
