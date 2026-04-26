import { create } from 'zustand';
import {
  type AgentItem,
  type TrainingJobItem,
  type GauntletItem,
  type GauntletDetailItem,
  type ArchetypeInfo,
  type StoreInfo,
  type SceneInfo,
  type DeckPoolItem,
  type DeckPoolDetail,
  getAgents,
  getJobs,
  getGauntlets,
  getGauntletDetail,
  getDeckLibraryArchetypes,
  getMetaScopeStores,
  getMetaScopeScenes,
  getDeckPools,
  getDeckPoolDetail,
} from '@/api/trainingApi';

interface TrainingState {
  agents: AgentItem[];
  jobs: TrainingJobItem[];
  gauntlets: GauntletItem[];
  selectedGauntlet: GauntletDetailItem | null;
  deckLibraryArchetypes: ArchetypeInfo[];
  metaStores: StoreInfo[];
  metaScenes: SceneInfo[];
  deckPools: DeckPoolItem[];
  selectedDeckPool: DeckPoolDetail | null;
  isLoading: boolean;
  error: string | null;

  fetchAgents: (params?: { archetype?: string }) => Promise<void>;
  fetchJobs: (params?: { status?: string; agent_id?: string; gauntlet_id?: string }) => Promise<void>;
  fetchGauntlets: () => Promise<void>;
  fetchGauntletDetail: (id: string) => Promise<void>;
  fetchDeckLibrary: (params?: { scope?: 'global' | 'store' | 'scene'; store_ids?: string; scene_id?: number }) => Promise<void>;
  fetchMetaStores: () => Promise<void>;
  fetchMetaScenes: () => Promise<void>;
  fetchDeckPools: (params?: { archetype?: string }) => Promise<void>;
  fetchDeckPoolDetail: (id: string) => Promise<void>;
  clearError: () => void;
}

export const useTrainingStore = create<TrainingState>((set) => ({
  agents: [],
  jobs: [],
  gauntlets: [],
  selectedGauntlet: null,
  deckLibraryArchetypes: [],
  metaStores: [],
  metaScenes: [],
  deckPools: [],
  selectedDeckPool: null,
  isLoading: false,
  error: null,

  fetchAgents: async (params) => {
    set({ isLoading: true, error: null });
    try {
      const agents = await getAgents(params);
      set({ agents, isLoading: false });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : 'Failed to fetch agents', isLoading: false });
    }
  },

  fetchJobs: async (params) => {
    set({ isLoading: true, error: null });
    try {
      const jobs = await getJobs(params);
      set({ jobs, isLoading: false });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : 'Failed to fetch jobs', isLoading: false });
    }
  },

  fetchGauntlets: async () => {
    set({ isLoading: true, error: null });
    try {
      const gauntlets = await getGauntlets();
      set({ gauntlets, isLoading: false });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : 'Failed to fetch gauntlets', isLoading: false });
    }
  },

  fetchGauntletDetail: async (id) => {
    set({ isLoading: true, error: null });
    try {
      const detail = await getGauntletDetail(id);
      set({ selectedGauntlet: detail, isLoading: false });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : 'Failed to fetch gauntlet detail', isLoading: false });
    }
  },

  fetchDeckLibrary: async (params) => {
    set({ isLoading: true, error: null });
    try {
      const archetypes = await getDeckLibraryArchetypes(params);
      set({ deckLibraryArchetypes: archetypes, isLoading: false });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : 'Failed to fetch deck library', isLoading: false });
    }
  },

  fetchMetaStores: async () => {
    try {
      const stores = await getMetaScopeStores();
      set({ metaStores: stores });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : 'Failed to fetch meta stores' });
    }
  },

  fetchMetaScenes: async () => {
    try {
      const scenes = await getMetaScopeScenes();
      set({ metaScenes: scenes });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : 'Failed to fetch meta scenes' });
    }
  },

  fetchDeckPools: async (params) => {
    set({ isLoading: true, error: null });
    try {
      const deckPools = await getDeckPools(params);
      set({ deckPools, isLoading: false });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : 'Failed to fetch deck pools', isLoading: false });
    }
  },

  fetchDeckPoolDetail: async (id) => {
    set({ isLoading: true, error: null });
    try {
      const detail = await getDeckPoolDetail(id);
      set({ selectedDeckPool: detail, isLoading: false });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : 'Failed to fetch deck pool detail', isLoading: false });
    }
  },

  clearError: () => set({ error: null }),
}));
