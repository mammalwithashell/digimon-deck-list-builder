# NOTES — Dianamon (BT25-028)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-028`):
**5 clauses** — `effect#0`, `effect#1`, `effect#2`, `effect#3`, `inherited#0`.
DCGO script: `BT25/Blue/BT25_028.cs` exists — the card is **not** `unavailable`.
**All 5 clauses now have a scenario; none is `unreachable`.** No verdict is
stored yet (`qa/qa-reports/exam-verdicts/` has no `BT25-028.json`): every clause
is `unmeasured` until an oracle run.

Book: `tm_bt25_028_pool.json`. Decks `tm-ts-line` / `tm-ts-opponent` as before,
plus **`tm-ts-ess-line`** added 2026-09-21 — `tm-ts-line` with its two BT4-010
swapped for the two EX9-021 the `inherited#0` line needs. Added as a THIRD deck
rather than by editing `tm-ts-line`, so the three older scenarios ride a deck
that is byte-identical to the one they lowered against.

| Clause | Scenario | Sim-only (2026-09-21) |
|---|---|---|
| `BT25-028#effect#0` `[Digivolve] Lv.5 w/[TS] trait: Cost 3` | `BT25-028-effect0.yaml` | re-verified, unchanged — lowers, 8/8 asserts pass |
| `BT25-028#effect#1` play-cost −5 vs an opposing Lv.6+ | `BT25-028-effect1.yaml` | re-verified, unchanged — lowers, 5/5 asserts pass |
| `BT25-028#effect#2` `[On Play] [When Digivolving]` lock + delete | `BT25-028-effect2.yaml` | re-verified, unchanged — lowers, 5/5 asserts pass |
| `BT25-028#effect#3` `[All Turns] [Once Per Turn]` trash any 4 / DNA | **`BT25-028-effect3.yaml` (NEW)** | lowers, 4/4 asserts pass — carries an engine finding, below |
| `BT25-028#inherited#0` `[When Attacking] [Once Per Turn]` can't suspend | **`BT25-028-inherited0.yaml` (NEW)** | lowers, 2/2 asserts pass |

## Prompt shapes, re-derived from `BT25_028.cs` + the shared DCGO machinery

Three claims the older revision of this file made are now **verified against the
C#**, not predicted:

1. **`SetIsSkippable(true)` is NOT an optional gate.** The gate is decided by
   `IsOptional` alone — `ICardEffect.cs:1203`,
   `Activate_Optional_Effect_Execute`: `if (((ICardEffect)activateICardEffect)
   .IsOptional) { if (isCheckOptional) Activate_Optional(...) }`.
   `IsSkippable` only feeds `MultipleSkills.IsOnlyOptionalEffectStacked`
   (`MultipleSkills.cs:17`) and the panel's `_CanNoSelect`. Both of BT25-028's
   activated bodies are registered `isOptional: false`
   (`SetUpActivateClass(..., 1, false, ...)` at lines 114 and 191), so **DCGO
   never asks an OptionalSkill for this card**.
2. **One active skill ⇒ no MultipleSkills row at all.**
   `MultipleSkills.cs:267-273`: `if (skillInfos_active.Count == 1) {
   _skillIndex = 0; yield return ... Activate(true); }`. The `MultipleSkills`
   rows in `effect0` / `effect1` / `effect2` / `effect3` are therefore legitimate
   ONLY where two of Dianamon's bodies are simultaneously active — i.e. on
   Dianamon's own play/digivolve, where the shared `[On Play]/[When Digivolving]`
   body and the `[All Turns]` body both fire. In `effect3`'s T8 window only the
   `[All Turns]` body is active, so that window correctly carries **no**
   MultipleSkills row.
3. **Declining does not burn the once-per-turn.** `BT25_028.cs:181`,
   `if (!executed) activateClass.RemoveUse();`. Our engine agrees, measured:
   in `BT25-028-inherited0.yaml` p1 plays twice on T6 and our `[All Turns]`
   gate parks **both** times.

