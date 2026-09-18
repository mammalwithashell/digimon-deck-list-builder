# NOTES — Satellamon (BT21-074)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT21-074`):
**5 clauses** — `effect#0`, `effect#1`, `effect#2`, `effect#3`, `inherited#0`.
DCGO script: `BT21/Purple/BT21_074.cs` exists — the card is **not** `unavailable`.

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT21-074#effect#0` | `[Digivolve] Lv.4 w/[Three Musketeers] in text: Cost 3` | `BT21-074-effect0.yaml` (book `tm_bt21_074_pool.json`, deck `tm-deputymon`) | lowers, asserts pass |
| `BT21-074#effect#1` | `[On Play] [When Digivolving] By placing 1 [Appmon] or [Three Musketeers] trait card … their effects can't return that Digimon to hands or decks or affect it with <De-Digivolve> effects.` | `BT21-074-effect1.yaml` ([On Play] arm) | lowers, asserts pass |
| `BT21-074#effect#2` | `[When Digivolving] [When Attacking] [Once Per Turn] By trashing 1 card with the [Appmon] or [Three Musketeers] trait from your Digimon's digivolution cards, <De-Digivolve 1> 1 of your opponent's Digimon.` | `BT21-074-effect2.yaml` ([When Attacking] arm) | lowers, asserts pass |
| `BT21-074#effect#3` | `+4000 DP` (the link-box DP grant) | — | **unreachable** (no link verb, below) |
| `BT21-074#inherited#0` | `<Link> [Appmon] trait: Cost 3 … [When Linking] Delete 1 of your opponent's level 4 or lower Digimon.` | — | **unreachable** (no link verb, below) |

## `effect#3` and `inherited#0` — unreachable: the exam has no link verb

