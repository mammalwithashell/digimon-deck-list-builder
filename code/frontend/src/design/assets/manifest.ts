/*
 * Vendored visual-asset manifest (add-desktop-design-system).
 *
 * Maps a stable logical id -> a Vite-resolved asset URL + provenance. Card
 * sleeves are from WE-Kaito's digimon-tcg-simulator; Digimon pixel sprites are
 * from Project Drasil. Slot components (CardSleeve / DigimonSprite) resolve
 * assets through here and fall back procedurally on an unknown id.
 *
 * This is a representative starter pack (9 sleeves of 97 upstream, 1 sprite) —
 * bulk-add more by dropping files under assets/{sleeves,sprites}/ and adding an
 * entry here. Assets are theme-stable (never recolored by the active theme).
 */
import sleeveBt10_045 from './sleeves/main/BT10_045_KOKUWAMON.png';
import sleeveBt10_048 from './sleeves/main/BT10_048_SUNFLOWMON.png';
import sleeveBt10_059 from './sleeves/main/BT10_059_SPADAMON.png';
import sleeveBt11_024 from './sleeves/main/BT11_024_PENGUINMON.png';
import sleeveBt11_040 from './sleeves/main/BT11_040_SUKAMON.png';
import sleeveBt11_052 from './sleeves/main/BT11_052_TYRANNOMON.png';
import sleeveBt11_056 from './sleeves/main/BT11_056_JIJIMON_ARTIST_SHOWDOWN.png';
import sleeveBt11_086 from './sleeves/main/BT11_086_MERVAMON_7D6.png';
import sleeveSt19_03 from './sleeves/main/ST19_03_SHOEMON.png';
import spriteVeemon from './sprites/V-mon.png';

export type AssetSource = 'we-kaito' | 'project-drasil' | 'builtin';

export interface AssetEntry {
  id: string;
  url: string;
  name: string;
  source: AssetSource;
}

const WE_KAITO_CREDIT = 'WE-Kaito / digimon-tcg-simulator';
const DRASIL_CREDIT = 'Project Drasil';

export const SLEEVES: Record<string, AssetEntry> = {
  'BT10-045': { id: 'BT10-045', url: sleeveBt10_045, name: 'Kokuwamon', source: 'we-kaito' },
  'BT10-048': { id: 'BT10-048', url: sleeveBt10_048, name: 'Sunflowmon', source: 'we-kaito' },
  'BT10-059': { id: 'BT10-059', url: sleeveBt10_059, name: 'Spadamon', source: 'we-kaito' },
  'BT11-024': { id: 'BT11-024', url: sleeveBt11_024, name: 'Penguinmon', source: 'we-kaito' },
  'BT11-040': { id: 'BT11-040', url: sleeveBt11_040, name: 'Sukamon', source: 'we-kaito' },
  'BT11-052': { id: 'BT11-052', url: sleeveBt11_052, name: 'Tyrannomon', source: 'we-kaito' },
  'BT11-056': { id: 'BT11-056', url: sleeveBt11_056, name: 'Jijimon (Artist Showdown)', source: 'we-kaito' },
  'BT11-086': { id: 'BT11-086', url: sleeveBt11_086, name: 'Mervamon', source: 'we-kaito' },
  'ST19-03': { id: 'ST19-03', url: sleeveSt19_03, name: 'Shoemon', source: 'we-kaito' },
};

export const SPRITES: Record<string, AssetEntry> = {
  veemon: { id: 'veemon', url: spriteVeemon, name: 'V-mon', source: 'project-drasil' },
};

export function getSleeve(id: string | undefined): AssetEntry | undefined {
  return id ? SLEEVES[id] : undefined;
}

export function getSprite(id: string | undefined): AssetEntry | undefined {
  return id ? SPRITES[id] : undefined;
}

/** First available sleeve — handy default card back. */
export const DEFAULT_SLEEVE_ID = 'ST19-03';

export interface AssetCredit {
  source: AssetSource;
  label: string;
  kind: string;
  url: string;
}

export const ASSET_CREDITS: AssetCredit[] = [
  { source: 'we-kaito', label: WE_KAITO_CREDIT, kind: 'Card sleeves', url: 'https://github.com/WE-Kaito/digimon-tcg-simulator' },
  { source: 'project-drasil', label: DRASIL_CREDIT, kind: 'Digimon pixel sprites', url: 'https://project-drasil.online' },
];
