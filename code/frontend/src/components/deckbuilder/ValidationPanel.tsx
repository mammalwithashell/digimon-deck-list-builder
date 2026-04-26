import { useDeckBuilderStore } from '@/stores/deckBuilderStore';

export function ValidationPanel() {
  const { validationResult } = useDeckBuilderStore();

  if (!validationResult) return null;

  const hasErrors = validationResult.errors.length > 0;
  const hasWarnings = validationResult.warnings.length > 0;

  if (!hasErrors && !hasWarnings) {
    return (
      <div className="text-xs text-green-400 p-2 bg-green-900/20 rounded">
        Deck is valid
      </div>
    );
  }

  return (
    <div className="space-y-1 p-2 bg-gray-800/50 rounded">
      {validationResult.errors.map((err, i) => (
        <div key={`e-${i}`} className="text-xs text-red-400 flex gap-1">
          <span className="font-bold">Error:</span>
          <span>{err.message}</span>
        </div>
      ))}
      {validationResult.warnings.map((warn, i) => (
        <div key={`w-${i}`} className="text-xs text-orange-400 flex gap-1">
          <span className="font-bold">Warning:</span>
          <span>{warn.message}</span>
        </div>
      ))}
    </div>
  );
}
