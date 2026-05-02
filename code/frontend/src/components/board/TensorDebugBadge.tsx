import type { TensorSummary } from '@/types/game';

interface TensorDebugBadgeProps {
  summary: TensorSummary | null;
}

export function TensorDebugBadge({ summary }: TensorDebugBadgeProps) {
  if (!summary) return null;

  return (
    <div className="ib-tensor-badge" aria-label="Board tensor summary">
      <span>{summary.profileId}</span>
      <span>P{summary.playerId}</span>
      <span>T{summary.tensorSize}</span>
      <span>A{summary.maskSize}</span>
      <span>L{summary.legalActionCount}</span>
      <span>{summary.phase}</span>
    </div>
  );
}
