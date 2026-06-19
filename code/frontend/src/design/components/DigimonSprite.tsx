import type { CSSProperties } from 'react';
import { getSprite } from '../assets/manifest';

export interface DigimonSpriteProps {
  spriteId?: string;
  size?: number;
  className?: string;
  style?: CSSProperties;
  alt?: string;
}

/** Pixel sprite from the asset manifest; procedural grid fallback on unknown id. */
export function DigimonSprite({ spriteId, size, className = '', style, alt }: DigimonSpriteProps) {
  const entry = getSprite(spriteId);
  const dim: CSSProperties = size ? { width: size, height: size } : {};
  if (!entry) {
    return (
      <div
        className={`ds-sprite ds-sprite--fallback ${className}`.trim()}
        style={{ ...dim, ...style }}
        role="img"
        aria-label={alt ?? 'sprite'}
      />
    );
  }
  return (
    <img
      className={`ds-sprite ${className}`.trim()}
      style={{ ...dim, ...style }}
      src={entry.url}
      alt={alt ?? entry.name}
      draggable={false}
    />
  );
}
