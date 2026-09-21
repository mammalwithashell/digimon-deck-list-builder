# NOTES — BeelStarmon / Fly Bullet (BT25-085)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-085`):
**7 clauses** — `effect#0` … `effect#6`.
DCGO script: `BT25/Purple/BT25_085.cs` exists — the card is **not** `unavailable`.
Book for every authored line: `qa/dcgo-exams/EX7/three_musketeers_pool.json`
(`three-musketeers` vs `quiet-opponent`).

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT25-085#effect#0` | `[Digivolve] Lv.5 w/[Three Musketeers] in text or w/[TS] trait: Cost 3` | `BT25-085-effect0.yaml` | lowers, asserts pass |
| `BT25-085#effect#1` | `<Blocker>` | `BT25-085-effect1.yaml` | lowers, asserts pass |
| `BT25-085#effect#2` | `[When Digivolving] [When Attacking] [Once Per Turn] You may use 1 [Three Musketeers] or [TS] trait Option card from your hand or this Digimon's digivolution cards without paying the cost.` | `BT25-085-effect2.yaml` ([When Digivolving] arm, from hand) | lowers, asserts pass — **carries a DCGO-only row that is a finding** (below) |
| `BT25-085#effect#3` | `[When Digivolving] [When Attacking] [Counter] [Once Per Turn] By trashing 1 Option card … this Digimon unsuspends.` | `BT25-085-effect3.yaml` ([When Attacking] arm) | lowers, asserts pass |
| `BT25-085#effect#4` | `<Use Req. ([Three Musketeers] in text)>` (Option face) | `BT25-085-effect4.yaml` | lowers, asserts pass — **awaiting the oracle** |
| `BT25-085#effect#5` | `[Main] Delete 1 of your opponent's highest level Digimon. Then, you may place 1 [Three Musketeers] trait card …` (Option face) | `BT25-085-effect5.yaml` | lowers, asserts pass — **awaiting the oracle** |
| `BT25-085#effect#6` | `<Arts Digivolve>` (DUAL rule) | `BT25-085-effect6.yaml` | lowers, asserts pass — **awaiting the oracle** |

The three Option-face lines use a **per-card** book,
`qa/dcgo-exams/BT25/tm_bt25_085_pool.json` (decks `tm-dual-085` /
`quiet-red-085`), not the shared `three_musketeers_pool.json`: `ordered_deck`
refuses to stack a card the named deck does not contain, and these lines need a
RED Digi-Egg (ST1-01), a Red Lv.3 with "Three Musketeers" in its text
(EX7-008), a vanilla Purple Lv.5 (BT3-085) and a vanilla Black Lv.3
(BT2-052) — none of which the shared book carries.

> **UPDATE 2026-09-18 (BT25-082#effect#2 triage): the card-data gap below is FIXED for
> BT25-085** — `data/cards.json` / `data/card_overrides.json` now carry `card_kind: 4` +
> a `dual` block (guard: `bt25_085_data_cards_json_is_a_dual_card`). `effect#4/#5/#6`
> are therefore AUTHORABLE now; their `unreachable` verdicts rest on a stale reason and
> should be re-authored from the prompt shapes at the end of this section. The
> `play: { card: BT25-085, from: hand }` collateral lines no longer lower (correct:
> the card cannot be played). The section is kept as the record of what was measured.
>
> **DONE 2026-09-20** — all three are authored (see the closing section). The stale
> `unreachable` verdicts have been RETRACTED in
> `qa/qa-reports/exam-verdicts/BT25-085.json` and now read `unmeasured`, because a
> scenario that lowers is not a measurement: only the oracle pass can move them to
> `confirmed` or `diverged`. **Nothing below this line describes the current engine.**

## HISTORICAL (superseded 2026-09-19 by `20df249e6`, closed out 2026-09-20) — `effect#4` / `effect#5` / `effect#6` were unreachable because `data/cards.json` did not know BT25-085 is a DUAL card

**Measured, not inferred.** `data/cards.json` carries BT25-085 as
`card_kind: 0` (Digimon), `play_cost: 6`, with **no `dual` block**
(`card_overrides.json` patches only its `evo_costs`). The engine takes a card's
kind from that file (`card_data.rs::parse_card_kind`, `4 => CardKind::Dual`) and
the DSL bridge does not overwrite it (`dsl_bridge::enrich_card_data_with_dsl_alt_paths`
merges `ace_overflow`, aliases and DNA costs only). The YAML's `kind: dual` /
`dual:` block therefore never reaches the production `CardData` — the
`cards_behavioral` tests are green because `DebugRunner` builds its `CardData`
from the compiled YAML (`debug_runner.rs` ~1321), which the exam harness
(`--cards-json data/cards.json`), the hosted API, the desktop app and RL
training do not.

What the harness engine actually does with `play: { card: BT25-085, from: hand }`
(probe: Sparrowmon on P0's field, Biyomon on P1's, memory 3):

1. the mask takes the **non-Option** branch (`mask.rs` ~110: `is_option_use` is
   false for `CardKind::Digimon`), so the card is offered as an ordinary
   6-cost **Digimon play** — the printed face says *"Can't play to the field."*;
2. BeelStarmon lands on the battle area at 12000 DP, memory 3 → -3;
3. the Option face's `[Main]` clause fires anyway as part of that play (the
   probe parked `'Delete 1 of your opponent's highest level Digimon'`, and
   Biyomon was deleted);
