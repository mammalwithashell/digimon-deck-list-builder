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

---

## Re-verification, 2026-09-21 (job three-musketeers-2, close-out)

Stored verdicts at the start of this stage: `effect#0` and `effect#1`
**confirmed** (`qa/qa-reports/exam-verdicts/BT25-026.json`); `effect#2` and
`inherited#0` still have NO stored row — the oracle pass never reached them.
Both scenarios were re-run and re-audited rather than re-authored:

| Clause | Scenario | Sim-only (2026-09-21) |
|---|---|---|
| `BT25-026#effect#2` | `BT25-026-effect2.yaml` | lowers 26 steps, 7/7 asserts pass — unchanged |
| `BT25-026#inherited#0` | `BT25-026-inherited0.yaml` | lowers 22 steps, 4/4 asserts pass — unchanged |

**Adversarial C# re-read of the rows both files rest on** — all three claims
hold, so neither file needed a repair:

1. *The `dcgo_only` MultipleSkills row over `[BT25-028, BT25-028]` is real.*
   `MultipleSkills.cs:267-273` short-circuits (`if (skillInfos_active.Count == 1)
   { _skillIndex = 0; Activate(true); }`) only at ONE active skill. Dianamon's
   own digivolve makes BOTH its shared `[On Play]/[When Digivolving]` body and
   its `[All Turns]` body active, so the panel does open with two candidates.
2. *Our `sim_only` gate row before the trash-digivolve pick is correct.*
   `BT25_026.cs:126` registers the `[Your Turn]` body
   `SetUpActivateClass(CanActivateCondition, ActivateCoroutine, -1, false, ...)`
   — **isOptional FALSE**, and `SetIsSkippable(true)` on line 127 is not an
   optional gate (`ICardEffect.cs:1203` keys `Activate_Optional` off `IsOptional`
   alone). DCGO asks no yes/no here; the printed "may" is `canNoSelect: isOptional`
   on the card pick inside `DigivolveIntoHandOrTrashCard`.
3. *The prompt ORDER is card-then-cost, as the file has it.*
   `CardEffectCommons.cs` `DigivolveIntoHandOrTrashCard` installs the
   `ChangeDigivolutionCostPlayerEffect` reducer first (no prompt), then raises the
   `SelectCardEffect` over `Root.Trash` (`canNoSelect: isOptional`), and only
   then pays — so DCGO's `SelectCountEffect` cost row follows the card row.
   The file's `dcgo_only` `value: 3` sits in exactly that slot.

`F-DATA-BT25-026-INHERITED-TEXT` (above) is unchanged and still unfixed, so the
expected oracle outcome for `inherited0` is still a one-field `p1.security`
divergence to triage as OUR card-data bug.

---

## Oracle run 2026-09-21 (close-out job `three-musketeers-2`) — both clauses CLOSED

### The 2026-09-20 pass never measured either clause — an AUTHORING defect, not a DCGO one

Both lines aborted with `prompt mismatch: step N expected count 2 but DCGO asked
for count 1` on the `dcgo_only` MultipleSkills row, which the drain stage read as
"DCGO stacks only ONE body on Dianamon's own digivolve". **That reading was
wrong.** `count:` on a scripted row is the number of PICKS THE PROMPT ASKS FOR,
and DCGO's MultipleSkills hook passes the literal `1`:

```csharp
// DCGO/Assets/Scripts/Script/MultipleSkills.cs:597-599
if (!Digimon.Harness.InputDriver.TryAnswerStep(
        playerID, Digimon.Harness.InputDriver.KindMultipleSkills,
        1, __candidateIds, out __step))
```

The stack SIZE is asserted by `candidates:`, and `ScriptedLine.TryTakeStep`
(`ScriptedLine.cs:127-140`) checks `expect_prompt` BEFORE `expect_count` — so the
kind matched, i.e. DCGO really did open a MultipleSkills panel. Every other card's
confirmed scenario (EX12-036, EX12-047, EX7-073, ST19-14, BT25-085) already used
`count: 1` with a two-element candidate list; only the eight BT25-026/028/058
rows were authored `count: 2`. Fixed in all eight, re-emitted, re-drained against
`D:/dcgo-build/scripted-v16`, and **both BT25-026 lines then ran `completed`** —
with the two-candidate `[BT25-028, BT25-028]` multiset passing, which CONFIRMS
claim 1 of the 2026-09-21 adversarial re-read rather than retiring it.

### `effect#2` — **confirmed**
CLEAN diff over 22 of 26 rows (4 sim-only, 2 DCGO intermediate), sidecar
`20260921T045051Z_3ed6c0e1f5b3484fbc00e2890918759f.state.jsonl`. The
`dcgo_only` `value: 3` cost row and the trash-digivolve card row both hold.

### `inherited#0` — **confirmed**, after `F-DATA-BT25-026-INHERITED-TEXT` was FIXED
The first diff reported exactly the predicted one-field divergence:

```
DIVERGED at step 21
  p1.security: ours=3 dcgo=4
  p1.trash:    ours=[ST1-02, ST1-03, ST1-04] dcgo=[ST1-02, ST1-03]
```

— our extra security check, caused by `data/cards.json` carrying a DIFFERENT
card's inherited text (`<Security A. +1>`). The card face, the official Bandai DB
(`data/card_bundles/BT25-026.md`) and `BT25_026.cs`
(`CanNotSwitchAttackTargetClass`, `isInheritedEffect`) all print "[Your Turn]
This Digimon's attack target can't change." **Fixed this stage** in
`data/card_overrides.json` (durable) + `data/cards.json` (live), guarded by
`code/digimon-engine/tests/data_official_parity.rs::official_db_inherited_text_matches_cards_json`
(fails before / passes after). Re-diffing the SAME sidecar
(`20260921T045115Z_bd44ae610bab41afb025096fde1239cc.state.jsonl`) is then CLEAN.

A pool-wide sweep found 15 more gross inherited-text mismatches against the
official mirror (an "Ace Overflow" family: BT19-011/037/050, EX9-013, EX9-020,
LM-025, LM-026, LM-043, P-191; plus wording drift on BT1-060, BT3-027, BT21-059,
EX4-032/033/034). They are listed in the guard's doc comment and left for a
follow-up — only ids whose face has been read belong in its allowlist.

**Denominator: 4 clauses — 4 confirmed, 0 diverged, 0 unreachable, 0 unavailable,
0 unmeasured.**
