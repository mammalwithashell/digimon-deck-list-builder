import type { CSSProperties, ReactNode } from 'react';

export interface CardTileProps {
  name?: string;
  /** Art slot — typically a <DigimonSprite/> or card image. */
  art?: ReactNode;
  className?: string;
  style?: CSSProperties;
}

/** Card-aspect face tile with an art slot and a name strip. */
export function CardTile({ name, art, className = '', style }: CardTileProps) {
  return (
    <div className={`ds-cardtile ${className}`.trim()} style={style}>
      <div className="ds-cardtile__art">{art}</div>
      {name ? <div className="ds-cardtile__name">{name}</div> : null}
    </div>
  );
}
