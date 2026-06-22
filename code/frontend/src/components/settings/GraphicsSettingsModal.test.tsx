import '@testing-library/jest-dom/vitest';
import { render, screen, act, fireEvent } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { GraphicsSettingsModal, GRAPHICS_MODAL_ID } from './GraphicsSettingsModal';
import { useUiStore } from '@/stores/uiStore';

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    setSize: vi.fn(),
    setFullscreen: vi.fn(),
  }),
  LogicalSize: class {
    constructor(
      public width: number,
      public height: number,
    ) {}
  },
}));

describe('GraphicsSettingsModal', () => {
  beforeEach(() => {
    useUiStore.setState({ activeModal: null });
  });

  it('renders nothing when the graphics modal is not active', () => {
    const { container } = render(<GraphicsSettingsModal />);
    expect(container.querySelector('.ds-modal')).toBeNull();
  });

  it('renders the panel when active and closes on backdrop click', () => {
    useUiStore.setState({ activeModal: GRAPHICS_MODAL_ID });
    render(<GraphicsSettingsModal />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByTestId('graphics-motion-full')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('graphics-modal-backdrop'));
    expect(useUiStore.getState().activeModal).toBeNull();
  });

  it('closes on Escape', () => {
    useUiStore.setState({ activeModal: GRAPHICS_MODAL_ID });
    render(<GraphicsSettingsModal />);
    act(() => {
      fireEvent.keyDown(document, { key: 'Escape' });
    });
    expect(useUiStore.getState().activeModal).toBeNull();
  });
});
