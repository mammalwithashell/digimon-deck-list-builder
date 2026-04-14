const CDN_BASE = 'https://images.digimoncard.io/images/cards';

export function getCardImageUrl(cardId: string, altArt = false): string {
  const suffix = altArt ? '_alt' : '';
  return `${CDN_BASE}/${cardId}${suffix}.webp`;
}
