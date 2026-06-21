import { create } from 'zustand';
import {
  DEFAULT_PRESET,
  RESOLUTION_PRESETS,
  type ResolutionPreset,
} from '@/utils/constants';

const PRESET_STORAGE_KEY = 'desktop.graphicsPreset';
const FULLSCREEN_STORAGE_KEY = 'desktop.fullscreen';
const RAIL_COLLAPSED_STORAGE_KEY = 'desktop.railCollapsed';
const BOT_SPEED_STORAGE_KEY = 'gameplay.botSpeed';
const DECK_BUILDER_VIEW_STORAGE_KEY = 'deckBuilder.viewMode';
const MOTION_STORAGE_KEY = 'desktop.motion';
const LIVE_BG_STORAGE_KEY = 'desktop.liveBackground';

/** Deck builder card-pool layout emphasis. `browse` is the dense grid that
 *  emphasizes the card search; `inspect` emphasizes the selected card and its
 *  effect text (DCGO-style); `decklist` emphasizes the deck contents as a
 *  two-column list with the pool reduced to a compact add strip. The toggle
 *  only re-proportions the layout — it does not change which cards are shown
 *  or any filter state. */
export type DeckBuilderView = 'browse' | 'inspect' | 'decklist';

const DECK_BUILDER_VIEWS: DeckBuilderView[] = ['browse', 'inspect', 'decklist'];
const DEFAULT_DECK_BUILDER_VIEW: DeckBuilderView = 'browse';

function loadPersistedDeckBuilderView(): DeckBuilderView {
  if (typeof window === 'undefined') return DEFAULT_DECK_BUILDER_VIEW;
  try {
    const raw = window.localStorage.getItem(DECK_BUILDER_VIEW_STORAGE_KEY);
    return DECK_BUILDER_VIEWS.includes(raw as DeckBuilderView)
      ? (raw as DeckBuilderView)
      : DEFAULT_DECK_BUILDER_VIEW;
  } catch {
    return DEFAULT_DECK_BUILDER_VIEW;
  }
}

function persistDeckBuilderView(value: DeckBuilderView): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(DECK_BUILDER_VIEW_STORAGE_KEY, value);
  } catch {
    // Best-effort persistence.
  }
}

/** Bot action pacing (add-bot-action-pacing). Non-instant speeds make
 *  local bot games advance one agent action per request, with this
 *  inter-beat delay, so a human can perceive each action. `instant`
 *  keeps the legacy drain-the-whole-bot-turn-in-one-response behavior. */
export type BotSpeed = 'slow' | 'normal' | 'fast' | 'instant';

export const BOT_SPEED_DELAY_MS: Record<Exclude<BotSpeed, 'instant'>, number> = {
  slow: 3000,
  normal: 1500,
  fast: 700,
};

const BOT_SPEEDS: BotSpeed[] = ['slow', 'normal', 'fast', 'instant'];
const DEFAULT_BOT_SPEED: BotSpeed = 'slow';

function loadPersistedBotSpeed(): BotSpeed {
  if (typeof window === 'undefined') return DEFAULT_BOT_SPEED;
  try {
    const raw = window.localStorage.getItem(BOT_SPEED_STORAGE_KEY);
    return BOT_SPEEDS.includes(raw as BotSpeed)
      ? (raw as BotSpeed)
      : DEFAULT_BOT_SPEED;
  } catch {
    return DEFAULT_BOT_SPEED;
  }
}

function persistBotSpeed(value: BotSpeed): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(BOT_SPEED_STORAGE_KEY, value);
  } catch {
    // Best-effort persistence.
  }
}

/** How much animation the app renders (add-effects-and-motion-settings).
 *  `full` = all motion; `reduced` = functional one-shot feedback only (no
 *  ambient/looping/pointer effects); `off` = no non-essential animation.
 *  This is the single gate every live effect (cursor lighting, live
 *  atmosphere, the digivolve cut-in) reads from. It is projected onto a
 *  `data-motion` attribute on <html> (set pre-paint in index.html, re-asserted
 *  from the store on mount) so the bulk of gating is pure CSS. */
export type Motion = 'full' | 'reduced' | 'off';

const MOTIONS: Motion[] = ['full', 'reduced', 'off'];

/** First-run default: honor the OS reduced-motion setting, else `full`.
 *  Used only to DERIVE the default — once the user picks a level we keep their
 *  explicit choice. Deliberately NOT persisted on derivation, so the OS hint
 *  keeps driving the default across runs until an explicit choice is made. */
export function deriveDefaultMotion(): Motion {
  if (typeof window === 'undefined') return 'full';
  try {
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches
      ? 'reduced'
      : 'full';
  } catch {
    return 'full';
  }
}

