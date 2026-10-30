# PR draft — Format selection: Singleton and EDEN

> Working draft. Filled out fully by task 10.8; created early to carry the
> contributor guard below, which constrains every commit on this branch.

**Branch:** `format-selection` → `DCGO2/DCGO:develop`

---

## Contributor guard — read before committing

1. **No dependency on `Assets/Scripts/Script/Recording/`.** That directory is a
   downstream fork's game-recording harness. It does not exist on `develop`, so the
   constraint is structurally enforced here — but if this branch is ever rebased onto a
   tree that has it, nothing in this change may reference it.
2. **Never commit `Assets/CardBaseEntity/**` or other `.asset` files.** LFS smudge is
   configured with `--skip`, so a checkout that holds real assets shows thousands of files
   as modified against their pointer blobs. Those modifications are not yours.
3. **Legacy room names are a compatibility contract.** `"<5 digits>-True"`,
   `"<5 digits>-False"`, and the `randomMatchRoom` key must remain byte-identical so
   clients without this change keep matching. See `baseline.md`.
4. **The selected format must never reach `DeckBuildingRule.ModifiedDeckData`.** That
   function deletes cards and is run library-wide with a save. Format legality is
   reported, never repaired.

---

## What this adds

A play **format** axis, generalising the existing `useBanlist` boolean rather than sitting
beside it, with two new formats:

| Format | Rarity | Banlist | Copies |
|---|---|---|---|
| Standard | all | official | 4 |
| No Banlist | all | none | 4 |
| **Singleton** | all | none | **1** (across main + egg) |
| **EDEN** | EDEN pool + Anomaly Protocol | EDEN's own | 4 |

`useBanlist == true` is exactly Standard and `== false` is exactly No Banlist, so existing
behaviour is preserved by construction.

Each format gets its own matchmaking queue, and both players' decklists are validated
against the agreed format client-side — using the deck data already published to the room,
so no server and no trust in the opponent's claim are required.

## Compatibility

- Legacy room names and the legacy random-match key are unchanged, so clients without this
  change match exactly as before.
- New formats use *distinct* keys rather than suffixes, so an unmodded client's
  `Contains("randomMatchRoom")` scan cannot match a format room.
- The existing mixed Standard/No-Banlist random-match pool is **deliberately not split** —
  that is a separate defect, reported separately, and splitting it here would fragment the
  live player base.
- Unknown format ids resolve to Standard, so older clients and reverted builds degrade to
  current behaviour.

## Notes for reviewers

- EDEN's banlist and Anomaly Protocol ship as **data**, not code, so a rules revision is a
  data-only change. If you would rather serve them from `dcgo.online/Banlist.json`
  alongside the official list, that is a small follow-up and removes a drift risk — happy
  to do it that way instead.
