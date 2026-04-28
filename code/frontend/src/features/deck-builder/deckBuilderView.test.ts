import { describe, expect, it } from 'vitest';
import type { DigimonCardData } from '@/types/cards';
import type { DeckEntry } from '@/types/deck';
import {
  builderCardColorClass,
  deckEntriesToSlotArrays,
  filterBuilderCards,
  getBuilderCounts,
  groupDeckEntriesForBuilder,
  slotArraysToDeckEntries,
  type BuilderCardFilters,
} from './deckBuilderView';

function card(overrides: Partial<DigimonCardData>): DigimonCardData {
  return {
    name: 'Card',
    type: 'Digimon',
    color: 'Blue',
    stage: '',
    digi_type: '',
    attribute: '',
    level: '3',
    play_cost: '',
    evolution_cost: '',
    cardrarity: 'C',
    artist: '',
    dp: '',
    cardnumber: 'BT1-001',
    maineffect: '',
    soureeffect: '',
    set_name: '',
    card_sets: [],
    image_url: '',
    ...overrides,
  };
}

function filters(overrides: Partial<BuilderCardFilters> = {}): BuilderCardFilters {
  return {
    search: '',
    colors: [],
    type: '',
    level: '',
    rarity: '',
    inheritedOnly: false,
    securityOnly: false,
    ...overrides,
  };
}

function entry(cardData: DigimonCardData, count: number, isAltArt = false): DeckEntry {
  return {
    cardId: cardData.cardnumber,
    isAltArt,
    count,
    cardData,
  };
}

