# NOTES — Iron Slash (BT25-100)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-100`):
**8 clauses** — `effect#0` … `effect#7`.
DCGO script: `BT25/Black/BT25_100.cs` exists — the card is **not** `unavailable`.
**7 of 8 clauses have a scenario; `effect#5` is `unreachable`** (measured reason
below). No verdict is stored yet: the seven authored clauses are `unmeasured`
until an oracle run (this stage emitted no jobs).

Book: `tm_bt25_100_pool.json` (decks `tm-red-attacker`, `tm-blue-defender`).

Clause map (positional split of the official text, `data/card_bundles/BT25-100.md`):

| Clause | Printed | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `effect#0` | `<Use Req. ([TS] trait)>` | `BT25-100-effect0.yaml` | **new** — lowers, asserts pass |
| `effect#1` | `[Security] Activate this card's [Main] effects.` | `BT25-100-effect1.yaml` | count row added — lowers, asserts pass |
| `effect#2` | `[Main]` (marker; empty clause text) | `BT25-100-effect2.yaml` | count row added — lowers, asserts pass |
| `effect#3` | `<De-Digivolve 2>` 1 of your opponent's Digimon. Then, you may link … | `BT25-100-effect3.yaml` | count row added — lowers, asserts pass |
| `effect#4` | Link DP `DP+2000` | `BT25-100-effect4.yaml` | count row added, DP witness pinned — lowers, asserts pass |
| `effect#5` | Link Condition `<Link> [TS] trait: Cost 2` | — | **`unreachable`** (below) |
| `effect#6` | Link Effect `<Collision>` | `BT25-100-effect6.yaml` | count row added — lowers, asserts pass |
| `effect#7` | Link Effect `<Piercing>` | `BT25-100-effect7.yaml` | count row added — lowers, asserts pass |

## What the pre-Unity review changed in the six older files
1. **The `<De-Digivolve 2>` COUNT row (would have aborted every line).**
   `SelectPermanentEffect.cs:1008` builds `new IDegeneration(target, 2, cardEffect)`
   with no `DegenerationCountRuling`, so `Degeneration()` opens `SelectCountEffect`
   "How many cards do you trash?" with `MaxCount = min(digivolution cards, 2)`,
   `CanNoSelect: false`. Against the four-card Vermilimon stack that is
   candidates `{1, 2}` — a REAL choice (general_rule 16-11 "trash up to x",
   16-11-6 "the declared number"). Our engine parks nothing and always takes
   the maximum — the tracked rule-17 gap **G-DEDIGIVOLVE-NO-DECLARATION**
   (`docs/RUST_ENGINE_GAPS.md`). Each line now carries a `dcgo_only`
   `value: 2` row after the target pick (actor = the effect's owner, p1), and
   every later assert index moved by one. Against a bare Lv.3 (`effect0`) there
   is no row: `MaxCount` 0 and `Activate()` skips the prompt.
2. **Stale header text.** (a) The "PREDICTED DIVERGENCE AT THE LINK ROW" is
   resolved — engine 35958972b (G-ENGINE-SECURITY-OPTION-LINK-TO-OWN-DIGIMON)
   lifts a security-flipped Option onto the ordinary LinkSelectHost path.
   Re-measured sim-side: the link row is a LIVE OwnField pick, Kamemon reads
   3000 afterwards and p1's trash is empty. (b) "the scenario vocabulary has no
   way to answer [the mode-select]" is outdated — the `choice:` select form
   (harness d5f23792f) answers it; `effect0` uses it.
3. `effect4` (Link DP) now pins the actual witness (`p1.field` Kamemon
   `dp: 3000`, `p1.trash: []`); its asserts had been thinned out because of the
   stale divergence prediction.

Slot safety: in every line each seat addresses at most one permanent (p0 one
stack, p1 Kamemon; a linked Option is not a separate permanent).

*Scope note:* `effect1`, `effect2` and `effect3` all reach the `[Main]` body
through `[Security]`. That is the right route for `effect#1`; for `effect#2`
(the `[Main]` marker) a from-hand use would be the more literal witness —
`effect0` is that line and could be pointed at by a sister file.

## `effect#5` — `unreachable`: no shared action means "plug this Option in from hand"
The Link Condition's COST (2) is only ever paid by a hand declaration — the
`[Main]`'s own link is "without paying the cost" — and an Option has no
battle-area origin to link from (it is trashed or linked on use; no `<Delay>`).
Measured 2026-09-18:
- `link: { card: BT25-100, from: hand }` → **`no legal action matches Link`**.
  The hand-link `HAND_EFFECT` bit exists for Link DIGIMON only
  (`Game::activate_hand_link`, G-ENGINE-DIGIMON-LINK-FROM-HAND).
- On OUR side the Option plug-in lives on the PLAY bit behind the mode-select:
  `play:` + `choice: "Plug in"` parks the OwnField host pick (scratch probe).
- On DCGO the same declaration is a separate `ActivateCardAction` over the
  `LinkEffect`, which the harness emits only from a `HAND_EFFECT` bit
  (`InputDriver.BuildMainPhaseAction`); a Play-bit wire row is `PlayCardAction`
  = the `[Main]` use.
So one wire integer means two different things and no line can mean "hand
plug-in" on both engines. To make it reachable either the engine exposes the
Option hand-link on the `HAND_EFFECT` bit (as it does for Link Digimon), or the
emitter translates Play + branch 1 into DCGO's `ActivateCardAction`. The HOST
GATE half of the condition ([TS] Digimon) is exercised indirectly by every
line that links to Kamemon. Same clause, same reason: `BT25-093#effect#4`.

See also `NOTES-BT25-091.md`, F-ENGINE-PLUGIN-MODE-SELECT-WITHOUT-HOST — our
mode-select is offered (and its "Plug in" branch executes, paying 2 and
trashing the card) with NO legal Digimon host. Found with this card.
