import { useTheme } from '../theme/ThemeProvider';

export interface ThemeSwitchProps {
  className?: string;
}

/** Dual-theme toggle. Dark = CRT power-toggle; light = Win95 switch. */
export function ThemeSwitch({ className = '' }: ThemeSwitchProps) {
  const { theme, toggle } = useTheme();
  const label = theme === 'dark' ? 'Dark' : 'Light';
  return (
    <button
      type="button"
      className={`ds-themeswitch ${className}`.trim()}
      onClick={toggle}
      aria-label={`Switch theme (currently ${label})`}
      title={`Theme: ${label}`}
    >
      <span>{label}</span>
      <span className="ds-themeswitch__track" aria-hidden="true">
        <span className="ds-themeswitch__knob" />
      </span>
    </button>
  );
}
