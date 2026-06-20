import type { HTMLAttributes, ReactNode } from 'react';

export interface BackdropProps extends HTMLAttributes<HTMLDivElement> {
  children?: ReactNode;
}

/** App base surface + per-theme atmosphere (dark: scanlines/CRT; light: beige dot-grid). */
export function Backdrop({ children, className = '', ...rest }: BackdropProps) {
  return (
    <div className={`ds-backdrop ${className}`.trim()} {...rest}>
      {children}
    </div>
  );
}
