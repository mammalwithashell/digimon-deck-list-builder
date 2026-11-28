import { useTheme } from '../theme/ThemeProvider';
import { STYLE_PRESETS } from '../theme/themeStore';

export interface ThemeSwitchProps {
  className?: string;
}

/** Calm dual-polarity toggle (Dark / Light). */
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

/** 3-preset palette picker (Aegis Clean / Soft Digi-OS / Soft Adventure). */
export function StylePresetSwitch({ className = '' }: { className?: string }) {
  const { stylePreset, setStylePreset } = useTheme();
  return (
    <div
      className={`ds-style-picker ${className}`.trim()}
      role="group"
      aria-label="Visual style preset"
    >
      {STYLE_PRESETS.map((preset) => {
        const active = stylePreset === preset.id;
        return (
          <button
            key={preset.id}
            type="button"
            data-testid={`style-preset-${preset.id}`}
            aria-pressed={active}
            title={preset.description}
            onClick={() => setStylePreset(preset.id)}
            className={`ds-style-picker__btn${active ? ' is-active' : ''}`}
          >
            {preset.label}
          </button>
        );
      })}
    </div>
  );
}
