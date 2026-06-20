import type { HTMLAttributes, ReactNode } from 'react';

export interface PanelProps extends HTMLAttributes<HTMLDivElement> {
  /** Use the raised surface tone (slightly lifted from the base panel). */
  raised?: boolean;
  children?: ReactNode;
}

/** Bordered surface: dark = glow border + clip-path corner cut; light = bevel. */
export function Panel({ raised = false, className = '', children, ...rest }: PanelProps) {
  return (
    <div
      className={`ds-panel${raised ? ' ds-panel--raised' : ''} ${className}`.trim()}
      {...rest}
    >
      {children}
    </div>
  );
}
