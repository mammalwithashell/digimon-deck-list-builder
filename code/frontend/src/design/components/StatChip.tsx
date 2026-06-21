import type { ReactNode } from 'react';

export interface StatChipProps {
  label?: string;
  value: ReactNode;
  tone?: 'default' | 'signal';
  className?: string;
}

/** A small labelled stat (level / type / special move) for the analyzer + board. */
export function StatChip({ label, value, tone = 'default', className = '' }: StatChipProps) {
  const toneClass = tone === 'signal' ? ' ds-statchip--signal' : '';
  return (
    <span className={`ds-statchip${toneClass} ${className}`.trim()}>
      {label ? <span className="ds-statchip__label">{label}</span> : null}
      <span className="ds-statchip__value">{value}</span>
    </span>
  );
}
