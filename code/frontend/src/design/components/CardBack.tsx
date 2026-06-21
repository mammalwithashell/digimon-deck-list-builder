import type { CSSProperties } from 'react';
import { CardSleeve } from './CardSleeve';
import { DEFAULT_SLEEVE_ID } from '../assets/manifest';

export interface CardBackProps {
  sleeveId?: string;
  className?: string;
  style?: CSSProperties;
}

/** Face-down card — a card-aspect frame wrapping a sleeve. */
export function CardBack({ sleeveId = DEFAULT_SLEEVE_ID, className = '', style }: CardBackProps) {
  return (
    <div className={`ds-cardback ${className}`.trim()} style={style}>
      <CardSleeve sleeveId={sleeveId} alt="card back" />
    </div>
  );
}
