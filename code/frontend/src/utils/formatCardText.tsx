import type { ReactNode } from 'react';

/**
 * Known [timing] phrases — rendered as gold-on-black pills, vs other
 * [bracketed] names which render as violet refs. Mirrors how timings vs
 * card-name references print on the physical card.
 */
export const EFFECT_TIMINGS = new Set([
  'when digivolving', 'on play', 'on deletion', 'when attacking', 'end of attack',
  'start of your main phase', 'end of your main phase', 'start of your turn',
  'end of your turn', "start of opponent's turn", "end of opponent's turn",
  'all turns', 'your turn', "opponent's turn", 'main', 'security', 'counter',
  'when destroyed', 'once per turn', 'start of main phase', 'end of all turns',
]);

/**
 * Render card effect text with Mono Body highlighting: <keywords> as sharp
 * orange chips, known [timing] phrases as gold-on-black pills, and other
 * [bracketed] names as violet refs — mirroring how they print on the card.
 *
 * Shared by the deck-builder analyzer (scopes the chip styles under
 * `.bld-preview-effect`) and the in-game card inspector (via `CardEffectText`,
 * which scopes them under `.card-effect-text`).
 */
export function formatEffect(text: string): ReactNode {
  return text.split(/(\[[^\]]+\]|[<＜][^>＞]+[>＞])/g).map((part, i) => {
    const keyword = part.match(/^[<＜](.+)[>＞]$/);
    if (keyword) return <span key={i} className="kw">{keyword[1] ?? ''}</span>;
    const match = part.match(/^\[(.+)\]$/);
    if (!match) return part;
    const inner = match[1] ?? '';
    const norm = inner.trim().toLowerCase().replace(/[‘’]/g, "'");
    return EFFECT_TIMINGS.has(norm) ? (
      <span key={i} className="tm">{inner}</span>
    ) : (
      <span key={i} className="rf">[{inner}]</span>
    );
  });
}

/**
 * Effect text wrapped in a `.card-effect-text` container (JetBrains Mono + the
 * gold/orange/violet chip styles, defined in index.css). Used by the in-game
 * card inspector so it presents card text the same way as the deck-builder
 * analyzer.
 */
export function CardEffectText({ text, className = '' }: { text: string; className?: string }) {
  return <div className={`card-effect-text ${className}`.trim()}>{formatEffect(text)}</div>;
}