function loadPersistedMotion(): Motion {
  if (typeof window === 'undefined') return deriveDefaultMotion();
  try {
    const raw = window.localStorage.getItem(MOTION_STORAGE_KEY);
    return MOTIONS.includes(raw as Motion)
      ? (raw as Motion)
      : deriveDefaultMotion();
  } catch {
    return deriveDefaultMotion();
  }
}

function persistMotion(value: Motion): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(MOTION_STORAGE_KEY, value);
  } catch {
    // Best-effort persistence.
  }
}

/** Project the motion level onto the document root so CSS keyed on
 *  `[data-motion]` resolves. Mirrors `applyThemeAttribute` for the theme. */
export function applyMotionAttribute(motion: Motion): void {
  if (typeof document === 'undefined') return;
  document.documentElement.dataset.motion = motion;
}

/** Cosmetic switch for the live (animated) background. Effective only at
 *  motion `full` — see `selectEffectiveLiveBackground`. Defaults on. */
function loadPersistedLiveBackground(): boolean {
  if (typeof window === 'undefined') return true;
  try {
    const raw = window.localStorage.getItem(LIVE_BG_STORAGE_KEY);
    return raw === null ? true : raw === 'true';
  } catch {
    return true;
  }
}

function persistLiveBackground(value: boolean): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(LIVE_BG_STORAGE_KEY, String(value));
  } catch {
    // Best-effort persistence.
  }
}

/** Read a persisted preset from localStorage, falling back to the
 *  default. Validates that the persisted dimensions match one of the
 *  known presets — if a stale or hand-edited value is present, drops
 *  back to default rather than letting an arbitrary size apply. */
function loadPersistedPreset(): ResolutionPreset {
  if (typeof window === 'undefined') return DEFAULT_PRESET;
  try {
    const raw = window.localStorage.getItem(PRESET_STORAGE_KEY);
    if (!raw) return DEFAULT_PRESET;
    const parsed = JSON.parse(raw) as Partial<ResolutionPreset>;
    if (
      typeof parsed?.width !== 'number' ||
      typeof parsed?.height !== 'number'
    ) {
      return DEFAULT_PRESET;
    }
    const matched = RESOLUTION_PRESETS.find(
      (p) => p.width === parsed.width && p.height === parsed.height,
    );
    return matched ?? DEFAULT_PRESET;
  } catch {
    return DEFAULT_PRESET;
  }
}

function loadPersistedFullscreen(): boolean {
  if (typeof window === 'undefined') return false;
  try {
    const raw = window.localStorage.getItem(FULLSCREEN_STORAGE_KEY);
    return raw === 'true';
  } catch {
    return false;
  }
}

function loadPersistedRailCollapsed(): boolean {
  if (typeof window === 'undefined') return false;
  try {
    return window.localStorage.getItem(RAIL_COLLAPSED_STORAGE_KEY) === 'true';
  } catch {
    return false;
  }
}

function persistRailCollapsed(value: boolean): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(RAIL_COLLAPSED_STORAGE_KEY, String(value));
  } catch {
    // Best-effort persistence.
  }
}

function persistPreset(preset: ResolutionPreset): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(PRESET_STORAGE_KEY, JSON.stringify(preset));
  } catch {
    // Best-effort persistence — quota exceeded / private mode etc.
  }
}

function persistFullscreen(value: boolean): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(FULLSCREEN_STORAGE_KEY, String(value));
  } catch {
    // Best-effort persistence.
  }
}

interface UiStore {
  hoveredCard: string | null;
  activeModal: string | null;
  sidebarOpen: boolean;

  /** Currently selected resolution preset. Hydrated from localStorage
   *  on store creation; written back on every mutation. The CanvasScaler
   *  applies this to the Tauri window. */
  graphicsPreset: ResolutionPreset;
  /** Whether the window should be in fullscreen mode. */
  fullscreen: boolean;
  /** Current scale factor applied by CanvasScaler. Read by callers that
   *  need to compensate for the canvas transform — most notably the
   *  game's DndContext, which gets pointer deltas in window pixels but
   *  renders its DragOverlay inside the scaled canvas. The drag
   *  modifier divides transforms by this value so the overlay tracks
   *  the cursor 1:1 instead of falling behind by `(1 - scale)`. Always
   *  1.0 in non-desktop builds. */
  canvasScale: number;

  /** Bot action pacing speed for local games vs an agent. Persisted;
   *  changes apply from the next agent beat without restarting. */
  botSpeed: BotSpeed;

  /** Whether the desktop `MenuShell` nav rail is collapsed to an icon-only
   *  strip. Persisted so the choice survives relaunch. */
  railCollapsed: boolean;

