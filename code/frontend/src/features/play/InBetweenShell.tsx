import type { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import './InBetweenShell.css';

interface Crumb {
  label: string;
  href?: string;
}

interface InBetweenShellProps {
  title: string;
  stepLabel: string;
  crumbs: Crumb[];
  children: ReactNode;
  rightSlot?: ReactNode;
}

export function InBetweenShell({
  title,
  stepLabel,
  crumbs,
  children,
  rightSlot,
}: InBetweenShellProps) {
  return (
    <div className="ib-flow-frame">
      <div className="ib-flow-titlebar">
        <div className="ib-flow-dots" aria-hidden="true">
          <span className="r" />
          <span className="y" />
          <span className="g" />
        </div>
        <div className="ib-flow-window-title">THE AMPHITHEATER BETWIXT - {title}</div>
        <span className="ib-flow-pill-live">CONNECTED</span>
      </div>
      <div className="ib-flow-body">
        <div className="ib-flow-topbar">
          <nav className="ib-flow-crumb" aria-label="Play flow">
            <span className="idx">{stepLabel}</span>
            {crumbs.map((crumb, index) => (
              <span key={`${crumb.label}-${index}`} className="crumb-part">
                <span className="sep">/</span>
                {crumb.href ? <Link to={crumb.href}>{crumb.label}</Link> : <span>{crumb.label}</span>}
              </span>
            ))}
          </nav>
          <div className="ib-flow-topbar-right">{rightSlot}</div>
        </div>
        {children}
      </div>
    </div>
  );
}
