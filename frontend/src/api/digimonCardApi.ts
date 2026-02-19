import axios from 'axios';
import type { DigimonCardData } from '@/types/cards';

const DIGIMON_API = 'https://digimoncard.io/index.php/api-public/search';

interface SearchParams {
  n?: string;       // name
  color?: string;
  type?: string;
  card?: string;    // exact card ID or prefix (e.g. "BT20" for set search)
  pack?: string;    // set name
  attribute?: string;
  level?: string;
  sort?: string;    // name, power, color
  series?: string;  // "Digimon Card Game"
}

/** Map raw API response to our DigimonCardData shape.
 *  The API returns `id` but our components use `cardnumber`. */
function mapApiCard(raw: Record<string, unknown>): DigimonCardData {
  return {
    name: (raw.name as string) ?? '',
    type: (raw.type as string) ?? '',
    color: (raw.color as string) ?? '',
    stage: (raw.stage as string) ?? '',
    digi_type: (raw.digi_type as string) ?? '',
    attribute: (raw.attribute as string) ?? '',
    level: raw.level != null ? String(raw.level) : '',
    play_cost: raw.play_cost != null ? String(raw.play_cost) : '',
    evolution_cost: raw.evolution_cost != null ? String(raw.evolution_cost) : '',
    cardrarity: (raw.rarity as string) ?? '',
    artist: (raw.artist as string) ?? '',
    dp: raw.dp != null ? String(raw.dp) : '',
    cardnumber: (raw.id as string) ?? '',  // API uses 'id', we use 'cardnumber'
    maineffect: (raw.main_effect as string) ?? '',
    soureeffect: (raw.source_effect as string) ?? '',
    set_name: Array.isArray(raw.set_name) ? (raw.set_name as string[]).join(', ') : (raw.set_name as string) ?? '',
    card_sets: Array.isArray(raw.set_name) ? (raw.set_name as string[]) : [],
    image_url: '',  // Not provided by API; CDN URL constructed from cardnumber
    color2: (raw.color2 as string) ?? undefined,
  };
}

export async function searchCards(params: SearchParams): Promise<DigimonCardData[]> {
  const query: Record<string, string> = { series: 'Digimon Card Game' };
  for (const [key, value] of Object.entries(params)) {
    if (value) query[key] = value;
  }
  const { data } = await axios.get<Record<string, unknown>[]>(DIGIMON_API, { params: query });
  if (!Array.isArray(data)) return [];
  return data.map(mapApiCard);
}

export async function getCardById(cardId: string): Promise<DigimonCardData | null> {
  const results = await searchCards({ card: cardId });
  return results[0] ?? null;
}
