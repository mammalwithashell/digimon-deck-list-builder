import { render } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, it, expect } from 'vitest';
import { DebugBridgeNav } from './DebugBridgeNav';

describe('DebugBridgeNav', () => {
  it('renders nothing and does not throw outside a desktop build', () => {
    const { container } = render(
      <MemoryRouter>
        <DebugBridgeNav />
      </MemoryRouter>,
    );
    // Web/test build: IS_DESKTOP is false, so the effect early-returns and no
    // Tauri event API is imported. Component must render null.
    expect(container.firstChild).toBeNull();
  });
});
