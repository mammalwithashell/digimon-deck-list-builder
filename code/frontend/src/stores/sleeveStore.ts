import { useSyncExternalStore } from 'react';

const STORAGE_KEY = 'digimon.sleeve';
const CUSTOM_IMAGE_KEY = 'digimon.sleeve.custom';
const EGG_STORAGE_KEY = 'digimon.eggSleeve';
const CUSTOM_EGG_IMAGE_KEY = 'digimon.eggSleeve.custom';
const CUSTOM_IMAGE_MAX_EDGE = 512;

export const CUSTOM_CARD_SLEEVE_ID = 'custom';
export const CUSTOM_EGG_SLEEVE_ID = 'custom-egg';
export const CLASSIC_SLEEVE_ID = 'classic';

export interface CardSleeve {
  id: string;
  label: string;
  collection: string;
  src?: string;
}

export const CARD_SLEEVES: readonly CardSleeve[] = [
  {
    id: 'digimon-standard',
    label: 'Digimon Card Game',
    collection: 'Standard Card Back',
    src: '/sleeves/digimon-standard.webp',
  },
  {
    id: CLASSIC_SLEEVE_ID,
    label: 'Classic DCG',
    collection: 'Original',
  },
  {
    id: 'omnimon',
    label: 'Omnimon',
    collection: 'Official Card Sleeves',
    src: '/sleeves/omnimon.webp',
  },
  {
    id: 'alphamon',
    label: 'Alphamon',
    collection: 'Official Card Sleeves',
    src: '/sleeves/alphamon.webp',
  },
  {
    id: 'imperialdramon-paladin',
    label: 'Imperialdramon: Paladin Mode',
    collection: 'Official Card Sleeves',
    src: '/sleeves/imperialdramon-paladin.webp',
  },
  {
    id: 'gold',
    label: 'Gold',
    collection: 'Official Card Sleeves',
    src: '/sleeves/gold.webp',
  },
  {
    id: 'silver',
    label: 'Silver',
    collection: 'Official Card Sleeves',
    src: '/sleeves/silver.webp',
  },
  {
    id: 'animation-series-25th',
    label: 'Digimon Animation Series 25th',
    collection: 'Official Sleeves',
    src: '/sleeves/animation-series-25th.webp',
  },
  {
    id: 'official-01-crowd',
    label: 'Digimon Crowd',
    collection: 'Official Card Sleeves 01',
    src: '/sleeves/official-01-crowd.webp',
  },
  {
    id: 'official-01-agumon',
    label: 'Agumon and Friends',
    collection: 'Official Card Sleeves 01',
    src: '/sleeves/official-01-agumon.webp',
  },
  {
    id: 'official-01-appmon',
    label: 'Appmon',
    collection: 'Official Card Sleeves 01',
    src: '/sleeves/official-01-appmon.webp',
  },
  {
    id: 'official-01-silver-hair',
    label: 'Silver Hair',
    collection: 'Official Card Sleeves 01',
    src: '/sleeves/official-01-silver-hair.webp',
  },
  {
    id: 'official-02-light-blue',
    label: 'Cyber Sleuth: Light Blue',
    collection: 'Official Card Sleeves 02',
    src: '/sleeves/official-02-light-blue.webp',
  },
  {
    id: 'official-02-crimson',
    label: 'Cyber Sleuth: Crimson',
    collection: 'Official Card Sleeves 02',
    src: '/sleeves/official-02-crimson.webp',
  },
  {
    id: 'official-02-navy',
    label: 'Cyber Sleuth: Navy',
    collection: 'Official Card Sleeves 02',
    src: '/sleeves/official-02-navy.webp',
  },
  {
    id: 'official-02-yellow',
    label: 'Cyber Sleuth: Yellow',
    collection: 'Official Card Sleeves 02',
    src: '/sleeves/official-02-yellow.webp',
  },
  {
    id: 'official-03-adventure',
    label: 'Time Stranger: Protagonists',
    collection: 'Official Card Sleeves 03',
    src: '/sleeves/official-03-time-stranger-protagonists.webp',
  },
  {
    id: 'official-03-chronicle',
    label: 'Time Stranger: Inori and Aegiomon',
    collection: 'Official Card Sleeves 03',
    src: '/sleeves/official-03-time-stranger-inori-aegiomon.webp',
  },
  {
    id: 'official-03-generations',
    label: 'Data Squad',
    collection: 'Official Card Sleeves 03',
    src: '/sleeves/official-03-data-squad.webp',
  },
  {
    id: 'official-03-ghost-game',
    label: 'Beatbreak',
    collection: 'Official Card Sleeves 03',
    src: '/sleeves/official-03-beatbreak.webp',
  },
  {
    id: 'official-04-blue-knights',
    label: 'Blue Knights',
    collection: 'Official Card Sleeves 04',
    src: '/sleeves/official-04-blue-knights.webp',
  },
  {
    id: 'official-04-crimson-knights',
    label: 'Crimson Knights',
    collection: 'Official Card Sleeves 04',
    src: '/sleeves/official-04-crimson-knights.webp',
  },
  {
    id: 'official-04-baby-digimon',
    label: 'Baby Digimon',
    collection: 'Official Card Sleeves 04',
    src: '/sleeves/official-04-baby-digimon.webp',
  },
  {
    id: 'official-04-fire-and-ice',
    label: 'Fire and Ice',
    collection: 'Official Card Sleeves 04',
    src: '/sleeves/official-04-fire-and-ice.webp',
  },
  {
    id: 'official-2025v1-dark-angel',
    label: 'Dark Angel',
    collection: 'Official Sleeves 2025 Ver.1.0',
    src: '/sleeves/official-2025v1-dark-angel.webp',
  },
  {
    id: 'official-2025v1-impmon',
    label: 'Impmon',
    collection: 'Official Sleeves 2025 Ver.1.0',
    src: '/sleeves/official-2025v1-impmon.webp',
  },
  {
    id: 'official-2025v1-blue-flame',
    label: 'Blue Flame',
    collection: 'Official Sleeves 2025 Ver.1.0',
    src: '/sleeves/official-2025v1-blue-flame.webp',
  },
  {
    id: 'official-2025v1-gold-emblem',
    label: 'Gold Emblem',
    collection: 'Official Sleeves 2025 Ver.1.0',
    src: '/sleeves/official-2025v1-gold-emblem.webp',
  },
  {
    id: 'official-2024v2-purple-gold',
    label: 'Purple and Gold',
    collection: 'Official Sleeves 2024 Ver.2.0',
    src: '/sleeves/official-2024v2-purple-gold.webp',
  },
  {
    id: 'official-2024v2-adventure-partners',
    label: 'Adventure Partners',
    collection: 'Official Sleeves 2024 Ver.2.0',
    src: '/sleeves/official-2024v2-adventure-partners.webp',
  },
  {
    id: 'official-2024v2-green',
    label: 'Green Line Art',
    collection: 'Official Sleeves 2024 Ver.2.0',
    src: '/sleeves/official-2024v2-green.webp',
  },
  {
    id: 'official-2024v2-gold',
    label: 'Gold Line Art',
    collection: 'Official Sleeves 2024 Ver.2.0',
    src: '/sleeves/official-2024v2-gold.webp',
  },
  {
    id: 'official-2024v1-agumon',
    label: 'Agumon Spotlight',
    collection: 'Official Sleeves 2024 Ver.1.0',
    src: '/sleeves/official-2024v1-agumon.webp',
  },
  {
    id: 'official-2024v1-throne',
    label: 'Throne',
    collection: 'Official Sleeves 2024 Ver.1.0',
    src: '/sleeves/official-2024v1-throne.webp',
  },
  {
    id: 'official-2024v1-neon-back',
    label: 'Neon Card Back',
    collection: 'Official Sleeves 2024 Ver.1.0',
    src: '/sleeves/official-2024v1-neon-back.webp',
  },
  {
    id: 'official-2024v1-white-back',
    label: 'White Card Back',
    collection: 'Official Sleeves 2024 Ver.1.0',
    src: '/sleeves/official-2024v1-white-back.webp',
  },
  {
    id: 'official-2023-tai-agumon',
    label: 'Tai and Agumon',
    collection: 'Official Sleeves 2023',
    src: '/sleeves/official-2023-tai-agumon.webp',
  },
  {
    id: 'official-2023-matt-gabumon',
    label: 'Matt and Gabumon',
    collection: 'Official Sleeves 2023',
    src: '/sleeves/official-2023-matt-gabumon.webp',
  },
  {
    id: 'official-2023-loogamon',
    label: 'Loogamon',
    collection: 'Official Sleeves 2023',
    src: '/sleeves/official-2023-loogamon.webp',
  },
  {
    id: 'official-2023-shattered-glass',
    label: 'Shattered Glass',
    collection: 'Official Sleeves 2023',
    src: '/sleeves/official-2023-shattered-glass.webp',
  },
  {
    id: 'official-2022v2-laptop',
    label: 'Laptop Tamer',
    collection: 'Official Sleeves 2022 Ver.2.0',
    src: '/sleeves/official-2022v2-laptop.webp',
  },
  {
    id: 'official-2022v2-blue-dragons',
    label: 'Blue Dragons',
    collection: 'Official Sleeves 2022 Ver.2.0',
    src: '/sleeves/official-2022v2-blue-dragons.webp',
  },
  {
    id: 'official-2022v2-red-hexagons',
    label: 'Red Hexagons',
    collection: 'Official Sleeves 2022 Ver.2.0',
    src: '/sleeves/official-2022v2-red-hexagons.webp',
  },
  {
    id: 'official-2022v2-flames',
    label: 'Flames',
    collection: 'Official Sleeves 2022 Ver.2.0',
    src: '/sleeves/official-2022v2-flames.webp',
  },
  {
    id: 'official-2022-snowfall',
    label: 'Snowfall',
    collection: 'Official Sleeves 2022',
    src: '/sleeves/official-2022-snowfall.webp',
  },
  {
    id: 'official-2022-red-black',
    label: 'Red and Black',
    collection: 'Official Sleeves 2022',
    src: '/sleeves/official-2022-red-black.webp',
  },
  {
    id: 'official-2022-shoutmon',
    label: 'Shoutmon',
    collection: 'Official Sleeves 2022',
    src: '/sleeves/official-2022-shoutmon.webp',
  },
  {
    id: 'official-2022-meadow',
    label: 'Meadow',
    collection: 'Official Sleeves 2022',
    src: '/sleeves/official-2022-meadow.webp',
  },
  {
    id: 'official-2021v2-terriermon-lopmon',
    label: 'Terriermon and Lopmon',
    collection: 'Official Sleeves 2021 Ver.2.0',
    src: '/sleeves/official-2021v2-terriermon-lopmon.webp',
  },
  {
    id: 'official-2021v2-red-purple',
    label: 'Red and Purple',
    collection: 'Official Sleeves 2021 Ver.2.0',
    src: '/sleeves/official-2021v2-red-purple.webp',
  },
  {
    id: 'official-2021v2-imperialdramon',
    label: 'Imperialdramon',
    collection: 'Official Sleeves 2021 Ver.2.0',
    src: '/sleeves/official-2021v2-imperialdramon.webp',
  },
  {
    id: 'official-2021v2-gold-line-art',
    label: 'Gold Line Art (2021)',
    collection: 'Official Sleeves 2021 Ver.2.0',
    src: '/sleeves/official-2021v2-gold-line-art.webp',
  },
  {
    id: 'official-2021-armored-dragons',
    label: 'Armored Dragons',
    collection: 'Official Sleeves 2021',
    src: '/sleeves/official-2021-armored-dragons.webp',
  },
  {
    id: 'official-2021-agumon-gabumon',
    label: 'Agumon and Gabumon',
    collection: 'Official Sleeves 2021',
    src: '/sleeves/official-2021-agumon-gabumon.webp',
  },
  {
    id: 'official-2021-city-battle',
    label: 'City Battle',
    collection: 'Official Sleeves 2021',
    src: '/sleeves/official-2021-city-battle.webp',
  },
  {
    id: 'official-2021-pulsemon',
    label: 'Pulsemon',
    collection: 'Official Sleeves 2021',
    src: '/sleeves/official-2021-pulsemon.webp',
  },
  {
    id: 'official-2020-omnimon',
    label: 'Omnimon (2020)',
    collection: 'Official Sleeves 2020',
    src: '/sleeves/official-2020-omnimon.webp',
  },
  {
    id: 'official-2020-agumon',
    label: 'Agumon Line Art',
    collection: 'Official Sleeves 2020',
    src: '/sleeves/official-2020-agumon.webp',
  },
  {
    id: 'official-2020-in-training',
    label: 'In-Training',
    collection: 'Official Sleeves 2020',
    src: '/sleeves/official-2020-in-training.webp',
  },
  {
    id: 'official-2020-blue-back',
    label: 'Blue Card Back',
    collection: 'Official Sleeves 2020',
    src: '/sleeves/official-2020-blue-back.webp',
  },
];

