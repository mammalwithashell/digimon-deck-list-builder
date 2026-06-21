import type { DigimonCardData } from '@/types/cards';
import type { CardLegality, DeckEntry } from '@/types/deck';

export interface BuilderCardFilters {
  search: string;
  colors: string[];
  type: string;
  level: string;
  rarity: string;
  inheritedOnly: boolean;
  securityOnly: boolean;
  /** When true, hide cards illegal in the selected format (uses the
   *  engine-provided legality map; no rules are re-implemented here). */
  legalOnly: boolean;
}

export interface BuilderCounts {
  main: number;
  egg: number;
  side: number;
  total: number;
  eggCards: number;
  digimon: number;
  tamer: number;
  option: number;
  lv2: number;
  lv3: number;
  lv4: number;
  lv5: number;
  lv6: number;
  lv7: number;
}

export interface BuilderSection {
  label: string;
  expected: number;
  total: number;
  entries: DeckEntry[];
}

interface SectionDefinition {
  label: string;
  expected: number;
}

type SectionKey = 'lv2' | 'lv3' | 'lv4' | 'lv5' | 'lv6' | 'lv7' | 'tamer' | 'option' | 'other';

const SECTION_DEFINITIONS: Record<SectionKey, SectionDefinition> = {
  lv2: { label: 'LV2 / DIGI-EGG', expected: 5 },
  lv3: { label: 'LV3 / ROOKIE', expected: 0 },
  lv4: { label: 'LV4 / CHAMPION', expected: 0 },
  lv5: { label: 'LV5 / ULTIMATE', expected: 0 },
  lv6: { label: 'LV6 / MEGA', expected: 0 },
  lv7: { label: 'LV7 / ULTRA', expected: 0 },
  tamer: { label: 'TAMER', expected: 0 },
  option: { label: 'OPTION', expected: 0 },
  other: { label: 'OTHER', expected: 0 },
};

const SECTION_ORDER: readonly SectionKey[] = ['lv2', 'lv3', 'lv4', 'lv5', 'lv6', 'lv7', 'tamer', 'option', 'other'];
const DECK_ICON_COLOR_ORDER = ['red', 'blue', 'yellow', 'green', 'black', 'purple', 'white'];

function normalize(value: string | undefined): string {
  return (value ?? '').trim().toLowerCase();
}

function parseLevel(card: Pick<DigimonCardData, 'level'> | undefined): number {
  const level = Number.parseInt(card?.level ?? '', 10);
  return Number.isFinite(level) ? level : 0;
}

function parseNumber(value: string | number | undefined): number {
  const parsed = Number.parseInt(String(value ?? ''), 10);
  return Number.isFinite(parsed) ? parsed : 0;
}

function isDigiEgg(card: DigimonCardData | undefined): boolean {
  return normalize(card?.type) === 'digi-egg' || parseLevel(card) === 2;
}

function sectionKeyForEntry(entry: DeckEntry): SectionKey {
  const card = entry.cardData;
  const type = normalize(card?.type);

  if (!card) return 'other';
  if (isDigiEgg(card)) return 'lv2';
  if (type === 'tamer') return 'tamer';
  if (type === 'option') return 'option';

  const level = parseLevel(card);
  if (level === 3) return 'lv3';
  if (level === 4) return 'lv4';
  if (level === 5) return 'lv5';
  if (level === 6) return 'lv6';
  if (level >= 7) return 'lv7';
  return 'other';
}

function compareEntries(a: DeckEntry, b: DeckEntry): number {
  const levelDelta = parseLevel(a.cardData) - parseLevel(b.cardData);
  if (levelDelta !== 0) return levelDelta;

  const nameA = normalize(a.cardData?.name) || normalize(a.cardId);
  const nameB = normalize(b.cardData?.name) || normalize(b.cardId);
  const nameDelta = nameA.localeCompare(nameB);
  if (nameDelta !== 0) return nameDelta;

  const idDelta = a.cardId.localeCompare(b.cardId);
  if (idDelta !== 0) return idDelta;
  return Number(a.isAltArt) - Number(b.isAltArt);
}

