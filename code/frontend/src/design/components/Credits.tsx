import { ASSET_CREDITS } from '../assets/manifest';

export interface CreditsProps {
  className?: string;
}

/** In-app attribution for the bundled community art (sleeves + sprites). */
export function Credits({ className = '' }: CreditsProps) {
  return (
    <div
      className={className}
      style={{ fontFamily: 'var(--font-ui)', fontSize: 'var(--text-sm)', color: 'var(--ink-1)' }}
    >
      <div
        style={{
          fontFamily: 'var(--font-display)',
          fontSize: 'var(--text-lg)',
          color: 'var(--ink-0)',
          marginBottom: 8,
        }}
      >
        Credits
      </div>
      <p style={{ marginBottom: 12 }}>
        Bundled community art, used with thanks under fan-project terms. Digimon and
        all related marks are the property of their respective owners.
      </p>
      <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: 8 }}>
        {ASSET_CREDITS.map((c) => (
          <li key={c.source}>
            <span style={{ color: 'var(--ink-3)' }}>{c.kind}: </span>
            <a href={c.url} target="_blank" rel="noreferrer" style={{ color: 'var(--accent)' }}>
              {c.label}
            </a>
          </li>
        ))}
      </ul>
    </div>
  );
}