export const DEFAULT_CARD_SLEEVE = CARD_SLEEVES[0]!;

/** The white Digi-Egg back every egg deck uses by default. */
export const DEFAULT_EGG_SLEEVE: CardSleeve = {
  id: 'digimon-egg',
  label: 'Digi-Egg',
  collection: 'Standard Card Back',
  src: '/sleeves/digimon-egg.webp',
};

/** Egg sleeves reuse the main catalog after the standard Digi-Egg back. */
export const EGG_SLEEVES: readonly CardSleeve[] = [DEFAULT_EGG_SLEEVE, ...CARD_SLEEVES];

const listeners = new Set<() => void>();

function notify(): void {
  for (const listener of listeners) listener();
}

function readImage(key: string): string | undefined {
  if (typeof window === 'undefined') return undefined;
  try {
    return window.localStorage.getItem(key) ?? undefined;
  } catch {
    return undefined;
  }
}

let customSrc = readImage(CUSTOM_IMAGE_KEY);
let customEggSrc = readImage(CUSTOM_EGG_IMAGE_KEY);

export function getCustomCardSleeveSrc(): string | undefined {
  return customSrc;
}

export function getCustomEggSleeveSrc(): string | undefined {
  return customEggSrc;
}

function isCardSleeveId(id: string | null | undefined): id is string {
  if (!id) return false;
  if (id === CUSTOM_CARD_SLEEVE_ID) return Boolean(customSrc);
  return CARD_SLEEVES.some((sleeve) => sleeve.id === id);
}

