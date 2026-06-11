import { useUiStore, type BotSpeed } from '@/stores/uiStore';

const SPEEDS: { value: BotSpeed; label: string; title: string }[] = [
  { value: 'slow', label: 'Slow', title: 'Bot acts every 3s' },
  { value: 'normal', label: 'Normal', title: 'Bot acts every 1.5s' },
  { value: 'fast', label: 'Fast', title: 'Bot acts every 0.7s' },
  { value: 'instant', label: 'Instant', title: 'Bot turn resolves at once' },
];

/** Segmented control for the bot action pacing speed
 *  (add-bot-action-pacing). Persisted in uiStore; changes apply from the
 *  next agent beat without restarting the game. */
export function BotSpeedControl() {
  const botSpeed = useUiStore((s) => s.botSpeed);
  const setBotSpeed = useUiStore((s) => s.setBotSpeed);

  return (
    <div
      data-testid="bot-speed-control"
      className="flex items-center gap-1 text-xs"
    >
      <span className="text-gray-500 mr-1">Bot speed</span>
      {SPEEDS.map(({ value, label, title }) => (
        <button
          key={value}
          type="button"
          title={title}
          onClick={() => setBotSpeed(value)}
          className={`px-2 py-0.5 rounded border transition-colors ${
            botSpeed === value
              ? 'bg-cyan-700/60 border-cyan-500 text-cyan-100'
              : 'bg-gray-800/60 border-gray-700 text-gray-400 hover:text-gray-200'
          }`}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