### The vacuous optional gate (our side), unchanged and now fully characterised
`BT25-028.yaml` authors the `[All Turns]` body `optional: true`. The printed
"may"s live INSIDE the body (on the trash pick and on the DNA pick), exactly as
DCGO models them (`canNoTrash: true`; `DNADigivolvePermanentsIntoHandOrTrashCard
(..., isOptional: true)`), not on the trigger. So our engine parks a
`Replacement` prompt — `'You may activate BT25-028's triggered effect'`, one
accept id — that DCGO never asks. **Every** Dianamon scenario therefore carries
at least one `sim_only` row for it. Not fixed here (exam stage).

### The DNA half is structurally silent unless p0 holds two battle-area Digimon
`DNADigivolveEffects.cs:456` opens with
`if (owner.GetBattleAreaDigimons().Count < DnaPermanentCount) yield break`
(`DnaPermanentCount = 2`), and only then checks
`owner.HandCards.Some(canSelectDNACardCondition)`. Two independent gates, both
easy to keep shut — which is what makes the trash half separately measurable.

## `effect#3` — F-ENGINE-BT25-028-OPP-SOURCE-PICK-NEVER-OFFERED (**RETRACTED** 2026-09-21)

> **Read the retraction at the end of this file before anything below.** This
> section is the ORIGINAL 2026-09-21 write-up, kept because its reasoning is
> what a reader will otherwise re-derive. Its conclusion is WRONG: the engine
> does offer the pick. The cause was the exam harness spending a trailing
> `PASS` on the freshly parked `SourceMulti`
> (`G-TOOLING-EXAM-TRAILING-PASS-EATS-NEXT-PROMPT`, RESOLVED `b77d1b288`), and
> the clause now diffs CLEAN against the same sidecar.

**Our engine never offers the opponent-source pick, so the clause's whole first
half is a no-op.** Reproduced twice on 2026-09-21 with the committed
`BT25-028-effect3.yaml` board (p1's Garurumon ST2-06 holding
`sources: [ST2-01, ST2-02]`, verified by a probe assert at the gate step):

- the `[All Turns]` trigger fires and our `Replacement` gate parks
  (`valid [59]`);
- answered `yes`, **the very next prompt our engine parks is the DNA anchor** —
  `OwnField prompt 'Choose the first of 2 of your Digimon to DNA digivolve into
  [GraceNovamon]'`;
- no `SourceMulti` prompt is parked at any point, so `trash_selected_sources`
  runs over an empty pick list and nothing is trashed.

Reproduced both when the trigger is the OPPONENT's digivolve on the opponent's
turn (the committed line) and when it is p0's own play on p0's turn (a probe
that hard-plays BT3-007 on T9 instead). The controller is right in both — the
DNA prompt reports `zone owner 0` with p0's own slots — so this is not a
turn-player / controller mix-up.

It is **not** the vocabulary: `select_opponent_sources` with no `filter:` and
the same `then: trash_selected_sources` shape works in `EX12-035.yaml`, guarded
by the green `ex12_035_on_play_bottom_decks_opponent_with_lte_source_count`
DebugRunner test. The visible difference is BT25-028's `min: 0` (EX12-035 uses
`min: 4`) combined with the body running on RESUME after our `optional: true`
gate. `install_source_multi_selection`
(`effect_context/selections.rs:2848-2854`) runs the final callback immediately
and parks nothing when `source_multi_candidates` comes back empty, which is
exactly the observed shape — so the candidate scan is where to look, not the
installer. Isolating filter-vs-`card_sources`-vs-resume needs a DebugRunner
test or the engine MCP, which is outside this stage.

