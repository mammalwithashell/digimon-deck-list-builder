import type { CSSProperties } from 'react';
import { CardSleeve } from './CardSleeve';

export type DeckColor = 'r' | 'b' | 'y' | 'g' | 'k' | 'p' | 'w';

/** Canonical deck-color identity gradients (theme-stable). */
const COLOR_GRADIENT: Record<DeckColor, string> = {
  r: 'linear-gradient(160deg, #3a1414, #15060a)',
  b: 'linear-gradient(160deg, #14243a, #060d1d)',
  y: 'linear-gradient(160deg, #3a2c14, #1d1308)',
  g: 'linear-gradient(160deg, #14382a, #061f10)',
  k: 'linear-gradient(160deg, #1a1a1f, #08080b)',
  p: 'linear-gradient(160deg, #2c1438, #16061f)',
  w: 'linear-gradient(160deg, #2c3038, #14171c)',
};

export interface DeckColorBadgeProps {
  color: DeckColor;
  /** Optional sleeve to show instead of the flat color gradient. */
  sleeveId?: string;
  className?: string;
  style?: CSSProperties;
}

/** Per-color deck identity tile — a sleeve when available, else the color gradient. */
export function DeckColorBadge({ color, sleeveId, className = '', style }: DeckColorBadgeProps) {
  return (
    <span
      className={`ds-deckcolor ${className}`.trim()}
      style={{ background: sleeveId ? undefined : COLOR_GRADIENT[color], ...style }}
    >
      {sleeveId ? <CardSleeve sleeveId={sleeveId} /> : null}
      <span className="ds-deckcolor__letter">{color.toUpperCase()}</span>
    </span>
  );
}