function isEggSleeveId(id: string | null | undefined): id is string {
  if (!id) return false;
  if (id === CUSTOM_EGG_SLEEVE_ID) return Boolean(customEggSrc);
  return id === DEFAULT_EGG_SLEEVE.id || isCardSleeveId(id);
}

function readId(): string {
  if (typeof window === 'undefined') return DEFAULT_CARD_SLEEVE.id;
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    return isCardSleeveId(stored) ? stored : DEFAULT_CARD_SLEEVE.id;
  } catch {
    return DEFAULT_CARD_SLEEVE.id;
  }
}

function readEggId(): string {
  if (typeof window === 'undefined') return DEFAULT_EGG_SLEEVE.id;
  try {
    const stored = window.localStorage.getItem(EGG_STORAGE_KEY);
    return isEggSleeveId(stored) ? stored : DEFAULT_EGG_SLEEVE.id;
  } catch {
    return DEFAULT_EGG_SLEEVE.id;
  }
}

let currentId = readId();
let currentEggId = readEggId();
let deckSleeveId: string | undefined;
let deckEggSleeveId: string | undefined;

export function getCardSleeveId(): string {
  return currentId;
}

export function setCardSleeveId(id: string): void {
  if (!isCardSleeveId(id)) return;
  currentId = id;
  try {
    window.localStorage.setItem(STORAGE_KEY, id);
  } catch {
    // Cosmetic preference; blocked storage still applies for this session.
  }
  notify();
}

