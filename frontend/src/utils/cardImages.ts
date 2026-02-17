const CDN_BASE = 'https://images.digimoncard.io/images/cards';

export function getCardImageUrl(cardId: string): string {
  return `${CDN_BASE}/${cardId}.webp`;
}