Against DCGO this predicts a divergence the scenario is built to MEASURE rather
than hide: DCGO runs `SelectTrashDigivolutionCards(..., maxCount: 4,
canNoTrash: true, isFromOnly1Permanent: false)` — one `SelectPermanentEffect`
for the carrier then one `SelectCardEffect` over its `DigivolutionCards` — and
ends with `p1.field[0].sources: []`, `p1.trash: [ST2-01, ST2-02]`, while ours
ends with the sources untouched and an empty trash. **`p1.field[0].sources` and
`p1.trash` are deliberately left out of the `assert:` block** so the Unity-free
CI half does not freeze the bug; the differ compares every state field anyway,
so the oracle run still reports it. Same convention as
`BT25-026-inherited0.yaml`'s card-data finding.

### What `effect#3` does and does not pin
Pinned: the `[All Turns]` trigger firing on the OPPONENT's battle-area
digivolve during the OPPONENT's turn (the wording that separates this clause
from a `[Your Turn]` one), and the trash resolution.
NOT pinned: **the cap of 4**. Only two opponent digivolution cards exist at the
trigger, so `maxCount` clamps to 2 on both sides
(`maxDigivolutionDiscardCount = Math.Min(digivolutionCardsSum, maxCount)`).
A 4-source witness needs an opposing Lv.6 stack; `tm-ts-opponent`'s blue line
tops out at Garurumon ST2-06 (Lv.4), and swapping in a red line would put the
stack-building plays in Dianamon's own trigger window. Left as a known limit.
NOT pinned: **the DNA half**. `[GraceNovamon]` (BT25-103 / EX5-073) has no card
spec in our engine — `code/digimon-engine/cards/ex5/EX5-073.json` is ingested
card DATA, there is no YAML for either printing — and EX5-073 prints its own
`[When Digivolving] ... if DNA digivolving, trash any 8 ...`, which DCGO would
run and we would not. A line that completed the DNA digivolve would measure the
missing card, not this clause. **`effect#3` should be read as a partial
measurement even on a CLEAN oracle diff.**

Why the breeding area carries p1's stack: `CanUseCondition` gates on
`IsPermanentExistsOnBattleAreaDigimon`, and
`IsPermanentExistsOnBattleArea` (`GameContextDeterminarion.cs:448`) tests
`GetBattleAreaPermanents()`, which excludes the breeding frame. So p1's T6
breeding digivolve and its T8 move out are both invisible to Dianamon, leaving
the T8 battle-area digivolve as the single activation in the line.

## `inherited#0` — authored 2026-09-21, with a behavioural witness

`BT25_028.cs` region "When Attacking ESS": `SetUpActivateClass(..., 1, false,
...)` — **mandatory, no OptionalSkill** — `SetIsInheritedEffect(true)`, and one
`SelectPermanentEffect` (`canNoSelect: false`, `Mode.Custom`) over
`IsPermanentExistsOnOpponentBattleArea && (IsDigimon || IsTamer)`, guarded by
`HasMatchConditionOpponentsPermanent`. Our engine raises the same single
mandatory pick; the row is SHARED (`Select(cards: ["ST1-02"])`).

**The host.** Dianamon is Lv.6, so the attacker must be a Lv.7. GraceNovamon has
no spec, so the line borrows **Omnimon Alter-S EX9-021** (blue Lv.6 circle, cost
5; its `[DM]`-trait alt route does not apply — Dianamon's traits are Shaman /
Olympos XII / Iliad / TS). EX9-021 has no `[When Attacking]`, so the attack
window holds exactly one trigger: the clause.

EX9-021 is not free, and the line is shaped around its two live effects:
- `[When Digivolving]`: the immunity half is gated on `IsJogress`, but the
  delete is OUTSIDE that gate —
  `if (card.Owner.Enemy.GetBattleAreaDigimons().Count > 0) ... Destroy()` over
  `Filter(IsMaxLevel(permanent, Enemy))` (`EX9_021.cs:183-188`). So it deletes
  every one of p1's highest-LEVEL Digimon, with no selection on either side.
  p1 therefore fields two: Phoenixmon ST1-10 (Lv.6 — the overshoot that funds
  p0's T7, and the one EX9-021 eats) and Biyomon ST1-02 (Lv.3 — the survivor
  the clause picks).
