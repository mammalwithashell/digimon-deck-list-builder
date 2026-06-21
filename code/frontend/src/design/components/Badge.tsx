import type { HTMLAttributes, ReactNode } from 'react';

export type BadgeTone = 'default' | 'player' | 'opp' | 'good' | 'warn' | 'danger';

export interface BadgeProps extends HTMLAttributes<HTMLSpanElement> {
  tone?: BadgeTone;
  children?: ReactNode;
}

/** Small status pill — neutral or toned (player/opp/good/warn/danger). */
export function Badge({ tone = 'default', className = '', children, ...rest }: BadgeProps) {
  const toneClass = tone === 'default' ? '' : ` ds-badge--${tone}`;
  return (
    <span className={`ds-badge${toneClass} ${className}`.trim()} {...rest}>
      {children}
    </span>
  );
}
