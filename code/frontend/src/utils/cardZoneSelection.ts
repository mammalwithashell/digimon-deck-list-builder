import { SELECTION } from "./constants";
import { isTrashAction } from "./trashSelection";
import { isSourceSelectAction, sourceSelectionCards } from "./sourceSelection";
import {
  isAnyFieldSelectionKind,
  isFieldSelectionKind,
} from "./selectionTargets";
import type { PendingSelection, PermanentInfo } from "@/types/game";

/**
 * Card-zone selections: pending selections whose valid action ids each name a
 * concrete CARD in a hand, a trash, or a battle-area stack — possibly mixing
 * zones in one prompt (`SelectionKind::UnionZone` hand + trash picks,
 * budgeted multi-picks such as BT26-081's "up to 8 play cost's worth from your
 * hand or trash", BT26-060 ordering the returned top stacked cards of the
 * OPPONENT's Digimon).
 *
 * Ids decode by range, with `pendingSelection.zoneOwner` (absent = the
 * selecting player) saying whose zone they index:
 *  - hand    `SELECTION.HAND_START + i`        → that player's hand[i]
 *  - trash   `SELECTION.TRASH_START + i`       → that player's trash[i]
 *  - source  `SOURCE_SELECT_START + f*12 + s`  → that player's battleArea[f], stack index s
 *
 * Field-target and any-field prompts are board-clicked; trash-ONLY prompts are
 * owned by `TrashSelectModal`. Everything else whose ids all decode here is
 * rendered by the `SelectionPanel` card picker.
 */
export interface CardZoneTile {
  actionId: number;
  cardId: string;
  zone: "hand" | "trash" | "source";
}

export interface CardZoneContext {
  /** Local player's id on the wire (1 or 2). */
  localPlayer: number;
  ownHandIds: string[];
  ownTrashIds: string[];
  opponentTrashIds: string[];
  ownBattleArea: PermanentInfo[];
  opponentBattleArea: PermanentInfo[];
}

function isHandAction(id: number): boolean {
  return id >= SELECTION.HAND_START && id <= SELECTION.HAND_END;
}

/** Resolve every valid id to a card tile; returns null if any id doesn't
 *  decode to a card (so the prompt isn't a card-zone selection). */
export function cardZoneSelectionTiles(
  pending: PendingSelection,
  ctx: CardZoneContext,
): CardZoneTile[] | null {
  const opponentZone =
    pending.zoneOwner != null && pending.zoneOwner !== ctx.localPlayer;
  const trash = opponentZone ? ctx.opponentTrashIds : ctx.ownTrashIds;
  const area = opponentZone ? ctx.opponentBattleArea : ctx.ownBattleArea;
  const tiles: CardZoneTile[] = [];
  for (const id of pending.validIndices) {
    if (isHandAction(id)) {
      // An opponent's hand is hidden: never a card-zone pick for us.
      if (opponentZone) return null;
      const cardId = ctx.ownHandIds[id - SELECTION.HAND_START];
      if (!cardId) return null;
      tiles.push({ actionId: id, cardId, zone: "hand" });
    } else if (isTrashAction(id)) {
      const cardId = trash[id - SELECTION.TRASH_START];
      if (!cardId) return null;
      tiles.push({ actionId: id, cardId, zone: "trash" });
    } else if (isSourceSelectAction(id)) {
      const [tile] = sourceSelectionCards([id], area);
      if (!tile) return null;
      tiles.push({ actionId: id, cardId: tile.cardId, zone: "source" });
    } else {
      return null;
    }
  }
  return tiles;
}

/**
 * True when the `SelectionPanel` card picker should render `pending`: a
 * card-zone selection that no other surface owns (not a field / any-field
 * board pick, not a trash-only modal pick, not a keyword prompt).
 */
export function isCardZoneSelection(
  pending: PendingSelection | null | undefined,
  ctx: CardZoneContext,
): boolean {
  if (!pending || pending.keywordPrompt) return false;
  if (pending.validIndices.length === 0) return false;
  if (
    isFieldSelectionKind(pending.kind) ||
    isAnyFieldSelectionKind(pending.kind)
  )
    return false;
  // Trash-only prompts belong to TrashSelectModal.
  if (pending.validIndices.every(isTrashAction)) return false;
  return cardZoneSelectionTiles(pending, ctx) !== null;
}
