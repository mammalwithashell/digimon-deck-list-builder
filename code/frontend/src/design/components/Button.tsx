import type { ButtonHTMLAttributes, ReactNode } from 'react';

export type ButtonVariant = 'default' | 'primary' | 'accent' | 'ghost' | 'danger';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  children?: ReactNode;
}

/** Action button. Dark = angular bordered chip; light = beveled Win95 button. */
export function Button({
  variant = 'default',
  className = '',
  type = 'button',
  children,
  ...rest
}: ButtonProps) {
  const variantClass = variant === 'default' ? '' : ` ds-btn--${variant}`;
  return (
    <button type={type} className={`ds-btn${variantClass} ${className}`.trim()} {...rest}>
      {children}
    </button>
  );
}
