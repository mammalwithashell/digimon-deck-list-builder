/**
 * Desktop design system (add-desktop-design-system) — public surface.
 *
 * Token CSS + fonts are side-effect imports wired at the app root (main.tsx).
 * This barrel re-exports the theme runtime and the primitive component library.
 */
export * from './theme/themeStore';
export * from './theme/ThemeProvider';
export * from './components';