  /** Deck builder card-pool view emphasis. Persisted so the choice survives
   *  relaunch. */
  deckBuilderView: DeckBuilderView;

  /** How much animation the app renders. Persisted; projected onto the
   *  `data-motion` attribute. The single gate every live effect reads. */
  motion: Motion;

  /** Cosmetic toggle for the live animated background. Effective only when
   *  `motion === 'full'` (see `selectEffectiveLiveBackground`). Persisted. */
  liveBackground: boolean;

  setHoveredCard: (cardId: string | null) => void;
  openModal: (modal: string) => void;
  closeModal: () => void;
  toggleSidebar: () => void;

  setGraphicsPreset: (preset: ResolutionPreset) => void;
  setFullscreen: (value: boolean) => void;
  setCanvasScale: (value: number) => void;
  setBotSpeed: (value: BotSpeed) => void;
  setRailCollapsed: (value: boolean) => void;
  toggleRail: () => void;
  setDeckBuilderView: (value: DeckBuilderView) => void;
  toggleDeckBuilderView: () => void;
  setMotion: (value: Motion) => void;
  setLiveBackground: (value: boolean) => void;
}

export const useUiStore = create<UiStore>((set) => ({
  hoveredCard: null,
  activeModal: null,
  sidebarOpen: false,
  graphicsPreset: loadPersistedPreset(),
  fullscreen: loadPersistedFullscreen(),
  canvasScale: 1,
  botSpeed: loadPersistedBotSpeed(),
  railCollapsed: loadPersistedRailCollapsed(),
  deckBuilderView: loadPersistedDeckBuilderView(),
  motion: loadPersistedMotion(),
  liveBackground: loadPersistedLiveBackground(),

  setHoveredCard: (cardId) => set({ hoveredCard: cardId }),
  openModal: (modal) => set({ activeModal: modal }),
  closeModal: () => set({ activeModal: null }),
  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),

  setGraphicsPreset: (preset) => {
    persistPreset(preset);
    set({ graphicsPreset: preset });
  },
  setFullscreen: (value) => {
    persistFullscreen(value);
    set({ fullscreen: value });
  },
  setCanvasScale: (value) => set({ canvasScale: value }),
  setBotSpeed: (value) => {
    persistBotSpeed(value);
    set({ botSpeed: value });
  },
  setRailCollapsed: (value) => {
    persistRailCollapsed(value);
    set({ railCollapsed: value });
  },
  toggleRail: () =>
    set((s) => {
      const next = !s.railCollapsed;
      persistRailCollapsed(next);
      return { railCollapsed: next };
    }),
  setDeckBuilderView: (value) => {
    persistDeckBuilderView(value);
    set({ deckBuilderView: value });
  },
  toggleDeckBuilderView: () =>
    set((s) => {
      const index = DECK_BUILDER_VIEWS.indexOf(s.deckBuilderView);
      const next = DECK_BUILDER_VIEWS[(index + 1) % DECK_BUILDER_VIEWS.length]!;
      persistDeckBuilderView(next);
      return { deckBuilderView: next };
    }),
  setMotion: (value) => {
    persistMotion(value);
    applyMotionAttribute(value);
    set({ motion: value });
  },
  setLiveBackground: (value) => {
    persistLiveBackground(value);
    set({ liveBackground: value });
  },
}));

/** Effective live-background: animated atmosphere renders only when motion is
 *  `full` AND the cosmetic toggle is on. Downstream live-effect features
 *  (live atmosphere, board atmosphere) read this single resolved boolean. */
export const selectEffectiveLiveBackground = (s: UiStore): boolean =>
  s.motion === 'full' && s.liveBackground;

/** Hook accessors so other features read one source of truth for motion
 *  rather than re-deriving from the OS or the attribute. */
export const useMotion = (): Motion => useUiStore((s) => s.motion);
export const useEffectiveLiveBackground = (): boolean =>
  useUiStore(selectEffectiveLiveBackground);

// Exported for tests and direct CanvasScaler reads pre-mount.
export const __uiStoreInternals = {
  PRESET_STORAGE_KEY,
  FULLSCREEN_STORAGE_KEY,
  RAIL_COLLAPSED_STORAGE_KEY,
  BOT_SPEED_STORAGE_KEY,
  DECK_BUILDER_VIEW_STORAGE_KEY,
  MOTION_STORAGE_KEY,
  LIVE_BG_STORAGE_KEY,
  loadPersistedPreset,
  loadPersistedFullscreen,
  loadPersistedRailCollapsed,
  loadPersistedBotSpeed,
  loadPersistedDeckBuilderView,
  loadPersistedMotion,
  deriveDefaultMotion,
  loadPersistedLiveBackground,
};
