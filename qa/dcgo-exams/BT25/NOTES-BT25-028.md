# NOTES — Dianamon (BT25-028)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-028`):
**5 clauses** — `effect#0`, `effect#1`, `effect#2`, `effect#3`, `inherited#0`.
DCGO script: `BT25/Blue/BT25_028.cs` exists — the card is **not** `unavailable`.
**3 of 5 clauses have a scenario. `effect#3` and `inherited#0` are NOT authored
and stay `unmeasured`** — neither is `unreachable`; the reasons are below. No
verdict is stored yet (this stage emitted no jobs).

Book: `tm_bt25_028_pool.json` (decks `tm-ts-line`, `tm-ts-opponent`).

| Clause | Scenario | Sim-only (2026-09-18) |
|---|---|---|
| `BT25-028#effect#0` `[Digivolve] Lv.5 w/[TS] trait: Cost 3` | `BT25-028-effect0.yaml` | lowers, asserts pass (audited, unchanged) |
| `BT25-028#effect#1` play-cost −5 vs an opposing Lv.6+ | `BT25-028-effect1.yaml` | lowers, asserts pass (audited, unchanged) |
| `BT25-028#effect#2` `[On Play] [When Digivolving]` lock + delete | `BT25-028-effect2.yaml` | lowers, asserts pass (audited; steps unchanged, memory-ledger comments corrected — Agumon ST1-03 costs 3) |
| `BT25-028#effect#3` `[All Turns] [Once Per Turn]` trash any 4 / DNA | — | **not authored** (below) |
| `BT25-028#inherited#0` `[When Attacking] [Once Per Turn]` can't suspend | — | **not authored** (below) |

## The MultipleSkills prediction every Dianamon digivolve line rests on
Dianamon's own digivolve satisfies BOTH its `[When Digivolving]` body and its
`[All Turns]` "when any Digimon … digivolve" trigger (`CanUseCondition`:
`CanTriggerWhenPermanentDigivolving(hashtable, IsDigimonCondition)`; `CanActivate`
is only `IsExistOnBattleArea`). Two active skills ⇒ DCGO asks `MultipleSkills`.
Checked 2026-09-18 against `MultipleSkills.cs`: `SetIsSkippable(true)` only makes
that panel DECLINABLE (`_CanNoSelect`); it never suppresses the ordering prompt.
Our engine bundles nothing on a digivolve and parks the delete pick directly,
so the row is authored `dcgo_only`, `ordinal: 0` (predicted = the
`[When Digivolving]` body, by registration order). **Unverified until an oracle
run**: if DCGO orders the pair the other way the run aborts on that row and the
abort message prints DCGO's order — flip the ordinal then. The same row appears
in `BT25-026-effect2.yaml` and `BT25-026-inherited0.yaml`.

**Engine findings riding these lines (not fixed here):**
- *Play vs digivolve asymmetry (measured at lowering).* When Dianamon is
  PLAYED (`BT25-028-effect1.yaml`) our engine DOES park a TriggerOrder over the
  same two triggers, so that row is SHARED there; when it DIGIVOLVES
  (`effect0`, `effect2`, the two BT25-026 lines) it parks nothing and the row is
  `dcgo_only`. One card, one trigger pair, two behaviours.
- *Missing ordering decision.* Two simultaneous own triggers on a digivolve
  resolve in a fixed order on our side with no TriggerOrder prompt; DCGO (and
  §15-8-5) give the player the order.
- *Vacuous optional gate.* `BT25-028.yaml` authors the [All Turns] body
  `optional: true`; `BT25_028.cs` registers it `isOptional: false` (the "may"
  lives inside the body, on the picks). Our yes/no gate is therefore answered
  `sim_only` in every line.

## `effect#3` — attempted 2026-09-18, NOT authored
*DNA half:* needs `[GraceNovamon]` in hand. **Neither printing (BT25-103,
EX5-073) has a card spec in our engine** (`code/digimon-engine/cards/…` has no
YAML for either; DCGO scripts both). A line through it would measure the
missing card, not this clause. Blocked until GraceNovamon is implemented.

*Trash half:* a line was drafted and **does lower** — the `BT25-028-effect0.yaml`
prefix with p1 fielding a bare Biyomon (for the mandatory delete) plus a
Gabumon ST2-03 bred onto Tsunomon ST2-01 (a stack with exactly ONE digivolution
card, so our flat `select_opponent_sources` prompt has one candidate and DCGO's
`SelectTrashDigivolutionCards` loop ends after one round). Rows after the
digivolve: `dcgo_only` MultipleSkills → shared delete pick (Biyomon) →
`sim_only` `yes:` on our [All Turns] gate → `dcgo_only` SelectPermanentEffect
`[ST2-03]` → our source pick (`sim_only`) → `dcgo_only` SelectCardEffect
`[ST2-01]` → pass.
**Measured problem:** with our source pick answered as `yes: true` OR as
`targets: [opp.field.0]` the step lowers, the prompt closes (a second
`sim_only` row then reports "OUR engine has NO live prompt"), and **the source
is not trashed** (`p1.field` Gabumon still `sources: [ST2-01]`, `p1.trash`
`[ST1-02]`). From the harness alone I could not tell whether the answer
resolved to the prompt's end-selection id (a vocabulary limit: the resolver
cannot name a digivolution card by `cards:` — the `BT21-074-effect2.yaml`
finding — and this prompt is `min: 0`), or whether the pick was taken and
`trash_selected_sources` did not run (an engine/card bug). Separating the two
needs an engine-level look (a DebugRunner test or the engine MCP), which is
outside this stage. The draft was NOT committed: a file whose clause effect
never happens would "cover" the clause while exercising nothing.

## `inherited#0` — NOT authored (reachable in principle)
Needs Dianamon as a digivolution card under an attacking Digimon, i.e. a Lv.7
on top. The archetype's own host, GraceNovamon, has no card spec (above).
Implemented AND DCGO-scripted Lv.7s that accept a blue Lv.6 exist — e.g.
Omnimon BT13-112 (blue Lv.6, cost 4), Omnimon EX12-037 (cost 5), Omnimon
Alter-S EX9-021 / EX4-060 — but each brings its own `[When Digivolving]` /
`[When Attacking]` prompts that would stack with Dianamon's inherited trigger
in an order only the oracle can confirm. Not attempted in this session. A
single-Digimon line is possible (hard-play Dianamon for 12, digivolve the Lv.7
next turn, attack the turn after), so slot addressing is not an obstacle.
Prompt shape to expect from `BT25_028.cs` region "When Attacking ESS":
`isOptional: false`, one `SelectPermanentEffect` (`canNoSelect: false`) over
the opponent's Digimon/Tamers, guarded by `HasMatchConditionOpponentsPermanent`.
