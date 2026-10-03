import { describe, expect, it } from "vitest";

import {
  cardZoneSelectionTiles,
  isCardZoneSelection,
} from "./cardZoneSelection";
import { sourceSelectionCards } from "./sourceSelection";
import { SELECTION } from "./constants";
import type { PendingSelection, PermanentInfo } from "@/types/game";

const SOURCE = 2000;
const perm = (ids: string[]): PermanentInfo =>
  ({
    sources: ids.map((cardId, i) => ({
      cardId,
      cardName: cardId,
      isTop: i === ids.length - 1,
      optState: 0,
      dpContribution: 0,
      mainEffectText: "",
      inheritedEffectText: "",
      colors: [],
    })),
  }) as unknown as PermanentInfo;

const ctx = {
  localPlayer: 1,
  ownHandIds: ["H0", "H1"],
  ownTrashIds: ["T0", "T1", "T2"],
  opponentTrashIds: ["OT0"],
  ownBattleArea: [perm(["A", "B"])],
  opponentBattleArea: [perm(["X", "Y", "Z"])],
};

const sel = (
  validIndices: number[],
  extra: Partial<PendingSelection> = {},
): PendingSelection =>
  ({
    kind: "UnionZone { zones: UnionZoneSet(3) }",
    selectingPlayer: 1,
    validIndices,
    isOptional: true,
    prompt: "",
    ...extra,
  }) as PendingSelection;

describe("card-zone selections", () => {
  it("resolves a hand + trash union pick to card faces", () => {
    const tiles = cardZoneSelectionTiles(
      sel([SELECTION.HAND_START + 1, SELECTION.TRASH_START + 2]),
      ctx,
    );
    expect(tiles).toEqual([
      { actionId: SELECTION.HAND_START + 1, cardId: "H1", zone: "hand" },
      { actionId: SELECTION.TRASH_START + 2, cardId: "T2", zone: "trash" },
    ]);
    expect(isCardZoneSelection(sel([0, SELECTION.TRASH_START]), ctx)).toBe(
      true,
    );
  });

  it("resolves source picks against the zoneOwner side, top card included", () => {
    // BT26-060: order the opponent's returned top stacked cards; index 2 is
    // the TOP card of a 3-card stack.
    const p = sel([SOURCE + 2, SOURCE + 1], {
      kind: "OrderedPermutation { remaining: 2 }",
      zoneOwner: 2,
    });
    expect(cardZoneSelectionTiles(p, ctx)?.map((t) => t.cardId)).toEqual([
      "Z",
      "Y",
    ]);
  });

  it("leaves trash-only, field and undecodable prompts to their own surfaces", () => {
    expect(isCardZoneSelection(sel([SELECTION.TRASH_START]), ctx)).toBe(false);
    expect(
      isCardZoneSelection(
        sel([SELECTION.OWN_FIELD_START], { kind: "OppField" }),
        ctx,
      ),
    ).toBe(false);
    expect(isCardZoneSelection(sel([SELECTION.TRASH_START + 9]), ctx)).toBe(
      false,
    );
    // Opponent's hidden hand never renders.
    expect(isCardZoneSelection(sel([0], { zoneOwner: 2 }), ctx)).toBe(false);
  });

  it("keeps non-top source indexing unchanged for existing source picks", () => {
    expect(
      sourceSelectionCards([SOURCE + 0], [perm(["A", "B"])])[0]?.cardId,
    ).toBe("A");
  });
});
