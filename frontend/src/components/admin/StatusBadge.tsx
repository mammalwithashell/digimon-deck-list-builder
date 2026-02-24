const STATUS_COLORS: Record<string, string> = {
  queued: 'bg-gray-500',
  running: 'bg-blue-500 animate-pulse',
  completed: 'bg-green-600',
  failed: 'bg-red-600',
  applied: 'bg-emerald-500',
  pending: 'bg-yellow-600',
  canceled: 'bg-gray-600',
};

export function StatusBadge({ status }: { status: string }) {
  const color = STATUS_COLORS[status] ?? 'bg-gray-500';
  return (
    <span
      className={`inline-block px-2 py-0.5 rounded-full text-xs text-white font-medium ${color}`}
    >
      {status}
    </span>
  );
}
