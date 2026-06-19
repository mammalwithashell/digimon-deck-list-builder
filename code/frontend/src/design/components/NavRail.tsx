import type { ButtonHTMLAttributes, HTMLAttributes, ReactNode } from 'react';

export interface NavRailProps extends HTMLAttributes<HTMLElement> {
  children?: ReactNode;
}

/** Vertical navigation rail (launcher sidebar). */
export function NavRail({ className = '', children, ...rest }: NavRailProps) {
  return (
    <nav className={`ds-navrail ${className}`.trim()} {...rest}>
      {children}
    </nav>
  );
}

export interface NavRailItemProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  active?: boolean;
  children?: ReactNode;
}

export function NavRailItem({
  active = false,
  className = '',
  children,
  type = 'button',
  ...rest
}: NavRailItemProps) {
  return (
    <button
      type={type}
      className={`ds-navrail__item${active ? ' ds-navrail__item--active' : ''} ${className}`.trim()}
      aria-current={active ? 'page' : undefined}
      {...rest}
    >
      {children}
    </button>
  );
}
