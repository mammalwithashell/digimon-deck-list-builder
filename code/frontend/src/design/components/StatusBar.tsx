import type { HTMLAttributes, ReactNode } from 'react';

export interface StatusBarProps extends HTMLAttributes<HTMLDivElement> {
  children?: ReactNode;
}

/** Bottom status strip — mono, uppercase, theme-toned. */
export function StatusBar({ className = '', children, ...rest }: StatusBarProps) {
  return (
    <div className={`ds-statusbar ${className}`.trim()} {...rest}>
      {children}
    </div>
  );
}
