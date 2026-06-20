import type { CSSProperties } from 'react';
import { getSleeve } from '../assets/manifest';

export interface CardSleeveProps {
  sleeveId?: string;
  className?: string;
  style?: CSSProperties;
  alt?: string;
}

/** Card back from the asset manifest; procedural hatched fallback on unknown id. */
export function CardSleeve({ sleeveId, className = '', style, alt }: CardSleeveProps) {
  const entry = getSleeve(sleeveId);
  if (!entry) {
    return (
      <div
        className={`ds-sleeve ds-sleeve--fallback ${className}`.trim()}
        style={style}
        role="img"
        aria-label={alt ?? 'card back'}
      />
    );
  }
  return (
    <img
      className={`ds-sleeve ${className}`.trim()}
      style={style}
      src={entry.url}
      alt={alt ?? `${entry.name} sleeve`}
      draggable={false}
    />
  );
}
