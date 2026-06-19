import type { HTMLAttributes, ReactNode } from 'react';

export interface WindowProps extends HTMLAttributes<HTMLDivElement> {
  title: ReactNode;
  /** When provided, renders a close affordance (× / beveled close box). */
  onClose?: () => void;
  bodyClassName?: string;
  children?: ReactNode;
}

/**
 * Titled surface. Dark renders a terminal header (`title  ● ●`); light renders
 * a Windows-95 title bar (colored bar + close box). Same props either way.
 */
export function Window({
  title,
  onClose,
  className = '',
  bodyClassName = '',
  children,
  ...rest
}: WindowProps) {
  return (
    <div className={`ds-window ds-panel ${className}`.trim()} {...rest}>
      <div className="ds-window__title">
        <span className="ds-window__title-text">{title}</span>
        {onClose ? (
          <button
            type="button"
            className="ds-window__close"
            aria-label="Close"
            onClick={onClose}
          >
            ×
          </button>
        ) : (
          <span className="ds-window__leds" aria-hidden="true">
            <span className="ds-window__led ds-window__led--signal" />
            <span className="ds-window__led ds-window__led--accent" />
          </span>
        )}
      </div>
      <div className={`ds-window__body ${bodyClassName}`.trim()}>{children}</div>
    </div>
  );
}
