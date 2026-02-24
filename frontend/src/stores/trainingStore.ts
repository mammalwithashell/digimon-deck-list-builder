import { create } from 'zustand';
import {
  type AgentItem,
  type TrainingJobItem,
  type GauntletItem,
  type GauntletDetailItem,
  type ArchetypeInfo,
  getAgents,
  getJobs,
  getGauntlets,
  getGauntletDetail,
  getDeckLibraryArchetypes,
} from '@/api/trainingApi';

interface TrainingState {
  agents: AgentItem[];
  jobs: TrainingJobItem[];
  gauntlets: GauntletItem[];
  selectedGauntlet: GauntletDetailItem | null;
  deckLibraryArchetypes: ArchetypeInfo[];
  isLoading: boolean;
  error: string | null;

  fetchAgents: (params?: { archetype?: string }) => Promise<void>;
  fetchJobs: (params?: { status?: string; agent_id?: string; gauntlet_id?: string }) => Promise<void>;
  fetchGauntlets: () => Promise<void>;
  fetchGauntletDetail: (id: string) => Promise<void>;
  fetchDeckLibrary: () => Promise<void>;
  clearError: () => void;
}

export const useTrainingStore = create<TrainingState>((set) => ({
  agents: [],
  jobs: [],
  gauntlets: [],
  selectedGauntlet: null,
  deckLibraryArchetypes: [],
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

  fetchDeckLibrary: async () => {
    set({ isLoading: true, error: null });
    try {
      const archetypes = await getDeckLibraryArchetypes();
      set({ deckLibraryArchetypes: archetypes, isLoading: false });
    } catch (e) {
      set({ error: e instanceof Error ? e.message : 'Failed to fetch deck library', isLoading: false });
    }
  },

  clearError: () => set({ error: null }),
}));