function deckIconKindRank(card: DigimonCardData | undefined): number {
  const type = normalize(card?.type);
  if (type === 'digimon') return 0;
  if (type === 'tamer') return 1;
  if (type === 'option') return 2;
  if (type === 'digi-egg') return 3;
  return 4;
}

function deckIconColorRank(card: DigimonCardData | undefined): number {
  const index = DECK_ICON_COLOR_ORDER.indexOf(normalize(card?.color));
  return index === -1 ? DECK_ICON_COLOR_ORDER.length : index;
}

function compareDeckIconCandidates(a: DeckEntry, b: DeckEntry): number {
  const kindDelta = deckIconKindRank(a.cardData) - deckIconKindRank(b.cardData);
  if (kindDelta !== 0) return kindDelta;

  const levelDelta = parseLevel(b.cardData) - parseLevel(a.cardData);
  if (levelDelta !== 0) return levelDelta;

  const colorDelta = deckIconColorRank(a.cardData) - deckIconColorRank(b.cardData);
  if (colorDelta !== 0) return colorDelta;

  const costDelta = parseNumber(b.cardData?.play_cost) - parseNumber(a.cardData?.play_cost);
  if (costDelta !== 0) return costDelta;

  const dpDelta = parseNumber(b.cardData?.dp) - parseNumber(a.cardData?.dp);
  if (dpDelta !== 0) return dpDelta;

  const idDelta = a.cardId.localeCompare(b.cardId);
  if (idDelta !== 0) return idDelta;
  return Number(a.isAltArt) - Number(b.isAltArt);
}

export function builderCardColorClass(card: Pick<DigimonCardData, 'color'> | undefined): string {
  switch (normalize(card?.color)) {
    case 'red':
      return 'r';
    case 'blue':
      return 'b';
    case 'yellow':
      return 'y';
    case 'green':
      return 'g';
    case 'purple':
      return 'p';
    case 'white':
      return 'w';
    case 'black':
    default:
      return 'k';
  }
}

export function hasInheritedEffect(card: DigimonCardData): boolean {
  return card.soureeffect.trim().length > 0;
}

export function hasSecurityEffect(card: DigimonCardData): boolean {
  return /security/i.test(card.maineffect) || /security/i.test(card.soureeffect);
}

export function filterBuilderCards(
  cards: DigimonCardData[],
  filters: BuilderCardFilters,
  legality?: Record<string, CardLegality>,
): DigimonCardData[] {
  const search = normalize(filters.search);
  const colorSet = new Set(filters.colors.map(normalize));
  const type = normalize(filters.type);
  const level = normalize(filters.level);
  const rarity = normalize(filters.rarity);

  return cards.filter((card) => {
    if (filters.legalOnly && legality) {
      // Absent from the map → treat as legal (unknown/untested cards aren't
      // gated by format legality, matching the engine's "warn, don't reject").
      const leg = legality[card.cardnumber];
      if (leg && !leg.legal) return false;
    }

    if (search) {
      const searchable = [
        card.name,
        card.cardnumber,
        card.maineffect,
        card.soureeffect,
        card.digi_type,
        card.attribute,
        card.set_name,
      ]
        .map(normalize)
        .join(' ');
      if (!searchable.includes(search)) return false;
    }

    if (colorSet.size > 0) {
      // Intersection (AND): the card's colour identity must contain every
      // selected colour. Selecting one colour matches any card containing it;
      // selecting two matches only dual-colour cards with both; selecting
      // three matches nothing (no card has three colours).
      const cardColors = new Set<string>();
      const primary = normalize(card.color);
      const secondary = normalize(card.color2);
      if (primary) cardColors.add(primary);
      if (secondary) cardColors.add(secondary);
      for (const wanted of colorSet) {
        if (!cardColors.has(wanted)) return false;
      }
    }

    if (type && type !== 'all' && normalize(card.type) !== type) return false;
    if (level && level !== 'all' && normalize(card.level) !== level) return false;
    if (rarity && rarity !== 'all' && normalize(card.cardrarity) !== rarity) return false;
    if (filters.inheritedOnly && !hasInheritedEffect(card)) return false;
    if (filters.securityOnly && !hasSecurityEffect(card)) return false;
    return true;
  });
}

