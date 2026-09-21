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

## F-ENGINE-PLUGIN-MODE-SELECT-WITHOUT-HOST — FIXED 2026-09-20 (`f24b986d0`)
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
Same behaviour applied to every dual-mode Plug-In Option (BT25-093, BT24-091, …).

**Fixed at close-out (`f24b986d0`, three-musketeers-2).** `Game::option_legal_
play_modes` filtered only AFFORDABILITY; it now also drops `OptionPlayMode::Link`
when `link_host_candidates` is empty, so the mask and the mode-select offer agree
and a single surviving mode plays directly with no prompt.
`general_rule.pdf` §10-1-3-1 (p.19) makes "then the player chooses 1 of their
Digimon that meets the requirement" part of the link procedure, and §6-5-1-4
(p.12) is the main-phase action it implements — with no such Digimon the action
cannot be declared. DCGO agrees: `CardEffectFactory.LinkEffect` returns `null`
when `!HasMatchConditionPermanent(CanSelectPermanentCondition)` (`Link.cs:24`,
re-checked at `Link.cs:53`). Tests
`bt25_100_no_mode_select_when_no_link_host_exists` /
`…_when_only_non_ts_digimon_on_board` fail before and pass after; full entry in
`qa/resolved-gaps.md`.

Consequence for THIS scenario: `BT25-091-effect2.yaml` no longer needs (and can
no longer lower) its `sim_only` `choice: "[Main]"` row — our Play bit IS the
`[Main]` use here, exactly like DCGO's. The row was removed in the same commit.

## `effect#2` — RE-MEASURED 2026-09-20: CLEAN, verdict `confirmed`

> **Read the 2026-09-21 "ANSWERED" section at the end of this file before
> trusting this one.** The CLEAN run recorded here was taken on an older shape
> of the line; the current shape needed a harness fix before it re-measured
> clean. The verdict is the same, but the evidence under it was replaced.

The divergence below was measured BEFORE engine fix `5108e58ee` ("a used Option is
trashed before the triggers its `[Main]` caused"; `general_rule.pdf` 9-1-5 + 18-1-2 +
15-4-3-5) landed, and was never re-diffed against it. Re-run against the same
preserved sidecar (`20260918T130112Z_6a17348e0757476cbf19bd6e8b1a8090.state.jsonl`,
zero Unity time) the line is **CLEAN**, 12 of 13 rows compared (1 sim-only), so
Iron Slash is now in `p0.trash` while Monica's stacked trigger resolves, exactly as
DCGO has it. Verdict `diverged → confirmed`;
`G-ENGINE-OPTION-TRASH-VS-ON-USE-TRIGGER-ORDER` is RESOLVED in
`docs/RUST_ENGINE_GAPS.md`.

The ordering *choice* the rules give the Option's user over their OWN pending
triggers (9-1-5 + 18-1-2 + 15-4-3-5-1) was then taken up at close-out and is now
SURFACED by our engine — `G-ENGINE-OPTION-TRASH-TURN-PLAYER-ORDER` RESOLVED
2026-09-20 (`5aef07fa8`). This line is one of only 8 places in the whole behavioural suite that
reach it (Iron Slash's pending trash + Monica's own trigger), so
`BT25-091-effect2.yaml` gained a **`sim_only` `choice:` row** taking DCGO's order
("Trash the used Option"); DCGO asks nothing there, and every compared row is
unchanged.

### Original triage 2026-09-19 (superseded — kept for the record)
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

## ANSWERED 2026-09-21 — the `suspended` diff was a HARNESS double-answer, not an ordering divergence. Verdict `confirmed` re-earned.

The 2026-09-20 note below left one question open: is
`p0.field[0].suspended: ours=true dcgo=false` at the `select: { yes: true }` row a
harness SNAPSHOT-BOUNDARY property (our trace snapshots after the activation cost
is paid, DCGO's before) or a REAL ordering divergence? **Neither.** It was a
harness bug one level up — a decision our engine asked and the harness spent
twice — and `suspended` was only its symptom. The prescribed "teach the differ to
tolerate `suspended` on activation-cost rows" fix would have been actively
harmful: it papers over a row shift that leaves the clause unmeasured.

**What was happening.** `ReplayDriver::auto_answer_option_trash_order`
(`code/digimon-engine/src/runners/replay.rs`) runs at the top of EVERY step and
answers our engine-only 9-1-5 trash-order prompt with entry 0 ("trash now"). That
is right for the DCGO corpus — DCGO makes no such decision, so no recording can
carry an answer. It is wrong here, because this scenario writes that decision
itself as the `sim_only` `choice: "Trash the used Option"` row. Both fired, so:

| scenario row | prompt it was written for | prompt it actually answered |
|---|---|---|
| step 8 `choice: "Trash the used Option"` (sim-only) | our TriggerOrder trash-order pick | **Monica's §15-7-4 decline gate** |
| step 9 `select: { yes: true }` | Monica's §15-7-4 decline gate | the mandatory opponent-Digimon pick |
| step 10 `select: { targets: [opp.field.0] }` | the opponent-Digimon pick | nothing (no live prompt) |

So by step 9 the suspend cost had already auto-paid — hence `suspended: ours=true`
— and the clause's own "you may pay" decision was never exercised at all. The
lowering pass never auto-answered, so it assigned the rows CORRECTLY; only the
replay pass shifted, which is why the line lowered, validated
`expect: { prompt: OptionalSkill }`, and still diffed dirty.

**Our engine was right the whole time.** "by suspending this Tamer" is a §15-7
optional processing condition — `general_rule.pdf` (Ver.3.6) **§15-7-1** (p.24),
"conditions ... include text such as 'by X, Y'", and **§15-7-4** (p.24), "A player
can choose whether or not to execute the content of optional processing
conditions." Our engine parks that decline gate (the `bundle.len() == 1`
`needs_pre_cost_prompt` branch in `effect_queue.rs`) before paying. DCGO parks the
same decision: `BT25_091.cs:104`
`SetUpActivateClass(CanActivateCondition, ActivateCoroutine, -1, **true**, …)` —
the `isOptional` arm — and only then does `ActivateCoroutine` run
`SuspendPermanentsClass` followed by `SelectPermanentEffect`
(`BT25_091.cs:136-144`). The preserved sidecar has exactly that two-row shape:
step 12 `suspended: false` (the yes/no) then step 13 `suspended: true` (the pick).
Two prompts, same order, both engines.

