import { create } from 'zustand';
import {
  DEFAULT_PRESET,
  RESOLUTION_PRESETS,
  type ResolutionPreset,
} from '@/utils/constants';

const PRESET_STORAGE_KEY = 'desktop.graphicsPreset';
const FULLSCREEN_STORAGE_KEY = 'desktop.fullscreen';
const BOT_SPEED_STORAGE_KEY = 'gameplay.botSpeed';

/** Bot action pacing (add-bot-action-pacing). Non-instant speeds make
 *  local bot games advance one agent action per request, with this
 *  inter-beat delay, so a human can perceive each action. `instant`
 *  keeps the legacy drain-the-whole-bot-turn-in-one-response behavior. */
export type BotSpeed = 'slow' | 'normal' | 'fast' | 'instant';

export const BOT_SPEED_DELAY_MS: Record<Exclude<BotSpeed, 'instant'>, number> = {
  slow: 1500,
  normal: 900,
  fast: 400,
};

const BOT_SPEEDS: BotSpeed[] = ['slow', 'normal', 'fast', 'instant'];
const DEFAULT_BOT_SPEED: BotSpeed = 'normal';

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

  setHoveredCard: (cardId: string | null) => void;
  openModal: (modal: string) => void;
  closeModal: () => void;
  toggleSidebar: () => void;

  setGraphicsPreset: (preset: ResolutionPreset) => void;
  setFullscreen: (value: boolean) => void;
  setCanvasScale: (value: number) => void;
  setBotSpeed: (value: BotSpeed) => void;
}

export const useUiStore = create<UiStore>((set) => ({
  hoveredCard: null,
  activeModal: null,
  sidebarOpen: false,
  graphicsPreset: loadPersistedPreset(),
  fullscreen: loadPersistedFullscreen(),
  canvasScale: 1,
  botSpeed: loadPersistedBotSpeed(),

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
}));

// Exported for tests and direct CanvasScaler reads pre-mount.
export const __uiStoreInternals = {
  PRESET_STORAGE_KEY,
  FULLSCREEN_STORAGE_KEY,
  BOT_SPEED_STORAGE_KEY,
  loadPersistedPreset,
  loadPersistedFullscreen,
  loadPersistedBotSpeed,
};