/** Saves the custom main card sleeve image on this device and selects it. */
export function setCustomCardSleeve(dataUrl: string): void {
  window.localStorage.setItem(CUSTOM_IMAGE_KEY, dataUrl);
  customSrc = dataUrl;
  setCardSleeveId(CUSTOM_CARD_SLEEVE_ID);
}

export function clearCustomCardSleeve(): void {
  try {
    window.localStorage.removeItem(CUSTOM_IMAGE_KEY);
  } catch {
    // Nothing to clean up when storage is blocked.
  }
  customSrc = undefined;
  if (currentEggId === CUSTOM_CARD_SLEEVE_ID) {
    setEggSleeveId(DEFAULT_EGG_SLEEVE.id);
  }
  if (currentId === CUSTOM_CARD_SLEEVE_ID) {
    setCardSleeveId(DEFAULT_CARD_SLEEVE.id);
  } else {
    notify();
  }
}

/**
 * The playing deck's own sleeve, which overrides the global choice while set.
 * Undefined or unavailable ids fall back to the global card sleeve.
 */
export function setDeckSleeveId(id: string | undefined): void {
  if (id === deckSleeveId) return;
  deckSleeveId = id;
  notify();
}

export function getEffectiveCardSleeveId(): string {
  return deckSleeveId && isCardSleeveId(deckSleeveId) ? deckSleeveId : currentId;
}

export function cardSleeveById(id: string): CardSleeve {
  if (id === CUSTOM_CARD_SLEEVE_ID && customSrc) {
    return {
      id,
      label: 'Custom Image',
      collection: 'This device',
      src: customSrc,
    };
  }
  return CARD_SLEEVES.find((sleeve) => sleeve.id === id) ?? DEFAULT_CARD_SLEEVE;
}

export function getEggSleeveId(): string {
  return currentEggId;
}

export function setEggSleeveId(id: string): void {
  if (!isEggSleeveId(id)) return;
  currentEggId = id;
  try {
    window.localStorage.setItem(EGG_STORAGE_KEY, id);
  } catch {
    // Cosmetic preference; blocked storage still applies for this session.
  }
  notify();
}