describe('deck builder view helpers', () => {
  it('maps card colors to compact class suffixes', () => {
    expect(builderCardColorClass(card({ color: 'Red' }))).toBe('r');
    expect(builderCardColorClass(card({ color: 'Blue' }))).toBe('b');
    expect(builderCardColorClass(card({ color: 'Purple' }))).toBe('p');
    expect(builderCardColorClass(card({ color: 'White' }))).toBe('w');
    expect(builderCardColorClass(undefined)).toBe('k');
  });

  it('filters builder cards by search, color, type, level, rarity, inherited text, and security text', () => {
    const target = card({
      name: 'AeroVeedramon',
      color: 'Blue',
      type: 'Digimon',
      level: '4',
      cardrarity: 'R',
      cardnumber: 'BT11-028',
      maineffect: '[Security] Draw 1.',
      soureeffect: 'Your turn: this Digimon gets +1000 DP.',
      digi_type: 'Mythical Dragon',
      attribute: 'Vaccine',
      set_name: 'Dimensional Phase',
    });
    const missesSearch = card({ name: 'Agumon', color: 'Blue', level: '4', cardrarity: 'R' });
    const missesColor = card({ name: 'AeroVeedramon', color: 'Red', level: '4', cardrarity: 'R' });
    const missesType = card({ name: 'AeroVeedramon', color: 'Blue', type: 'Tamer', level: '4', cardrarity: 'R' });
    const missesLevel = card({ name: 'AeroVeedramon', color: 'Blue', level: '5', cardrarity: 'R' });
    const missesRarity = card({ name: 'AeroVeedramon', color: 'Blue', level: '4', cardrarity: 'U' });
    const missesInherited = card({
      name: 'AeroVeedramon',
      color: 'Blue',
      level: '4',
      cardrarity: 'R',
      maineffect: '[Security] Draw 1.',
      soureeffect: '',
    });
    const missesSecurity = card({
      name: 'AeroVeedramon',
      color: 'Blue',
      level: '4',
      cardrarity: 'R',
      maineffect: '',
      soureeffect: 'Your turn: this Digimon gets +1000 DP.',
    });

    const result = filterBuilderCards(
      [target, missesSearch, missesColor, missesType, missesLevel, missesRarity, missesInherited, missesSecurity],
      filters({
        search: 'dimensional',
        colors: ['Blue'],
        type: 'Digimon',
        level: '4',
        rarity: 'R',
        inheritedOnly: true,
        securityOnly: true,
      }),
    );

    expect(result).toEqual([target]);
    expect(filterBuilderCards([target], filters({ search: 'vaccine' }))).toEqual([target]);
    expect(filterBuilderCards([target], filters({ search: 'BT11-028' }))).toEqual([target]);
  });

  it('computes main, egg, total, type, and level counts from deck entries', () => {
    const lv2 = card({ cardnumber: 'BT1-001', name: 'DemiVeemon', type: 'Digi-Egg', level: '2' });
    const lv3 = card({ cardnumber: 'BT1-002', name: 'Veemon', type: 'Digimon', level: '3' });
    const lv4 = card({ cardnumber: 'BT1-003', name: 'ExVeemon', type: 'Digimon', level: '4' });
    const lv5 = card({ cardnumber: 'BT1-004', name: 'Paildramon', type: 'Digimon', level: '5' });
    const lv6 = card({ cardnumber: 'BT1-005', name: 'Imperialdramon', type: 'Digimon', level: '6' });
    const lv7 = card({ cardnumber: 'BT1-006', name: 'Omnimon', type: 'Digimon', level: '7' });
    const tamer = card({ cardnumber: 'BT1-007', name: 'Davis Motomiya', type: 'Tamer', level: '' });
    const option = card({ cardnumber: 'BT1-008', name: 'Hammer Spark', type: 'Option', level: '' });

    const eggDeck = [entry(lv2, 4)];
    const mainDeck = [
      entry(lv3, 10),
      entry(lv4, 8),
      entry(lv5, 6),
      entry(lv6, 4),
      entry(lv7, 2),
      entry(tamer, 3),
      entry(option, 5),
    ];

    expect(getBuilderCounts(mainDeck, eggDeck)).toMatchObject({
      main: 38,
      egg: 4,
      side: 0,
      total: 42,
      eggCards: 4,
      digimon: 30,
      tamer: 3,
      option: 5,
      lv2: 4,
      lv3: 10,
      lv4: 8,
      lv5: 6,
      lv6: 4,
      lv7: 2,
    });
  });

  it('round-trips slot arrays with alt-art flags and groups eggs with builder labels', () => {
    const egg = card({ cardnumber: 'BT1-001', name: 'DemiVeemon', type: 'Digi-Egg', level: '2' });
    const rookie = card({ cardnumber: 'BT1-002', name: 'Veemon', type: 'Digimon', level: '3' });
    const option = card({ cardnumber: 'BT1-003', name: 'Hammer Spark', type: 'Option', level: '' });
    const cardMap = new Map([
      [egg.cardnumber, egg],
      [rookie.cardnumber, rookie],
      [option.cardnumber, option],
    ]);
    const ids = [egg.cardnumber, egg.cardnumber, rookie.cardnumber, rookie.cardnumber, option.cardnumber];
    const altArts = [false, true, false, true, false];

    const entries = slotArraysToDeckEntries(ids, altArts, cardMap);

    expect(entries).toEqual([
      { cardId: egg.cardnumber, isAltArt: false, count: 1, cardData: egg },
      { cardId: egg.cardnumber, isAltArt: true, count: 1, cardData: egg },
      { cardId: rookie.cardnumber, isAltArt: false, count: 1, cardData: rookie },
      { cardId: rookie.cardnumber, isAltArt: true, count: 1, cardData: rookie },
      { cardId: option.cardnumber, isAltArt: false, count: 1, cardData: option },
    ]);
    expect(deckEntriesToSlotArrays(entries)).toEqual({ ids, altArts });

    const sections = groupDeckEntriesForBuilder(entries);

    expect(sections.map((section) => section.label)).toEqual([
      'LV2 / DIGI-EGG',
      'LV3 / ROOKIE',
      'OPTION',
    ]);
    expect(sections[0]).toMatchObject({
      label: 'LV2 / DIGI-EGG',
      total: 2,
    });
  });
});
