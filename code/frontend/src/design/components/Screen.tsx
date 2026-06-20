import type { HTMLAttributes, ReactNode } from 'react';

export interface ScreenProps extends HTMLAttributes<HTMLDivElement> {
  children?: ReactNode;
}

/** Inset LCD/CRT surface (dark grid screen in both themes). */
export function Screen({ className = '', children, ...rest }: ScreenProps) {
  return (
    <div className={`ds-screen ${className}`.trim()} {...rest}>
      {children}
    </div>
  );
}