/** Saves the custom Digi-Egg sleeve image on this device and selects it. */
export function setCustomEggSleeve(dataUrl: string): void {
  window.localStorage.setItem(CUSTOM_EGG_IMAGE_KEY, dataUrl);
  customEggSrc = dataUrl;
  setEggSleeveId(CUSTOM_EGG_SLEEVE_ID);
}

export function clearCustomEggSleeve(): void {
  try {
    window.localStorage.removeItem(CUSTOM_EGG_IMAGE_KEY);
  } catch {
    // Nothing to clean up when storage is blocked.
  }
  customEggSrc = undefined;
  if (currentEggId === CUSTOM_EGG_SLEEVE_ID) {
    setEggSleeveId(DEFAULT_EGG_SLEEVE.id);
  } else {
    notify();
  }
}

export function setDeckEggSleeveId(id: string | undefined): void {
  if (id === deckEggSleeveId) return;
  deckEggSleeveId = id;
  notify();
}

export function getEffectiveEggSleeveId(): string {
  return deckEggSleeveId && isEggSleeveId(deckEggSleeveId) ? deckEggSleeveId : currentEggId;
}

export function eggSleeveById(id: string): CardSleeve {
  if (id === CUSTOM_EGG_SLEEVE_ID && customEggSrc) {
    return {
      id,
      label: 'Custom Egg Image',
      collection: 'This device',
      src: customEggSrc,
    };
  }
  if (id === CUSTOM_CARD_SLEEVE_ID && customSrc) {
    return {
      id,
      label: 'Custom Image',
      collection: 'Uploaded card sleeve',
      src: customSrc,
    };
  }
  return id === DEFAULT_EGG_SLEEVE.id || !isCardSleeveId(id)
    ? DEFAULT_EGG_SLEEVE
    : cardSleeveById(id);
}

export function resetCardSleeve(): void {
  setCardSleeveId(DEFAULT_CARD_SLEEVE.id);
}

export function resetEggSleeve(): void {
  setEggSleeveId(DEFAULT_EGG_SLEEVE.id);
}

export function subscribeCardSleeve(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** Reactive hook returning the effective Main Card sleeve (deck override or global). */
export function useCardSleeve(): CardSleeve {
  const id = useSyncExternalStore(
    subscribeCardSleeve,
    getEffectiveCardSleeveId,
    () => DEFAULT_CARD_SLEEVE.id,
  );
  useSyncExternalStore(subscribeCardSleeve, getCustomCardSleeveSrc, () => undefined);
  return cardSleeveById(id);
}

/** Reactive hook returning the effective Digi-Egg sleeve (deck override or global). */
export function useEggSleeve(): CardSleeve {
  const id = useSyncExternalStore(
    subscribeCardSleeve,
    getEffectiveEggSleeveId,
    () => DEFAULT_EGG_SLEEVE.id,
  );
  useSyncExternalStore(subscribeCardSleeve, getCustomCardSleeveSrc, () => undefined);
  useSyncExternalStore(subscribeCardSleeve, getCustomEggSleeveSrc, () => undefined);
  return eggSleeveById(id);
}

/** Scales an uploaded image file down to a compact data URL suitable for localStorage. */
export async function toStorableSleeveDataUrl(file: File): Promise<string> {
  if (typeof createImageBitmap === 'function') {
    const bitmap = await createImageBitmap(file);
    try {
      const scale = Math.min(
        1,
        CUSTOM_IMAGE_MAX_EDGE / Math.max(bitmap.width, bitmap.height),
      );
      const canvas = document.createElement('canvas');
      canvas.width = Math.max(1, Math.round(bitmap.width * scale));
      canvas.height = Math.max(1, Math.round(bitmap.height * scale));
      const context = canvas.getContext('2d');
      if (!context) throw new Error('Canvas 2D context unavailable');
      context.drawImage(bitmap, 0, 0, canvas.width, canvas.height);
      const webp = canvas.toDataURL('image/webp', 0.82);
      return webp.startsWith('data:image/webp')
        ? webp
        : canvas.toDataURL('image/jpeg', 0.82);
    } finally {
      bitmap.close();
    }
  }

  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error('Failed to read image file'));
    reader.onload = () => {
      if (typeof reader.result === 'string') {
        resolve(reader.result);
      } else {
        reject(new Error('Invalid image result'));
      }
    };
    reader.readAsDataURL(file);
  });
}

export const sleeveUploadHelper = {
  toStorableDataUrl: toStorableSleeveDataUrl,
};

export const __sleeveStoreInternals = {
  STORAGE_KEY,
  CUSTOM_IMAGE_KEY,
  EGG_STORAGE_KEY,
  CUSTOM_EGG_IMAGE_KEY,
};