- `[End of Attack]`: `SetUpActivateClass(..., -1, TRUE, ...)` → DCGO asks an
  **OptionalSkill**; ours renders it as a two-branch `EffectChoice`
  `[0: "Play sources" | 1: "Decline"]`, which has no DCGO prompt class. Split
  into a `sim_only` `choice: "Decline"` row and a `dcgo_only` `decline: true`
  row, per the harness's own refusal message for a shared `choice:`.

**The behavioural witness (a negative + positive control, run at authoring time
and NOT committable as steps).** The "can't suspend" modifier is not carried by
the exam's state projection, so the line itself pins the prompt plus the board.
The lock was checked directly against our engine's mask:
- extending the line to T8 and adding
  `attack: { attacker: field.0, target: security }` for the LOCKED Biyomon →
  **`FAILED: step 26: no legal action matches Attack`**;
- extending instead to T10, after "until their turn ends" has expired, with the
  SAME step → the attack **lowers**.
Same board, same Digimon, same slot; only the lock differs. A scenario step can
only assert what IS legal, so the refusal lives here rather than in the file.
(A third control — playing a fresh Digimon on T8 and attacking with it — is
NOT a valid control and was discarded: a Digimon played this turn cannot attack
anyway, so its refusal proves nothing.)

**Incidental observation, not chased.** While running the T10 control our engine
parked EX9-021's `[End of Attack]` `EffectChoice` around an attack declared by
**p1**, not by EX9-021. `EX9_021.cs`'s `CanUseCondition` is
`CanTriggerOnEndAttack(hashtable, card)`. Not reproduced deliberately and not
part of any committed line; worth a look if EX9-021 is ever examined in its own
right.

## Authoring hazards this card hit, for the next author

- **Stack your draws to the END of the line, not to the clause.** p0's stack was
  13 long for a line that reaches T7; the last draw fell through to the seeded
  `rest`, which dealt a SECOND EX9-021 and made
  `digivolve: { using: EX9-021 }` ambiguous (`[400, 490]` — two hand indices,
  one field slot: `decode_digivolve` is `hand * 15 + field`). The fix is more
  stack entries, not a different card. The same trap bit `play: { card: ST1-10 }`
  when a stacked draw duplicated a hand card.
- **`attack:` takes `attacker:` / `target:`, not `from:`.**
- **`assert: { at: N }` is 0-based over the STEP LIST** (dcgo-only and sim-only
  rows included) and reads the state at which step N was taken — a probe assert
  at the digivolve step shows the board BEFORE it.

---

## Oracle run 2026-09-21 (close-out job `three-musketeers-2`) — all 5 clauses MEASURED

### First, the authoring defect that hid every one of them

The 2026-09-20 pass aborted all five lines with
`prompt mismatch: step N expected count 2 but DCGO asked for count 1`. That is
**not** DCGO refusing to stack two bodies. `count:` is the number of PICKS the
prompt asks for, and the MultipleSkills hook passes the literal `1`
(`DCGO/Assets/Scripts/Script/MultipleSkills.cs:597-599`); the stack size is what
`candidates:` asserts, and `ScriptedLine.TryTakeStep` (`ScriptedLine.cs:127-140`)
compares `expect_prompt` first — so the panel DID open. All eight BT25-026/028/058
rows were the only ones in the corpus authored `count: 2`; fixed, re-emitted,
re-drained on `D:/dcgo-build/scripted-v16`, all five lines then `completed` with
the `[BT25-028, BT25-028]` multiset matching. **Claim 2 of "Prompt shapes,
re-derived" above is CONFIRMED, not falsified.**

### Verdicts

