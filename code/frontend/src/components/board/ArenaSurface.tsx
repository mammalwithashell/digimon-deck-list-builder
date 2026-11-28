/**
 * Tactile, theme-aware Hearthstone-style playmat surface.
 *
 * Renders layered CSS/SVG depth (curved opponent & player daises, subtle table
 * felt lighting, and a clean center memory seam) using only design-system
 * tokens so all 6 (dark|light x aegis|digi-os|adventure) presets harmonize.
 */
export function ArenaSurface() {
  return (
    <div className="hs-arena-surface" aria-hidden="true">
      <div className="hs-arena-surface__felt" />
      <svg
        className="hs-arena-surface__geometry"
        viewBox="0 0 1600 900"
        preserveAspectRatio="none"
      >
        {/* Top opponent dais arc */}
        <path
          d="M 380 0 Q 800 185 1220 0 Z"
          className="hs-arena-surface__dais hs-arena-surface__dais--opp"
        />
        <path
          d="M 380 0 Q 800 185 1220 0"
          fill="none"
          className="hs-arena-surface__dais-rim"
        />
        {/* Bottom player dais arc */}
        <path
          d="M 360 900 Q 800 710 1240 900 Z"
          className="hs-arena-surface__dais hs-arena-surface__dais--player"
        />
        <path
          d="M 360 900 Q 800 710 1240 900"
          fill="none"
          className="hs-arena-surface__dais-rim"
        />
        {/* Center battle horizon seam */}
        <line
          x1="80"
          y1="450"
          x2="1520"
          y2="450"
          className="hs-arena-surface__horizon"
        />
      </svg>
      <div className="hs-arena-surface__vignette" />
    </div>
  );
}
