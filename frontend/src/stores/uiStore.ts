import { create } from 'zustand';

interface UiStore {
  hoveredCard: string | null;
  activeModal: string | null;
  sidebarOpen: boolean;

  setHoveredCard: (cardId: string | null) => void;
  openModal: (modal: string) => void;
  closeModal: () => void;
  toggleSidebar: () => void;
}

export const useUiStore = create<UiStore>((set) => ({
  hoveredCard: null,
  activeModal: null,
  sidebarOpen: false,

  setHoveredCard: (cardId) => set({ hoveredCard: cardId }),
  openModal: (modal) => set({ activeModal: modal }),
  closeModal: () => set({ activeModal: null }),
  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),
}));
