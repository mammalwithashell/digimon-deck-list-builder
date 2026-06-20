import { describe, expect, it } from 'vitest';
import { render } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { useThemeStore, type Theme } from '@/design/theme/themeStore';
import { LauncherActions } from './LauncherActions';

describe('LauncherActions window chrome', () => {
  it.each<Theme>(['dark', 'light'])(
    'renders titled Play and My-Decks windows in %s theme',
    (theme) => {
      useThemeStore.getState().setTheme(theme);
      render(
        <MemoryRouter>
          <LauncherActions />
        </MemoryRouter>,
      );
      expect(document.documentElement.dataset.theme).toBe(theme);
      // Design-system Window title bars carry the panel names; their accent
      // color (green in dark / navy in light) is supplied by the themed
      // `.ds-window__title` CSS keyed on `[data-theme]`.
      const titles = document.querySelectorAll('.ds-window__title-text');
      const labels = Array.from(titles).map((el) => el.textContent);
      expect(labels).toContain('PLAY');
      expect(labels).toContain('MY DECKS');
    },
  );
});
