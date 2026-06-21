import { fnv1a } from './hash';

/** Roster of Digimon-franchise character names used as cosmetic opponent
 *  aliases. Pure flavor — the alias never affects gameplay. Grouped by theme
 *  for readability; the runtime treats it as one flat pool.
 *
 *  See openspec/changes/anonymize-opponent-names. */
export const BOT_NAMES: readonly string[] = [
  // AI / Digital-World god-computers (most on-theme for an AI opponent)
  'Yggdrasil',
  'King Drasil',
  'Kunlun',
  'Homeros',
  'Homeostasis',
  'ENIAC',
  'Leviathan',
  'Hypnos',
  'D-Reaper',
  'Huanglongmon',
  // Mentors / operators
  'Gennai',
  'Jijimon',
  'Babamon',
  'Shibumi',
  'Yamaki',
  'Wizardmon',
  'Tao',
  // Human Tamers / protagonists
  'Tai',
  'Matt',
  'Sora',
  'Izzy',
  'Davis',
  'Ken',
  'Takato',
  'Rika',
  'Henry',
  'Jeri',
  'Ryo',
  'Takuya',
  'Koji',
  'J.P.',
  'Marcus',
  'Thomas',
  'Taiki',
  'Kiriha',
  'Nene',
  // Partner Digimon
  'Agumon',
  'Gabumon',
  'Patamon',
  'Gatomon',
  'Guilmon',
  'Terriermon',
  'Renamon',
  'Veemon',
  'Impmon',
  'BanchoLeomon',
  'Leomon',
  'Shoutmon',
  'Gaomon',
  // Villains (boss flavor)
  'Myotismon',
  'Piedmon',
  'Apocalymon',
  'Diaboromon',
  'Lucemon',
  'Beelzemon',
  'Millenniummon',
  'Bagramon',
  'Quartzmon',
] as const;

/** Deterministically map a seed (e.g. `${gameId}:${seat}`) to a roster alias.
 *  Same seed → same name, so an opponent's displayed name is stable for the
 *  whole game (no flicker on WebSocket state updates) and is identical on both
 *  clients and for spectators. Different games (different game ids) vary. */
export function pickAlias(seed: string): string {
  return BOT_NAMES[fnv1a(seed) % BOT_NAMES.length]!;
}

/** Player-label map for a live game from the local viewer's perspective.
 *  - Seated viewer (`yourPlayerId` 1 or 2): own seat stays "You", the opponent
 *    is aliased.
 *  - Spectator / replay (`yourPlayerId` null): both seats are aliased.
 *  Aliases are seeded by `gameId`, so they are stable across state updates and
 *  reloads and identical on every client. (anonymize-opponent-names) */
export function liveGameLabels(
  gameId: string,
  yourPlayerId: number | null | undefined,
): Record<number, string> {
  if (yourPlayerId == null) {
    return { 1: pickAlias(`${gameId}:1`), 2: pickAlias(`${gameId}:2`) };
  }
  const other = yourPlayerId === 1 ? 2 : 1;
  return {
    [yourPlayerId]: 'You',
    [other]: pickAlias(`${gameId}:${other}`),
  };
}
