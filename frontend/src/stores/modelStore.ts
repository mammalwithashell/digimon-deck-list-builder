import { create } from 'zustand';
import {
  deleteCachedModel,
  downloadModel,
  fetchModelManifest,
  type ModelEntry,
  type ModelManifest,
} from '@/api/modelsApi';

interface InFlight {
  [key: string]: 'downloading' | 'deleting' | undefined;
}

interface ModelStore {
  manifest: ModelManifest | null;
  loading: boolean;
  error: string | null;
  inFlight: InFlight;

  /** Key: `${slug}@${version}` */
  key: (slug: string, version: string) => string;

  refresh: (force?: boolean) => Promise<void>;
  download: (slug: string, version: string) => Promise<void>;
  remove: (slug: string, version: string) => Promise<void>;

  /** Entries sorted alphabetically by name, with deprecated pushed to the bottom. */
  listModels: () => ModelEntry[];
}

export const useModelStore = create<ModelStore>((set, get) => ({
  manifest: null,
  loading: false,
  error: null,
  inFlight: {},

  key: (slug, version) => `${slug}@${version}`,

  refresh: async (force = false) => {
    set({ loading: true, error: null });
    try {
      const manifest = await fetchModelManifest(force);
      set({ manifest, loading: false });
    } catch (exc) {
      set({
        loading: false,
        error: exc instanceof Error ? exc.message : String(exc),
      });
    }
  },

  download: async (slug, version) => {
    const k = get().key(slug, version);
    set((s) => ({ inFlight: { ...s.inFlight, [k]: 'downloading' } }));
    try {
      await downloadModel(slug, version);
      await get().refresh();
    } catch (exc) {
      set({ error: exc instanceof Error ? exc.message : String(exc) });
    } finally {
      set((s) => {
        const copy = { ...s.inFlight };
        delete copy[k];
        return { inFlight: copy };
      });
    }
  },

  remove: async (slug, version) => {
    const k = get().key(slug, version);
    set((s) => ({ inFlight: { ...s.inFlight, [k]: 'deleting' } }));
    try {
      await deleteCachedModel(slug, version);
      await get().refresh();
    } catch (exc) {
      set({ error: exc instanceof Error ? exc.message : String(exc) });
    } finally {
      set((s) => {
        const copy = { ...s.inFlight };
        delete copy[k];
        return { inFlight: copy };
      });
    }
  },

  listModels: () => {
    const models = get().manifest?.models ?? [];
    return [...models].sort((a, b) => {
      if (a.is_deprecated !== b.is_deprecated) {
        return a.is_deprecated ? 1 : -1;
      }
      return a.name.localeCompare(b.name);
    });
  },
}));
