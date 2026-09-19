# NOTES — Monica Simmons (BT25-091)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-091`):
**4 clauses** — `effect#0`, `effect#1`, `effect#2`, `effect#3`.
DCGO script: `BT25/Black/BT25_091.cs` exists — the card is **not** `unavailable`.
All four clauses have a scenario; none is `unreachable`. No verdict is stored
yet: every clause is `unmeasured` until an oracle run (this stage emitted no jobs).

Book: `tm_bt25_091_pool.json` (decks `tm-monica`, `quiet-opponent`).

| Clause | Scenario | Sim-only (2026-09-18) |
|---|---|---|
| `BT25-091#effect#0` `[Start of Your Turn] If you have 2 or less memory, set it to 3.` | `BT25-091-effect0.yaml` | lowers, asserts pass (audited, unchanged) |
| `BT25-091#effect#1` `[On Play]` return a [TS] Option, else `<Draw 1>` | `BT25-091-effect1.yaml` | oracle CONFIRMED 2026-09-19 after YAML fix (see below) |
| `BT25-091#effect#2` `[Your Turn] When you use [TS] trait Option cards, by suspending this Tamer …` | `BT25-091-effect2.yaml` | **new** — lowers, asserts pass |
| `BT25-091#effect#3` `[Security] Play this card without paying the cost.` | `BT25-091-effect3.yaml` | lowers, asserts pass (audited, unchanged) |

## Prompt shapes read off `BT25_091.cs`
- `effect#1`: `SetUpActivateClass(…, -1, FALSE, …)` — no OptionalSkill; the
  "may" is the `SelectCardEffect` over the trash (`canNoSelect: () => true`),
  asked only when a [TS] Option is there; otherwise `<Draw 1>` with no prompt.
  `effect1` puts Iron Slash in the trash through a SECURITY flip (so it never
  meets our from-hand mode-select), against a bare Lv.3 (no De-Digivolve count
  row: `MaxCount` 0).
- `effect#2`: `SetUpActivateClass(…, -1, TRUE, …)` — OptionalSkill yes/no;
  `CanActivate` needs `CanActivateSuspendCostEffect` (Tamer unsuspended). Body:
  suspend Monica, then one `SelectPermanentEffect` (`canNoSelect: false`).

## `effect#2` — prompt ORDER, verified on both sides before Unity
DCGO's option-use path (`CardController.cs` ~2000) only STACKS `OnUseOption`
triggers and then runs the Option's own `OptionSkill` effects at once, so the
sequence is Iron Slash `[Main]` pick → Monica's OptionalSkill → Monica's pick.
Our engine lowers the line in exactly that order (a line authored the other way
round would not lower). p0 fields only the Tamer, so neither engine asks the
`[Main]`'s "you may link" pick and p0 never addresses a slot.

*Witness limit:* the can't-attack lock is a modifier the state projection does
not carry. The line shows Monica `suspended: true` and Iron Slash in the trash;
the lock itself would only be visible as a missing attack bit on p1's turn, and
a scripted illegal attack cannot lower.

## F-ENGINE-PLUGIN-MODE-SELECT-WITHOUT-HOST (our bug — measured, not fixed)
Found while authoring `effect#2`. With **no Digimon at all** on p0's board
(only the Tamer Monica), playing Iron Slash BT25-100 from hand still parks our
`[Main]`/`[Link]` mode-select ("Play as a [Main] Option" / "Plug in via Link
Requirements (Cost 2)"). The printed Link Condition names a DIGIMON ("into the
specified Digimon in the battle area") and `BT25_100.cs` gates the host on
`IsDigimon && HasTSTraits`; DCGO offers no link declaration here.
Scratch probe, taking the "Plug in" branch: **no host pick is asked, memory
goes 3 → 1 (the link cost IS paid), Iron Slash lands in the TRASH linked to
nothing, and Monica's "when you use" trigger fires.** So our engine offers and
executes a link declaration with zero legal hosts — an illegal action in the
RL action space. Likely site: the mode-select's "is a link legal" check in
`Game::play_option_core` (it should require at least one host satisfying the
card's link condition, and the branch should fizzle-free refuse otherwise).
The committed line takes the `[Main]` branch through a `sim_only` `choice:` row,
so the finding does not contaminate the clause. Same behaviour is expected for
every dual-mode Plug-In Option (BT25-093, BT24-091, …). Needs the card-fix gate
(failing-then-passing test); not applied in this stage.

## `effect#2` oracle divergence — triaged 2026-09-19 (rules-ambiguous, not fixed)
`--all-diffs` against the preserved sidecar
(`20260918T130112Z_6a17348e0757476cbf19bd6e8b1a8090.state.jsonl`): only steps 9-10 differ,
`p0.trash ours=[] dcgo=[BT25-100]` — DCGO has trashed Iron Slash before Monica's stacked
OnUseOption trigger resolves (`CardController.cs:2000-2126`); ours trashes after the
observers (`finish_option_after_body`). Suspension, lock target and the post-clause board
match. general_rule.pdf 9-1-5 + 18-1-2 + 15-4-3 put the Option's trash (pending processing)
and the pending trigger at the same timing with the turn player choosing the order, so each
engine hard-codes one legal order. Logged as G-ENGINE-OPTION-TRASH-VS-ON-USE-TRIGGER-ORDER in
`docs/RUST_ENGINE_GAPS.md`. Verdict stays `diverged`. F-ENGINE-PLUGIN-MODE-SELECT-WITHOUT-HOST
(above) is independent of this divergence (the sim-only row absorbs it).

## `effect#1` oracle divergence — triaged 2026-09-19 (our bug, FIXED)

Lead: step 14 `p0.hand` — ours carried an extra BT3-007 (8 cards vs DCGO's 7).
Printed text (official DB bundle): "If this effect didn't return, ＜Draw 1＞";
Q&A: the draw happens if you choose not to return. DCGO `BT25_091.cs:52-93`
draws only on `!returned`, set iff a card was picked. Our YAML gated the draw
on `effect_returned_any_card: false`, but `add_to_hand_from_trash` records its
move in the result log's `added_to_hand`, never `returned_cards` (that list
tracks field/deck returns) — so the gate was always true and the draw fired
even after a successful return. Card-level fix: gate on
`effect_added_any_card_to_hand: false`. Tests:
`bt25_091_on_play_return_does_not_draw` (failed before, passes after) and
`bt25_091_on_play_declined_return_draws`. The oracle diff against the
preserved sidecar `20260918T130047Z_8f408f4b…` is now CLEAN; the scenario now
asserts the DCGO-observed `p0.hand`. No engine change.