export function deckEntryCardCount(entries: DeckEntry[]): number {
  return entries.reduce((sum, entry) => sum + entry.count, 0);
}

export function getBuilderCounts(mainDeck: DeckEntry[], eggDeck: DeckEntry[]): BuilderCounts {
  const counts: BuilderCounts = {
    main: deckEntryCardCount(mainDeck),
    egg: deckEntryCardCount(eggDeck),
    side: 0,
    total: 0,
    eggCards: 0,
    digimon: 0,
    tamer: 0,
    option: 0,
    lv2: 0,
    lv3: 0,
    lv4: 0,
    lv5: 0,
    lv6: 0,
    lv7: 0,
  };
  counts.total = counts.main + counts.egg;

  for (const entry of [...eggDeck, ...mainDeck]) {
    const card = entry.cardData;
    const type = normalize(card?.type);
    const level = parseLevel(card);

    if (isDigiEgg(card)) counts.eggCards += entry.count;
    else if (type === 'digimon') counts.digimon += entry.count;
    else if (type === 'tamer') counts.tamer += entry.count;
    else if (type === 'option') counts.option += entry.count;

    if (level === 2) counts.lv2 += entry.count;
    else if (level === 3) counts.lv3 += entry.count;
    else if (level === 4) counts.lv4 += entry.count;
    else if (level === 5) counts.lv5 += entry.count;
    else if (level === 6) counts.lv6 += entry.count;
    else if (level >= 7) counts.lv7 += entry.count;
  }

  return counts;
}

export function selectDeckIconCardId(
  mainDeck: DeckEntry[],
  eggDeck: DeckEntry[],
  explicitCardId?: string | null,
): string | null {
  const presentEntries = [...mainDeck, ...eggDeck].filter((entry) => entry.count > 0);
  if (explicitCardId && presentEntries.some((entry) => entry.cardId === explicitCardId)) {
    return explicitCardId;
  }

  const mainCandidates = mainDeck.filter((entry) => entry.count > 0);
  if (mainCandidates.length > 0) {
    return [...mainCandidates].sort(compareDeckIconCandidates)[0]?.cardId ?? null;
  }

  const eggCandidates = eggDeck.filter((entry) => entry.count > 0);
  if (eggCandidates.length > 0) {
    return [...eggCandidates].sort((a, b) => a.cardId.localeCompare(b.cardId))[0]?.cardId ?? null;
  }

  return null;
}

export function slotArraysToDeckEntries(
  ids: string[],
  altArts: boolean[] | undefined,
  cardMap: Map<string, DigimonCardData>,
): DeckEntry[] {
  const grouped = new Map<string, DeckEntry>();

  ids.forEach((cardId, index) => {
    const isAltArt = Boolean(altArts?.[index]);
    const key = `${cardId}|${isAltArt ? '1' : '0'}`;
    const existing = grouped.get(key);
    if (existing) {
      existing.count += 1;
      return;
    }

    grouped.set(key, {
      cardId,
      isAltArt,
      count: 1,
      cardData: cardMap.get(cardId),
    });
  });

  return Array.from(grouped.values());
}

export function deckEntriesToSlotArrays(entries: DeckEntry[]): { ids: string[]; altArts: boolean[] } {
  const ids: string[] = [];
  const altArts: boolean[] = [];

  for (const entry of entries) {
    for (let i = 0; i < entry.count; i += 1) {
      ids.push(entry.cardId);
      altArts.push(entry.isAltArt);
    }
  }

  return { ids, altArts };
}

export function groupDeckEntriesForBuilder(entries: DeckEntry[]): BuilderSection[] {
  const groups = new Map<SectionKey, DeckEntry[]>();

  for (const entry of entries) {
    const key = sectionKeyForEntry(entry);
    const group = groups.get(key) ?? [];
    group.push(entry);
    groups.set(key, group);
  }

  return SECTION_ORDER.flatMap((key) => {
    const group = groups.get(key);
    if (!group || group.length === 0) return [];
    const definition = SECTION_DEFINITIONS[key];
    return [
      {
        label: definition.label,
        expected: definition.expected,
        total: deckEntryCardCount(group),
        entries: [...group].sort(compareEntries),
      },
    ];
  });
}