**Fix + re-measure.** `RecordingSource::answers_engine_only_prompts()` (default
`false`, `ScenarioAdapter` → `true`) suppresses the auto-answer for exam
scenarios; the DCGO-corpus path is untouched. Regression test:
`code/tools/dcgo-harness/tests/exam_engine_only_prompt_not_auto_answered.rs`
(fails before with this exact symptom, passes after). Re-diffed against the same
preserved sidecar `20260918T130112Z_6a17348e0757476cbf19bd6e8b1a8090.state.jsonl`
(zero Unity time): **CLEAN**, compared 12 of 13 ours / 12 dcgo steps (1 sim-only
row with no DCGO prompt). Verdict re-recorded `confirmed` 2026-09-21 — now on
evidence the CURRENT engine and the preserved oracle trace both produce. Full
`cards_behavioral`: 8227 passed / 0 failed / 37 ignored.

**The silent second instance existed.** An instrumented sweep of all 369 exam
scenarios found exactly two that reach the engine-only trash-order prompt: this
one and `qa/dcgo-exams/BT3/BT3-096-effect0.yaml`. BT3-096#effect#0 was stored
`confirmed` too, and pre-fix it re-diffs `DIVERGED` with `memory: ours=2 dcgo=1` —
Mimi's memory gain already applied because her "you may suspend this Tamer" gate
had been eaten the same way. It is CLEAN post-fix and its verdict is re-recorded.
Full write-up: `G-TOOLING-EXAM-AUTO-ANSWER-EATS-NEXT-PROMPT` in
`qa/resolved-gaps.md`.

### Original open finding, 2026-09-20 (superseded — kept for the record)

Re-diffing `BT25-091-effect2.yaml` against the preserved sidecar
(`20260918T130112Z_6a17348e0757476cbf19bd6e8b1a8090.state.jsonl`) after the
mode-select row was dropped gives **DIVERGED at the `select: { yes: true }`
row** (12 of 13 ours compared / 12 DCGO, 1 sim-only):

```
p0.field[0].suspended: ours=true dcgo=false
```

It is a ONE-FIELD, ONE-ROW transient: every later compared row agrees, so both
engines have Monica suspended by the next prompt. Ours applies the
"by suspending this Tamer" cost as part of resolving the OptionalSkill accept;
DCGO's state row for that accept is snapshotted before its coroutine pays it.

**This is NOT caused by `f24b986d0`.** Reproduced on the UNMODIFIED engine
(`git show HEAD:code/digimon-engine/src/game_actions/mod.rs` restored, harness
rebuilt) against the pre-fix scenario: identical lead,
`DIVERGED at step 10 … p0.field[0].suspended: ours=true dcgo=false`. It is the
same logical row in both shapes — the row indices differ only because the
sim-only mode-select row is gone.

It appeared with the `G-ENGINE-OPTION-TRASH-TURN-PLAYER-ORDER` close-out
(`5aef07fa8`), which inserted our `TriggerOrder` prompt ahead of Monica's
OptionalSkill: the pre-`5aef07fa8` shape of this line cannot be re-run (the
scenario without the trash-order row now fails to lower — "our engine's parked
TriggerOrder prompt maps to DCGO `MultipleSkills`"), so the CLEAN re-measure
recorded above was taken on the older shape.

The stored verdict for `BT25-091#effect#2` is still `confirmed` (recorded
2026-09-20T20:42). It is **left untouched here** — flipping it needs the
triage this note asks for, not a drive-by edit. Next dispatch should decide
whether the snapshot boundary is a harness property (in which case the exam
differ should tolerate it for activation-cost rows) or a real ordering
divergence, and re-record the verdict either way.