| Clause | Verdict | Evidence |
|---|---|---|
| `effect#0` | **confirmed** | CLEAN, 14/16 rows (sidecar `20260921T045136Z_0f07954d…`). DCGO's `action_detail` independently shows the alt-path cost: `cost_paid 3`, memory 3 → 0 |
| `effect#1` | **confirmed** | CLEAN, 13/14 rows (`20260921T045153Z_05752781…`). The cost −5 witness reproduces on DCGO: `cost_paid 7`, memory 5 → −2 |
| `effect#2` | **confirmed** | CLEAN, 14/16 rows (`20260921T045210Z_19e5b7ae…`) |
| `effect#3` | **confirmed** | CLEAN, 21/26 rows (`20260921T045227Z_15e1e879…`), re-diffed 2026-09-21 against the SAME preserved sidecar after the harness fix `b77d1b288` and a re-authored source-pick row. See the retraction below |
| `inherited#0` | **confirmed** | CLEAN, 18/24 rows (`20260921T045251Z_1d972901…`) |

### `effect#3` — F-ENGINE-BT25-028-OPP-SOURCE-PICK-NEVER-OFFERED is **RETRACTED**

**There is no engine bug here.** The finding was a HARNESS artefact:
`G-TOOLING-EXAM-TRAILING-PASS-EATS-NEXT-PROMPT` (docs/RUST_ENGINE_GAPS.md,
RESOLVED `b77d1b288`), the same tooling bug EX7-073#effect#2 surfaced 16 minutes
after this sidecar was recorded.

What the oracle run reported, from the sidecar
`20260921T045227Z_15e1e879de3c4c9baebf0c0cba4cad59`:

```
DIVERGED at step 24 (20 of 26 ours / 23 dcgo)
  p1.trash:             ours=[]                dcgo=[ST2-01, ST2-02]
  p1.field[0].sources:  ours=[ST2-01, ST2-02]  dcgo=[]
```

**Mechanism.** `runners/selection_resolve.rs::resolve_next` emitted ONE trailing
`PASS` after a row's picks were exhausted whenever the parked prompt's KIND looked
like an open multi-pick. Here the exhausted row was the `sim_only` `yes:` that
accepts our `optional: true` gate; accepting the gate runs the clause body, whose
first step installs `SourceMulti { min: 0, max: 4, picked: 0 }` — and `min: 0`
makes `PASS` legal. So the trash pick was **declined by the harness** before any
scenario row reached it, `trash_selected_sources` ran over an empty pick list, and
the next prompt the line saw was the DNA anchor. That is indistinguishable, from
the scenario's side, from "the engine never offers the pick" — which is exactly how
it was read.

**Why the contradiction went unresolved last stage.** The three characterization
tests in `tests/cards_behavioral/bt25/bt25_028.rs`
(`bt25_028_all_turns_offers_opponent_source_pick`, `…_on_opponent_entry`,
`…_on_opponent_digivolve`) were GREEN and each parks the `SourceMulti`. They were
right; the exam line was measuring the harness, not the board. They are kept — they
now double as the regression witnesses for the engine half.

**Re-measurement (zero Unity time).** With `b77d1b288` in and one row added to the
scenario — the shared identity pick
`select: { cards: [ST2-01, ST2-02] }` at DCGO's `SelectCardEffect`
(`G-TOOLING-EXAM-SOURCEMULTI-IDENTITY-PICK`, 8cb317bb4) — the SAME preserved
sidecar diffs:

```
CLEAN (compared 21 of 26 ours / 23 dcgo steps
       (5 sim-only rows with no DCGO prompt + 2 DCGO intermediate rows not comparable))
```

`p1.field[0].sources: []` and `p1.trash: [ST2-01, ST2-02]` — previously left
unpinned so the CI half would not freeze the supposed bug — are now **pinned** in
the `assert:` block, to the DCGO-confirmed values. Sim-only: 8/8 checks pass.

Unchanged caveat: `effect#3` is a PARTIAL measurement even on a clean diff — the
DNA half needs `[GraceNovamon]` (BT25-103 / EX5-073), which has no YAML spec, and
the cap of 4 is not pinned (only two opponent digivolution cards exist at the
trigger, so `maxCount` clamps to 2 on both sides).

**Denominator: 5 clauses — 5 confirmed, 0 diverged, 0 unreachable, 0 unavailable,
0 unmeasured.** (`effect#3` is a partial measurement — see its caveat above.)
