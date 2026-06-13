import { GamePhase, type PendingSelection } from '@/types/game';
import { PHASE_NAMES } from '@/utils/constants';

interface PromptBarProps {
  currentPhase: GamePhase;
  pendingSelection: PendingSelection | null;
  /** 1 = player 1 (you), 2 = player 2 (opponent) */
  localPlayer: number;
  isGameOver: boolean;
  /** Paced bot mode (add-bot-action-pacing): a non-human agent is taking
   *  its actions beat by beat; human input is locked while true. */
  agentActing?: boolean;
}

/** Phase-based default prompts when no pendingSelection prompt is available. */
const PHASE_PROMPTS: Partial<Record<GamePhase, string>> = {
  [GamePhase.Start]: 'Starting phase…',
  [GamePhase.Draw]: 'Drawing cards…',
  [GamePhase.Breeding]: 'Breeding phase — Hatch a Digi-Egg or skip.',
  [GamePhase.Main]: 'Main phase — Play, digivolve, attack, or pass.',
  [GamePhase.End]: 'End phase.',
  [GamePhase.SelectTarget]: 'Select a target.',
  [GamePhase.SelectMaterial]: 'Select evolution material.',
  [GamePhase.BlockTiming]: 'Block timing — Select a Digimon to block, or decline.',
  [GamePhase.CounterTiming]: 'Counter timing — Use a counter effect, or decline.',
  [GamePhase.SelectTrash]: 'Select a card from the trash.',
  [GamePhase.SelectSource]: 'Select a digivolution source.',
  [GamePhase.SelectHand]: 'Select a card from your hand.',
  [GamePhase.SelectReveal]: 'Select from revealed cards.',
  [GamePhase.SelectEffectChoice]: 'Choose an effect.',
  [GamePhase.SelectSecurity]: 'Select a security card.',
  [GamePhase.EndOfTurnAction]: 'End-of-turn actions available.',
  [GamePhase.AllianceTiming]: 'Alliance timing — Select an alliance partner, or decline.',
  [GamePhase.Mulligan]: 'Choose to keep your hand or mulligan.',
  [GamePhase.SelectPermutation]: 'Place the remaining cards on the deck in any order — click them in the order you want them placed.',
  [GamePhase.SelectUnion]: 'Choose a [Union]/Xros material.',
  [GamePhase.SelectBudgeted]: 'Choose cards within the budget.',
  [GamePhase.SelectBreedingPermanent]: 'Choose a Digimon to send to breeding.',
  [GamePhase.SelectPlayOrder]: 'Choose who plays first.',
};

/** Selection/interrupt phases where a prompt bar is most useful. */
const SELECTION_PHASES = new Set<GamePhase>([
  GamePhase.SelectTarget,
  GamePhase.SelectMaterial,
  GamePhase.BlockTiming,
  GamePhase.CounterTiming,
  GamePhase.SelectTrash,
  GamePhase.SelectSource,
  GamePhase.SelectHand,
  GamePhase.SelectReveal,
  GamePhase.SelectEffectChoice,
  GamePhase.SelectSecurity,
  GamePhase.EndOfTurnAction,
  GamePhase.AllianceTiming,
  GamePhase.SelectPermutation,
  GamePhase.SelectUnion,
  GamePhase.SelectBudgeted,
  GamePhase.SelectBreedingPermanent,
  GamePhase.SelectPlayOrder,
]);

export function PromptBar({
  currentPhase,
  pendingSelection,
  localPlayer,
  isGameOver,
  agentActing = false,
}: PromptBarProps) {
  if (isGameOver) return null;

  // Paced bot sequence in progress — the strongest signal: the player
  // can't act regardless of phase/selection until the beats drain.
  if (agentActing) {
    return (
      <div
        data-testid="prompt-bar"
        className="flex items-center justify-center gap-2 px-4 py-2 bg-amber-900/40 border-b border-amber-700/50"
      >
        <span className="inline-block w-2 h-2 rounded-full bg-amber-400 animate-pulse" />
        <span className="text-sm text-amber-200">Opponent is acting…</span>
      </div>
    );
  }

  // Determine if opponent is the one selecting
  const isOpponentSelecting =
    pendingSelection != null && pendingSelection.selectingPlayer !== localPlayer;

  if (isOpponentSelecting) {
    return (
      <div
        data-testid="prompt-bar"
        className="flex items-center justify-center gap-2 px-4 py-2 bg-amber-900/40 border-b border-amber-700/50"
      >
        <span className="inline-block w-2 h-2 rounded-full bg-amber-400 animate-pulse" />
        <span className="text-sm text-amber-200">Waiting for opponent…</span>
      </div>
    );
  }

  // Show prompt bar for selection/interrupt phases, or when there's an explicit prompt
  const hasPrompt = pendingSelection?.prompt;
  const isSelectionPhase = SELECTION_PHASES.has(currentPhase);

  if (!hasPrompt && !isSelectionPhase) return null;

  // Build contextual prompt: prefer server prompt, fall back to phase-based
  const promptText = hasPrompt
    || PHASE_PROMPTS[currentPhase]
    || `${PHASE_NAMES[currentPhase] ?? 'Unknown phase'}`;

  // Color coding by phase category
  let barClass = 'bg-blue-900/40 border-blue-700/50 text-blue-200';
  if (
    currentPhase === GamePhase.BlockTiming ||
    currentPhase === GamePhase.CounterTiming
  ) {
    barClass = 'bg-red-900/40 border-red-700/50 text-red-200';
  } else if (currentPhase === GamePhase.AllianceTiming) {
    barClass = 'bg-indigo-900/40 border-indigo-700/50 text-indigo-200';
  } else if (
    currentPhase === GamePhase.SelectTarget ||
    currentPhase === GamePhase.SelectMaterial
  ) {
    barClass = 'bg-purple-900/40 border-purple-700/50 text-purple-200';
  }

  return (
    <div
      data-testid="prompt-bar"
      className={`flex items-center justify-center px-4 py-2 border-b ${barClass}`}
    >
      <span className="text-sm font-medium">{promptText}</span>
      {pendingSelection?.isOptional && (
        <span className="ml-2 text-xs opacity-70">(Optional)</span>
      )}
    </div>
  );
}

