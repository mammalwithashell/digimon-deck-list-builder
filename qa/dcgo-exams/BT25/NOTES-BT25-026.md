# NOTES — Crescemon (BT25-026)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-026`):
**4 clauses** — `effect#0`, `effect#1`, `effect#2`, `inherited#0`.
DCGO script: `BT25/Blue/BT25_026.cs` exists — the card is **not** `unavailable`.
All four clauses have a scenario; none is `unreachable`. No verdict is stored
yet: every clause is `unmeasured` until an oracle run (this stage emitted no jobs).

Book: `tm_bt25_026_pool.json` (decks `tm-ts-line`, `tm-ts-opponent`).

| Clause | Scenario | Sim-only (2026-09-18) |
|---|---|---|
| `BT25-026#effect#0` `[Digivolve] Lv.4 w/[TS] trait: Cost 3` | `BT25-026-effect0.yaml` | lowers, asserts pass (audited, unchanged) |
| `BT25-026#effect#1` `[On Play] [When Digivolving] Trash the bottom 3 …` | `BT25-026-effect1.yaml` | lowers, asserts pass (audited, unchanged) |
| `BT25-026#effect#2` `[Your Turn] … may digivolve into [Dianamon] in the trash with the cost reduced by 2` | `BT25-026-effect2.yaml` | **repaired** — lowers, asserts pass |
| `BT25-026#inherited#0` `[Your Turn] This Digimon's attack target can't change.` | `BT25-026-inherited0.yaml` | **re-authored** — lowers, asserts pass |

## `effect#0` — what the line cannot separate
The pool has no non-blue Lv.4 with the [TS] trait, so every legal [TS] base
also satisfies the printed Blue Lv.4 circle at the same cost 3. One distinct
cost, no cost prompt on either side: the line measures "legal at exactly 3",
not "legal through the alt path alone". Stated in the file header.

## `effect#2` — two repairs (the crashed draft did not lower)
1. **Cost row.** The draft answered a shared `value: 2` "cost choice". Our
   engine asks no such thing on an effect-initiated digivolve: it auto-mins the
   route (`game_actions/digivolve.rs`, `effect_initiated_digivolve_from_source_inner`:
   "Costs auto-min (rule-17 cost CHOICE for effect digivolves is a known
   follow-up)"). DCGO does ask — `PlayCardClass.SelectCost` builds the list from
   the BASE costs `{3, 4}` (the −2 rides a separate `ChangeDigivolutionCost`
   effect applied afterwards) and opens `SelectCountEffect`. The row is now
   `dcgo_only`, `value: 3`, naming the route ours takes (3 − 2 = 1).
   **Engine finding (not fixed here):** a choice the rules grant (which
   digivolution cost to pay) is not surfaced for effect-initiated digivolves.
2. **Slot addressing.** p1 attacked with `field.0` while holding Biyomon AND
   Agumon. `field.N` reaches DCGO as a compact index in frame order and Digimon
   seat centre-out (frames 4, 3, 5, …), so on DCGO `field.0` was Agumon
   (`NOTES-BT25-083.md`). p1 now plays Agumon AFTER the T4 attack.

Dianamon's two same-card triggers after the digivolve follow the
`BT25-028-effect0.yaml` shape (`dcgo_only` MultipleSkills row, shared delete
pick, `sim_only` decline of our `optional: true` [All Turns] gate). Memory note:
Agumon ST1-03 costs 3, not 2 — the draft's ledger was wrong; corrected.

## `inherited#0` — re-authored, and a CARD-DATA finding
The draft reused the trash-digivolve line and then ran `attack: field.0` on a
two-Digimon board (Dianamon + Agumon) — the same slot artefact. Re-authored so
p0 only ever has ONE Digimon: Crescemon is hard-played, Dianamon digivolves
onto it from hand (shared `value: 3` cost row — ours asks the distinct-cost
EffectChoice on a hand digivolve), and the witness is a `<Blocker>` (Grizzlymon
ST2-07) that cannot block on T7.

**F-DATA-BT25-026-INHERITED-TEXT (our bug, card data — measured, not fixed).**
`data/cards.json` carries BT25-026's `inherited_effect_description_eng` as
`<Security A. +1> (…)`. `card_data.rs` parses printed keywords out of that
text, so on our side Dianamon-on-Crescemon checks TWO security cards
(`p1.security` 3, a second card in `p1.trash`). The card image, the official
bundle (`data/card_bundles/BT25-026.md`: "[Your Turn] This Digimon's attack
target can't change.") and `BT25_026.cs` (`CanNotSwitchAttackTargetClass`,
inherited) all agree there is no Security A. Measured: re-running the scenario
against a scratch copy of `cards.json` with the inherited text corrected gives
`p1.security: 4`, `p1.trash: [ST1-02, ST1-03]`, all asserts green.
`data/cards.json` was NOT edited (exam stage; the fix needs the card-fix gate:
an override in `data/card_overrides.json` + a failing-then-passing test).
The scenario therefore leaves `p1.security` / `p1.trash` unpinned. **Expected
oracle outcome until the data is fixed:** a one-field `p1.security` divergence
on the final row (ours 3, DCGO 4) — triage as OUR BUG (card data), not a clause
finding; the no-block-window witness is unaffected.
