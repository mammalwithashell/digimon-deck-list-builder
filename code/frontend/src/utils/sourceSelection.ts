// Source-card (digivolution material) selection helpers.
//
// `select_material` / `SelectSource` effects (e.g. ST2-15 Kaiser Nail "play 1
// source from under 1 of your Digimon") park the engine on a selection whose
// `valid_action_ids` live in the SOURCE_SELECT range:
//   battle-area source:  SOURCE_SELECT_START + field*SOURCES_PER_FIELD + source
// where `source` indexes the permanent's stack at `field`, bottom→top (the top
// card is the last index; most effects only offer the non-top ones). These helpers map those ids back to the card they act
// on so the UI can render a clickable picker — there is no board affordance for
// picking a card *inside* a stack (the board's SelectMaterial path is for the
// raw-field-index DNA-digivolve pick, a different mechanism).

export const SOURCE_SELECT_START = 2000;
export const SOURCES_PER_FIELD = 12;
/** Breeding-carrier sources start here; battle-area sources are below this. */
export const BREEDING_SOURCE_SELECT_START = 2168;
export const SOURCE_SELECT_END = 2192;

/** True for any SOURCE_SELECT-range action id (battle-area or breeding). */
export function isSourceSelectAction(id: number): boolean {
  return id >= SOURCE_SELECT_START && id < SOURCE_SELECT_END;
}

interface SourceLike {
  cardId: string;
  cardName: string | null;
  isTop: boolean;
}
interface PermLike {
  sources: SourceLike[];
  /** Link cards; a source index past the stack addresses `linkedCardIds[i - stack length]`. */
  linkedCardIds?: string[];
}
export interface SourceTile {
  actionId: number;
  cardId: string;
  cardName: string | null;
}

/**
 * Resolve battle-area SOURCE_SELECT action ids to pickable source tiles. Each
 * id decodes to `(field, sourceIndex)`; the tile's card is the `sourceIndex`-th
 * NON-TOP source of `battleArea[field]` (bottom→top, matching the engine's
 * `encode_source_select`). Ids that don't resolve (missing permanent/source)
 * are skipped. Breeding-carrier source ids are out of scope here.
 */
export function sourceSelectionCards(
  validIndices: number[],
  battleArea: PermLike[],
): SourceTile[] {
  const tiles: SourceTile[] = [];
  for (const id of validIndices) {
    // Battle-area sources only; breeding-carrier ids are out of scope here.
    if (id < SOURCE_SELECT_START || id >= BREEDING_SOURCE_SELECT_START) continue;
    const offset = id - SOURCE_SELECT_START;
    const field = Math.floor(offset / SOURCES_PER_FIELD);
    const sourceIndex = offset % SOURCES_PER_FIELD;
    const perm = battleArea[field];
    if (!perm) continue;
    // Engine `card_sources` order: the NON-TOP cards bottom→top, then the top
    // card last. Rebuild that from `sources` (independent of where the wire
    // puts the top) so a non-top index resolves exactly as before and an
    // effect that also offers the TOP card (BT26-060 returning the top 5
    // stacked cards) resolves to it.
    const top = perm.sources.find((s) => s.isTop);
    const stack = [...perm.sources.filter((s) => !s.isTop), ...(top ? [top] : [])];
    const card = stack[sourceIndex];
    if (card) {
      tiles.push({ actionId: id, cardId: card.cardId, cardName: card.cardName });
      continue;
    }
    // Past the stack: the engine's zone-card picks encode a LINK card there
    // (`zone_cards.rs`, BT26-102 Seven Code PAD materials).
    const linkId = perm.linkedCardIds?.[sourceIndex - stack.length];
    if (linkId) tiles.push({ actionId: id, cardId: linkId, cardName: null });
  }
  return tiles;
}