4. `<Arts Digivolve>` never runs — there is no Option use to replace the trash of.

DCGO models it as a dual card (`CEntity_Base.IsDualCard => cardKind.Count > 1`;
`UseRequirements`, the `OptionSkill` `[Main]` and `ArtsDigivolveEffect` in
`BT25_085.cs`). So no line that is legal on BOTH sides can use Fly Bullet from
hand: ours plays a Digimon that cannot be played, DCGO uses an Option. The
three Option-face clauses are `unreachable` with this reason — never a silent
skip — until the card data is fixed (`card_kind: 4` + a `dual` block shaped like
EX12-018's; `code/tools/ingest_cards.py` already emits one when the API types a
card `Dual`, which it did not for BT25). **Same gap, same symptom, not this
stage's cards:** BT25-043, BT25-057, BT25-104, ST23-09 (all `card_kind: 0` in
`cards.json`, all `kind: dual` in YAML; only EX12-018/-033/-052 are `4`).

**Collateral on other lines (not edited here — not this stage's cards):**
`BT25/BT25-083-effect2.yaml` and `EX7/EX7-066-effect1.yaml` reach a
board-resident BeelStarmon with `play: { card: BT25-085, from: hand }`. That
lowers only because of the gap above; DCGO has no Digimon play for a dual card
(and on an empty board the Option's Use Req. fails too), so those lines will
abort at that row in the oracle. They need the digivolve prefix used here
(LadyDevimon played, BeelStarmon digivolved on top) when re-authored.

Once the data is fixed the three lines are straightforward (prompt shape from
`BT25_085.cs` `#region Option Effects`): Use Req. = an own battle-area/breeding
permanent whose top card `HasText("Three Musketeers")`; `[Main]` = mandatory
`SelectPermanentEffect(Mode.Destroy)` over `IsMaxLevel` opponent Digimon, then —
only if an own Digimon exists AND a [Three Musketeers] TRAIT card is in hand or
trash — a `generic_int` menu {1 hand, 2 trash, 3 "Do not place"} →
`SelectHandEffect` / `SelectCardEffect(Root.Trash)` (canNoSelect) →
`SelectPermanentEffect` (canNoSelect); `<Arts Digivolve>` =
`SelectPermanentEffect("Select 1 Digimon to Arts Digivolve.", canNoSelect: true)`
→ `PlayCardClass(payCost: false, root: Execution, activateETB: true)`, so the
Digimon face's [When Digivolving] stack follows.

## FINDING on `effect#2`/`effect#3` — a triggered clause gated at TRIGGER time is lost when its cost becomes payable mid-stack

Measured at lowering on `BT25-085-effect2.yaml`. Both of BeelStarmon's [When
Digivolving] clauses trigger on the digivolve. At that instant only `#effect#2`
can do anything (P-180 in hand; no Option under any Digimon). After `#effect#2`
resolves, P-180 sits under BeelStarmon and `#effect#3`'s cost is payable.

- **DCGO** asks `#effect#3` then. `additionalActivateCondition` is wired into
  `CanActivateCondition`, not `CanUseCondition`
  (`CardEffectFactory.WhenDigivolvingClass`), so the effect is *stacked* at the
  trigger (`CanTrigger` = max-count + CanUse only), and
  `MultipleSkills.ActivateMultipleSkills_OnePlayer` re-evaluates `CanActivate`
  for every still-stacked effect on each pass of its `while (true)` loop (a
  non-activatable effect is `continue`d, never removed). Second pass: one
  activatable effect → `Activate(true)` → `SelectPermanentEffect("Select a
  Digimon to trash a card from", canNoSelect: true)`.
- **Our engine** parks nothing after the tuck (measured). `BT25-085.yaml`
  mirrors the DCGO gate as a clause-level `condition:`; whether the engine
  drops the trigger when the condition fails at queue time or resolves it as a
  no-op ahead of `#effect#2` without offering an order was NOT separated here
  (no cargo in this stage) -- either way `#effect#3` is never offered once its
  cost has become payable.

`general_rule.pdf` §15-8-3 (trigger-type effects wait as *pending activation*;
whether the "By …" cost can be paid is decided when the effect is activated)
supports DCGO. It matters most on **[When Attacking]**, where it is the deck's
core line: use an Option from hand for free (it tucks itself under BeelStarmon),
then trash that same Option to unsuspend and swing again — with our gate that
second effect is only offered when an Option was ALREADY under a Digimon before
the attack was declared. Same family as the `AdditionalActivateCondition`
gates on LadyDevimon BT25-083 (`NOTES-BT25-083.md`), which asked for a
`condition:` to be ADDED — the right primitive for both is an
*activation-time* gate (queue the trigger, test the condition when it
resolves, park nothing if it fails), not a trigger-time one. Candidate engine /
DSL gap; **not fixed here** (exam stage: triage and report).

`BT25-085-effect2.yaml` answers DCGO's extra prompt with a `dcgo_only` DECLINE
(cancel → `executed` false → `RemoveUse`), so both engines end in the same
state. That is deliberate and is not laundering: the row's *existence* is the
measurement (if DCGO does not ask it, the job aborts on a prompt mismatch).

## Authoring decisions worth knowing before the oracle run

- **The prefix.** LadyDevimon BT25-083 is *played* (7) and BeelStarmon
  digivolves onto it. BeelStarmon and P-180 are both [Three Musketeers] TRAIT
  cards and would wake LadyDevimon's [On Play] tuck if they were in hand when
  she lands, so they are stacked as later draws (T5/T7).
- **The cost row.** From LadyDevimon both BeelStarmon routes are open (printed
  Purple Lv.5 / 4 and the clause's cost 3), so both engines ask — DCGO's
  `SelectCountEffect` "Which digivolution cost do you pay?" over the distinct
  `CostList`, ours the rule-17 `EffectChoice` — answered by VALUE `3` on all
  four lines (`effect#0` measures that answer; the others just use it).
- **`effect#3`'s trigger order.** Two same-card [When Attacking] triggers, no
  keyword on either → `ordinal: 1`. DCGO's order is the `CardEffects()`
  registration order (use-Option, then unsuspend); ours was verified by the
  prompt that follows the pick. `ordinal:` is the non-portable fallback
  (`docs/DCGO_EXAM.md`), so if the oracle aborts on the row after it, read
  DCGO's order off the abort message before touching anything else.
- **`effect#3`'s T9 draw is stacked** (Satellamon). Unstacked, sim-only deals
  Der Blitz EX7-070 there — a [Three Musketeers] Option that silently changes
  `#effect#2`'s candidate set and opens a second `TriggerOrder`.
- **P1's board is empty on the `effect#3` line** so P-180's own "when effects
  trash this card from digivolution cards" delete has no target on either
  engine; with a target the two engines order that trigger differently
  (`NOTES-P-180.md`), which is not this card's clause.
- **Link cards.** `#effect#3` also reads "or link cards"; the authored line
  trashes from digivolution cards. A link-card arm needs an Option *linked* to
  an own Digimon (a Plug-In Option via `link:`) and is a second line, not a
  second clause — not authored.

## The three DUAL clauses: stale reason RETRACTED, scenarios authored (2026-09-20)

`BT25-085#effect#4` / `#effect#5` / `#effect#6` were recorded `unreachable` with
the reason "card-data gap: `data/cards.json` carries BT25-085 as `card_kind 0`
(Digimon) with no `dual` block, so the engine never sees the Option face". That
cause **no longer holds**: commit `20df249e6` (taken during the
`BT25-082#effect#2` triage, cited to `data/card_bundles/BT25-085.md` and
`BT25_085.cs:60 IsTraitedOption => cardSource.IsOption`) rewrote the entry to
`card_kind 4` **with** a `dual` block, and BT25-082/-083 lines then measured
CLEAN through the Option face.

Re-measured 2026-09-20: `play: { card: BT25-085, from: hand }` now lowers as an
**Option USE** in our engine, which is what DCGO does too — `PlayCardClass`
routes a dual card played with no target permanent through
`isDualCardAsOption(cardSource) => cardSource.IsDigimon && cardSource.IsOption
&& !isEvolution` into `UseOptionClass.UseOption()`
(`CardController.cs:1223-1265`), with **no face-selection prompt on either
side**. So all three clauses are authored:

| Clause | Scenario | What the line makes load-bearing |
|---|---|---|
| `#effect#4` | `BT25-085-effect4.yaml` | P0's only permanent is ToyAgumon EX7-008 — a **RED** Lv.3 with "Three Musketeers" in its text. The Option is Purple/Black, so the ordinary colour requirement FAILS and the use is admitted solely by `<Use Req.>`. |
| `#effect#5` | `BT25-085-effect5.yaml` | P1 holds Biyomon ST1-02 (Lv.3) **and** Kokatorimon BT1-014 (Lv.4); only the Lv.4 is deleted and the assertion reads the Lv.3 still standing. The place half is genuinely offered (Der Blitz EX7-070 in hand is a [Three Musketeers] **trait** card). |
| `#effect#6` | `BT25-085-effect6.yaml` | The same use ends with `p0.trash: []` and BeelStarmon standing on SkullMeramon at `sources: [BT3-085]`, memory −3 = 3 − 6 for the use alone: the Option became the Digimon, free. |

**Three things this stage learned that the next author should not re-derive.**

1. **An Option's colour requirement is ALL of its colours, not any** —
   `colorsToCheck.Every(...)` in `CardSource.MatchColorRequirement`
   (`CardSource.cs:305-310`), mirrored by `option_color_match_available`
   (`mask.rs:764-799`, citing `general_rule.pdf` 4-19-3). A first draft of
   `effect6` put a lone PURPLE Lv.5 on the board and the use was **not offered
   at all** — our engine's mask emitted only "Digivolve hand 0 onto slot 0" for
   the card. Hagurumon BT2-052 (Black Lv.3, vanilla) exists on that line purely
   to cover the Black half.
2. **`targets:` lowers to a card IDENTITY**, not a slot — the lowered wire row
   is `SelectWire { card_ids: ["BT1-014"] }`. Distinct card names on every
   permanent therefore neutralise DCGO's centre-out battle-area seating
   (`NOTES-P-180.md`), which is what aborted two r1 lines in job 1. All three
   scenarios use distinct names for that reason.
3. **The exam projection SORTS `sources`, `hand` and `trash`, and sorts `field`
   by `(card_id, dp, suspended, sources)`** (`projection.rs:14-17`). An
   assertion over them witnesses MEMBERSHIP, never position — so
   `effect5`'s `sources: [EX7-070, ST1-01]` does **not** prove the placed card
   went to the BOTTOM, and its header says so explicitly. Do not read a sorted
   list as a positional claim.

**Prompt shapes, re-derived from `BT25_085.cs` `#region Option Effects` before
each scenario was written** (the [Main] is
`SetUpActivateClass(null, ActivateCoroutine, -1, FALSE, …)` → `isOptional`
false → **no `OptionalSkill` yes/no anywhere on any of the three lines**; an
`expect:` row there would desynchronise everything after it):

* `<Use Req.>` = `UseRequirements(card, HasText("Three Musketeers"))` →
  `IgnoreColorConditionClass` whose `CanUse` is "an own **battle-area or
  breeding** permanent that `IsDigimon || IsTamer` and whose `TopCard`
  `HasText(...)`" (`CardEffectFactory.cs:722-748`), consulted from
  `MatchColorRequirement`'s "effects of itself" branch
  (`CardSource.cs:292-300`). It is an OR beside the colour check on both sides,
  never an extra requirement.
* `[Main]` = mandatory `SelectPermanentEffect(Mode.Destroy, canNoSelect:
  false)` over `IsMaxLevel(opponent)` — skipped entirely when the opponent has
  no Digimon; then, only if an own battle-area Digimon exists AND a [Three
  Musketeers] **trait** card is in hand or trash, a `generic_int` menu
  {1 hand, 2 trash, 3 "Do not place"} → `SelectHandEffect` /
  `SelectCardEffect(Root.Trash)` (both `canNoSelect`) → `SelectPermanentEffect`
  (`canNoSelect`) → `AddDigivolutionCardsBottom`. Our engine has no zone menu
  (one `select_union_zone { zones: [hand, trash] }`), so the menu row is
  `dcgo_only`; the card pick and the placement pick are SHARED.
* `<Arts Digivolve>` = `ArtsDigivolveEffect(card)`
  (`KeyWordEffects/ArtsDigivolve.cs:7-58`), an `OptionResolutionClass` that runs
  at DISPOSAL in place of the trash: `CanResolveCondition` needs an own Digimon
  `CanPlayCardTargetFrame` accepts, then
  `SelectPermanentEffect("Select 1 Digimon to Arts Digivolve.", canNoSelect:
  true)` → `PlayCardClass(payCost: false, targetPermanent: …, root: Execution,
  activateETB: true)`. Ours: `dual.option.arts_digivolve: true` →
  `install_arts_digivolve_selection` (`game_actions/mod.rs:986-1110`), an
  optional `OwnField` selection whose decline path is `dispose_option()`.
  **It is offered on EVERY use with a legal target** — which is why `effect4`
  and `effect5` deliberately keep P0's board free of any Lv.5.

**Status.** `--sim-only` is green on all three (13 / 17 / 19 lowered steps,
7 assertion checks each, 0 failed) against
`qa/dcgo-exams/BT25/tm_bt25_085_pool.json`. That is the regression half and
**cannot** find a divergence; the verdicts stay `unmeasured` until an oracle
pass against `D:/dcgo-build/scripted-v16` runs them.

## 2026-09-21 — ORACLE PASS: all three Option-face clauses CONFIRMED

Close-out job `three-musketeers-2`, DCGO `scripted-v16` (`fc67f9ae6`,
action-space digest `711d23bf12`), book
`qa/dcgo-exams/BT25/tm_bt25_085_pool.json`. All three jobs came back
`completed` (full scripted line) and every diff is CLEAN:

| Clause | Sidecar | Diff |
|---|---|---|
| `#effect#4` | `20260921T042353Z_6b34…6cc1` | CLEAN, 13 of 13 ours / 13 dcgo |
| `#effect#5` | `20260921T042419Z_0949…0631` | CLEAN, 16 of 17 ours / 17 dcgo (1 sim-only row + 1 DCGO intermediate row) |
| `#effect#6` | `20260921T042450Z_fed9…b687` | CLEAN, 19 of 19 ours / 19 dcgo |

`#effect#5`'s one uncompared pair is the predicted zone-menu asymmetry this
file already documents: DCGO's `generic_int` {1 hand, 2 trash, 3 do-not-place}
row (`dcgo_only`) against our single `select_union_zone { zones: [hand, trash] }`
prompt, which the harness reports as "kind `UnionZone` has no unambiguous DCGO
prompt mapping". Both wires then take the SAME card pick and the SAME placement
pick, and the end states agree — the asymmetry is the prompt shape, not the
outcome.

**Denominator: 7 clauses — 7 confirmed, 0 diverged, 0 unreachable, 0
unavailable, 0 unmeasured.** The `unmeasured` trio (retracted from a stale
`unreachable` on 2026-09-20 when `20df249e6` restored BT25-085's `card_kind 4`
+ `dual` block in `data/cards.json`) is now measured. The card is closed out.
