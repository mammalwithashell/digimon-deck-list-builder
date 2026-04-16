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

/** The digimoncard.io API returns multiple entries per card id — one for the
 *  base printing plus one per TCGPlayer SKU (alt arts, reprints, promos).
 *  The only signal distinguishing an alt-art SKU from a reprint is the
 *  parenthetical in `tcgplayer_name` (e.g. "Omnimon (Alternate Art - sasasi)").
 *  The CDN only serves a single alt-art image per card under the `_alt`
 *  suffix, so we collapse all of the "Alternate Art" SKUs down to one entry. */
function isAltArtEntry(raw: Record<string, unknown>): boolean {
  const tcgName = (raw.tcgplayer_name as string | null | undefined) ?? '';
  return /alternate art/i.test(tcgName);
}

/** Map raw API response to our DigimonCardData shape.
 *  The API returns `id` but our components use `cardnumber`. */
function mapApiCard(raw: Record<string, unknown>, isAltArt = false): DigimonCardData {
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
    isAltArt,
  };
}

/** Collapse multiple API entries sharing the same card id down to at most two
 *  entries: the base printing and (if present) one alternate-art variant.
 *  Without this, the grid would render the same image N times because every
 *  TCGPlayer SKU maps to the same CDN filename. */
function dedupeByCardId(raw: Record<string, unknown>[]): DigimonCardData[] {
  const baseByCardId = new Map<string, DigimonCardData>();
  const altByCardId = new Map<string, DigimonCardData>();
  const order: string[] = [];

  for (const entry of raw) {
    const cardId = (entry.id as string) ?? '';
    if (!cardId) continue;
    if (!baseByCardId.has(cardId) && !altByCardId.has(cardId)) {
      order.push(cardId);
    }
    if (isAltArtEntry(entry)) {
      if (!altByCardId.has(cardId)) {
        altByCardId.set(cardId, mapApiCard(entry, true));
      }
    } else if (!baseByCardId.has(cardId)) {
      baseByCardId.set(cardId, mapApiCard(entry, false));
    }
  }

  const out: DigimonCardData[] = [];
  for (const cardId of order) {
    const base = baseByCardId.get(cardId);
    const alt = altByCardId.get(cardId);
    if (base) out.push(base);
    // If the card only appears as alt-art SKUs (no plain entry), surface the
    // alt-art entry as the base so the card still shows up at all.
    else if (alt) {
      out.push({ ...alt, isAltArt: false });
      continue;
    }
    if (alt && base) out.push(alt);
  }
  return out;
}

export async function searchCards(params: SearchParams): Promise<DigimonCardData[]> {
  const query: Record<string, string> = { series: 'Digimon Card Game' };
  for (const [key, value] of Object.entries(params)) {
    if (value) query[key] = value;
  }
  const { data } = await axios.get<Record<string, unknown>[]>(DIGIMON_API, { params: query });
  if (!Array.isArray(data)) return [];
  return dedupeByCardId(data);
}

export async function getCardById(cardId: string): Promise<DigimonCardData | null> {
  const results = await searchCards({ card: cardId });
  return results[0] ?? null;
}