Same measured limit as `NOTES-BT21-071.md` (Scopemon's DigiLink half), restated
here so this card's record stands alone:

1. `code/tools/dcgo-harness/src/exam/scenario.rs` fixes the `do:` verbs to
   `hatch, pass, move, play, digivolve, attack, main, select` (the `StepKind`
   enum, ~lines 42-64). There is no `link`.
2. `play:` cannot stand in for a hand-link: `lower.rs::matches_intent` accepts
   it only for `ActionKind::Play`, and our engine exposes no hand-initiated
   *Digimon* link action — link is emitted only on the per-permanent
   `FIELD_EFFECT` range at sub-slot `FIELD_EFFECT_SLOT_FOR_LINK`.
3. `main:` cannot stand in for a field-link: DCGO's
   `Assets/Scripts/Script/Harness/InputDriver.cs` maps the `FIELD_EFFECT`
   range to `ActivatePermanentAction(slot, 0)` and rejects any sub-slot other
   than `FIELD_EFFECT_SLOT_FOR_MAIN`, so a lowered link id aborts the job on
   DCGO's side before any prompt is asked.

`effect#3` is observable only on a host while Satellamon is linked;
`inherited#0` fires only after a link is declared. Neither can be reached, so
both stay `unreachable` with this reason — never a silent skip, never
`confirmed`. DCGO does script the link half (`AddSelfLinkConditionStaticEffect
(HasAppmonTraits, linkCost: 3)`, `LinkEffect`, and a mandatory `WhenLinked`
ActivateClass with a `SelectPermanentEffect(Mode.Destroy)` over opponent Digimon
with `HasLevel && Level <= 4`), so the oracle exists; the wire cannot reach it.

**Update 2026-09-17 — reachable now.** See the same-dated update in
`NOTES-BT21-071.md`: the `link:` verb exists on both sides of the wire
(`from: field.N` or `from: hand.N`, host answered by the next `select:`
step), so `effect#3` and `inherited#0` are `unmeasured`, not `unreachable`,
until their lines are authored. `inherited#0`'s line will need an opponent
Lv.4-or-lower Digimon on the board for the mandatory `WhenLinked` delete pick
(`select_permanent` over `opp.field.N`), and `effect#3` an assertion on the
host's DP after the link.

## Authoring decisions worth knowing before the oracle run

### `effect#0` — base choice and the vacuous-trigger check
- Satellamon's other routes (Purple Lv.4 / 4, Black Lv.4 / 4, `[Sup.]` / 4)
  would all open beside the clause on a purple/black/Sup. base and force the
  digivolution-cost prompt. **Deputymon EX7-010** (red Lv.4, trait `[Mutant]`,
  effect text prints "[Three Musketeers]") opens the clause **alone**, so the
  digivolve is legal on both sides *only* through the clause and no
  `SelectCountEffect` / `EffectChoice` opens. DCGO's `HasText` scans name +
  effect/inherited/security text + attributes/types (`CardSource.cs:2120`),
  so Deputymon qualifies there as it does under our `in_text_contains`.
- Deputymon's "[Your Turn] gains the [Three Musketeers] trait" does **not**
  make it a `[Three Musketeers]`-trait *source* once it is under Satellamon:
  `EX7_010.cs`'s `ChangeTraitsClass` is gated on
  `PermanentOfThisCard().TopCard == card`, and our `kind: aura` grant is a
  top-card overlay. That is what keeps Satellamon's OPT De-Digivolve trigger
  silent after the digivolve (no `[Appmon]`/`[Three Musketeers]` card under
  any own Digimon) and avoids a `MultipleSkills` stack with the mandatory
  tuck effect.

### `effect#1` — what is and is not measured
The scenario measures the **cost half** (the tuck is observable in
`p0.field[0].sources`). The **protection half** ("until your opponent's turn
ends, their effects can't return that Digimon to hands or decks or affect it
with <De-Digivolve> effects") installs modifiers the state projection cannot
see, and the `quiet-opponent` pool (red ST1) holds no bounce / De-Digivolve
effect P1 could aim at Satellamon on T4 — so it is **not probed** by this
line. It is not a prompt-shape risk: both engines resolve the immunity at
De-Digivolve time as a no-op rather than by hiding the target
(`CardController.cs` `IDegeneration.Degeneration`: `if
(_permanent.ImmuneFromDeDigivolve()) yield break;`; ours:
`WhenWouldBeDeDigivolved` cancel in `game_actions/helpers.rs`
`de_digivolve_core`). A future probe needs a per-card book with a black
opponent (e.g. Laser Eye ST5-15) and would measure only that nothing changes.

### `effect#2` — the De-Digivolve count row (affects other cards too)
DCGO's `IDegeneration(target, 1, activateClass)` is constructed **without** a
`DegenerationCountRuling`, so `Degeneration()` opens a `SelectCountEffect`
("How many cards do you trash?") over the single candidate `{1}`.
`SelectCountEffect.Activate()` auto-picks a lone candidate, **but through
`SetCount`, whose harness intercept (`InputDriver.TryAnswerStep`,
`KindSelectCount`) consumes a scripted row** — the same "consumed even with a
single candidate" behaviour `P-180-effect1.yaml` documents for
`SelectPermanentEffect`. Our engine parks nothing for `<De-Digivolve 1>`
(keyword-semantics 16-11: mandatory, no choice at x = 1), so the row is
authored `dcgo_only` with `value: 1` after the target pick.

**Heads-up for the campaign:** the committed `EX7/EX7-070-effect0.yaml` and
`P/P-180-effect0.yaml` attack lines resolve a `<De-Digivolve 1>` through the
same `IDegeneration` path and carry **no** count row. Both already predict an
oracle abort earlier in the line (trigger-timing finding, see their NOTES),
so the missing row is masked today; once that is fixed they will need this
row too. Not edited here — they are not this stage's cards.

### The memory-reset rule (measured while authoring the sister BT25-083 line)
There is **no** start-of-turn memory reset in the rules: `general_rule.pdf`
§6-2-1-1 quotes "[Start of Your Turn] If you have 2 or less memory, set it
to 3" as a *card* effect, DCGO sets 3 only on a pass
(`AutoProcessing.cs::EndTurnProcess`, `Passed && Main`), and our engine
agrees (an overshoot to -1 leaves the opponent's turn starting at exactly 1).
Lines here therefore only ever cross a turn boundary by a pass or by an
overshoot of 3+; `effect#0` gives P0 five memory on T5 by having P1 overspend
on T4 (Biyomon + Garudamon), the `BT21-071-effect1.yaml` convention.
