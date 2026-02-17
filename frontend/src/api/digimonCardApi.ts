import axios from 'axios';
import type { DigimonCardData } from '@/types/cards';

const DIGIMON_API = 'https://digimoncard.io/index.php/api-public/search';

interface SearchParams {
  n?: string;       // name
  color?: string;
  type?: string;
  card?: string;    // exact card ID
  pack?: string;    // set name
  attribute?: string;
  level?: string;
  sort?: string;    // name, power, color
  series?: string;  // "Digimon Card Game"
}

export async function searchCards(params: SearchParams): Promise<DigimonCardData[]> {
  const query: Record<string, string> = { series: 'Digimon Card Game' };
  for (const [key, value] of Object.entries(params)) {
    if (value) query[key] = value;
  }
  const { data } = await axios.get<DigimonCardData[]>(DIGIMON_API, { params: query });
  return Array.isArray(data) ? data : [];
}

export async function getCardById(cardId: string): Promise<DigimonCardData | null> {
  const results = await searchCards({ card: cardId });
  return results[0] ?? null;
}
