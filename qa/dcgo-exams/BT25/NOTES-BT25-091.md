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
| `BT25-091#effect#1` `[On Play]` return a [TS] Option, else `<Draw 1>` | `BT25-091-effect1.yaml` | lowers, asserts pass (audited, unchanged) |
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
