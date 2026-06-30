import { GamePhase, type PendingSelection } from '@/types/game';
import { isAnyFieldSelectionKind, isFieldSelectionKind } from './selectionTargets';
import { isSourceSelectAction } from './sourceSelection';

/**
 * Classify whether a pending selection is resolved by clicking permanents
 * directly on the board (no obscuring overlay) rather than through a modal.
 *
 * Board-driven selections:
 *  - Field-target picks (`OwnField` / `OppField`) — single OR capped-multi
 *    ("delete up to 2 of your opponent's Digimon"); the player clicks the
 *    Digimon on the board (see `handleSlotClick` / `fieldSelectionActionId`).
 *  - `AnyField` picks (`select_any_permanent` / `select_dna_pair`) — span both
 *    battle areas, also clicked on the board.
 *  - DNA-digivolve material picks (`SelectMaterial` whose valid ids are RAW
 *    battle-area indices, not `SOURCE_SELECT`-range ids) — clicked on the board.
 *
 * Everything else (hand / security / effect-choice / source-select / trash /
 * reveal / keyword prompts) pops a modal that covers the board.
 *
 * The `PeekButton` exists only to temporarily hide such board-covering modals;
 * for board-driven selections there is nothing to hide and the floating pill
 * just overlaps the prompt bar / effect description, so it must not render.
 */
export function isBoardDrivenSelection(
  pending: PendingSelection | null | undefined,
  phase: GamePhase,
): boolean {
  if (!pending) return false;

  // Field / any-field target clicks happen on the board.
  if (isFieldSelectionKind(pending.kind) || isAnyFieldSelectionKind(pending.kind)) {
    return true;
  }

  // DNA-digivolve material pick: SelectMaterial driven by RAW battle-area
  // indices (board clicks). The source-select variant of SelectMaterial uses
  // SOURCE_SELECT-range ids and is rendered in the SelectionPanel modal, so it
  // is NOT board-driven.
  if (
    phase === GamePhase.SelectMaterial &&
    !pending.validIndices.some(isSourceSelectAction)
  ) {
    return true;
  }

  return false;
}
