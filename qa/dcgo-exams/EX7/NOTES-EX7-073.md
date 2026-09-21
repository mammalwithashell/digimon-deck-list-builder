# NOTES — BeelStarmon (X Antibody) (EX7-073)

Denominator: **3 clauses** (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-073`).

| Clause | Scenario | Status |
|---|---|---|
| `EX7-073#effect#0` | `EX7-073-effect0.yaml` | authored, lowers sim-only, asserts pass (re-lowered 2026-09-18) |
| `EX7-073#effect#1` | `EX7-073-effect1.yaml` | authored, lowers sim-only, asserts pass (re-lowered 2026-09-18) |
| `EX7-073#effect#2` | — | **unreachable** (see below) |

## `EX7-073#effect#2` — unreachable

Printed text: *"[When Digivolving] [When Attacking] By trashing 2 cards with
the [Three Musketeers] trait from this Digimon's digivolution cards, delete 1
of your opponent's Digimon with the highest level, and trash their top security
card."*

**Measured reason (exam tooling, not the game).** The clause is reachable in
both engines — a full line was authored and lowered up to the payment step —
but the cost prompt cannot be ANSWERED by the scenario format:

- Our engine parks the payment as `SelectionKind::SourceMulti { min: 0, max: 2 }`
  (the YAML's `select_own_sources`), whose valid ids are `SOURCE_SELECT` slots
  (observed at the T9 attack: `[2000, 2004, 62]` = sources 0 and 4 of field
  slot 0, plus PASS).
- `code/digimon-engine/src/runners/selection_resolve.rs` resolves identity
  picks (`cards:`) only for `Hand / Trash / Reveal / Material /
  CountCappedMultiSelect / Security`; `SourceMulti` falls through the
  `_ => None` arm, so `select: { cards: [BT25-085, BT25-085] }` fails with
  `card pick 'BT25-085' not found in SourceMulti {...} (valid [2000, 2004, 62])`.
- `targets:` resolves battle-area FRAMES, not source sub-slots, and
  `value:` is lowered to the `count` payload (a SelectCountEffect VALUE that
  needs `effect_choices`/`candidates`), never to the raw-index `int_value`
  path — so neither alternative form can name a source either.
- The decline path IS scriptable (a DCGO-only OptionalSkill "no" while our
  engine resolves the unpayable single-candidate pick silently) and is
  exercised at the T7 digivolve in `EX7-073-effect0.yaml`, but a declined gate
  never runs the clause body and measures nothing about it.

Closing this needs a `SourceMulti` arm in `selection_resolve.rs` mirroring the
`Material` arm (decode each valid id with `decode_source_select`, map to the
carrier's source card ids, `find_id` by occurrence). That is harness/engine
tooling outside this authoring stage's remit (scenario/NOTES/book files only)
— proposed gap id `G-TOOLING-EXAM-SOURCEMULTI-IDENTITY-PICK` for
`docs/RUST_ENGINE_GAPS.md`, alongside `G-TOOLING-EXAM-PROBE-NO-ORACLE-MODE`.

### The line, ready to revive once the resolver can answer it

Reuse `EX7-073-effect0.yaml` through its T7 digivolve (EX7-073 over BeelStarmon
BT25-085 = one [Three Musketeers]-trait source), then:

1. T7: `attack: { attacker: field.0, target: security }`. Two [When Attacking]
   triggers stack (BlackGatomon's inherited — BeelStarmon #2 is in hand — and
   this clause, offered because DCGO's CanActivateCondition is `>= 1` trait
   source). `select: { cards: [EX7-073] }` `expect: MultipleSkills, count: 2`;
   then the clause's vacuous gate `select: { decline: true, dcgo_only: true }`
   `expect: OptionalSkill` (our engine resolves the unpayable pick silently);
   then the inherited: `select: { yes: true }` `expect: OptionalSkill`,
   `select: { cards: [BT25-085] }` `expect: SelectHandEffect` (P0's trash is
   empty, so DCGO's hand/trash `generic_bool` is auto-set, no row). Pass.
2. T8: P1 pass.
3. T9: breeding pass; Asuna's trigger as two rows (`decline, sim_only` for
   our gate + `decline, dcgo_only` `expect: SelectHandEffect`); attack. Only
   the clause triggers (no trait card left in hand, so the inherited is not
   offered on either side). Payment: DCGO OptionalSkill(yes) + SelectCardEffect
   over the digivolution cards (exactly 2, canEndNotMax: false) — one row
   `select: { cards: [BT25-085, BT25-085] }` `expect: OptionalSkill` (the
   fold) once `SourceMulti` resolves by identity; then the delete of Agumon
   ST1-03 as a SHARED `select: { targets: [opp.field.0] }`
   `expect: SelectPermanentEffect` row (corrected 2026-09-18: the earlier draft
   of this note made it `sim_only`, reasoning that DCGO force-picks a lone
   mandatory candidate via `canSelectCount == _maxCount -> EndSelect_RPC`. That
   short-circuit lives inside SelectPermanentEffect's human-UI branch
   (`_selectPlayer.isYou && !HarnessAuto.DrivesLocalSeat`); under the harness
   the AI branch runs and calls SetTargetFrames -> InputDriver.TryAnswerStep,
   so a single-candidate pick still consumes a wire row -- the reading
   `EX7-073-effect1.yaml` and `../P/P-180-effect1.yaml` already use); the security trash asks
   nothing. Trailing pass.
   Expected end state: `p0.trash: [BT25-085, BT25-085]`, `p1.field: []`,
   `p1.security: 2` (5 − T7 check − clause trash − T9 check),
   `p1.trash: [ST1-02, ST1-02, ST1-02, ST1-03]`.

Verdict to record: `unreachable` — "cost prompt is a SourceMulti selection the
exam resolver cannot answer by identity (G-TOOLING-EXAM-SOURCEMULTI-IDENTITY-PICK)".

## Re-audit 2026-09-18 (resumed campaign stage)

- DCGO script `EX7/Purple/EX7_073.cs` exists — the card is not `unavailable`.
  No verdict is stored yet (`qa/qa-reports/exam-verdicts/EX7-073.json` does not
  exist): `effect#0` / `effect#1` are `unmeasured` until the oracle pass,
  `effect#2` is `unreachable` for the tooling reason above.
- The `SourceMulti` limit was re-checked against the rebuilt harness
  (`d5f23792f`): `selection_resolve.rs` still has no identity arm for
  `SelectionKind::SourceMulti` (it appears only in the "multiselect-ish"
  PASS-handling match), so `effect#2` stays `unreachable`. The
  single-accept-id `yes:` path that answers the ONE-candidate source picks in
  the P-180 / EX7-070 / EX7-071 lines does not help here: the payment is
  exactly two cards, so the first pick always has two accept ids.
- Neither committed line digivolves INTO BeelStarmon BT25-085 from a Lv.5:
  both ride BlackGatomon's (Lv.4) `[All Turns]` grant, the only open route, so
  BT25-085's later-added "Lv.5 w/[Three Musketeers] in text or w/[TS] trait:
  Cost 3" circle (`9aa21ed0f`) opens no second cost prompt on either side, and
  EX7-073 onto BeelStarmon has exactly one open circle (this card's own, cost
  1). Both files lower unchanged.
- `effect#1`: the unused BeelStarmon BT25-085 the first draft left in P0's hand
  (stack[3]) was replaced by Scopemon BT21-071. A dual card in hand is a
  second candidate of THE CLAUSE's own `IsOption && HasText("Three
  Musketeers")` hand scan on whichever engine counts a dual card as an Option;
  the pick is by identity so it could not desynchronize the line, but the
  file's own header promised no Option-faced card in hand and the stack broke
  that promise.
- `effect#0`'s `ordinal: 0` on the `MultipleSkills` row remains the one
  order-dependent answer on this card. It is outcome-neutral by construction
  (branch (a) resolves silently, branch (b) is declined whichever runs first,
  and the wire row sequence `MultipleSkills` → `OptionalSkill` is the same in
  both orders), so an abort there would be a harness index message to read
  DCGO's order from, not a rules finding.

## 2026-09-20 — `effect#2`: the cost prompt IS answerable now

`G-TOOLING-EXAM-SOURCEMULTI-IDENTITY-PICK` is **closed** (close-out job
`three-musketeers-2`). `runners/selection_resolve.rs` gains a
`SelectionKind::SourceMulti` arm: it reads the prompt's own candidate snapshot
off the parked `ResumeFrame::SourceMultiStep` (`candidates:
Vec<(u16, SourceSelectionRef)>` — the very vector
`install_source_multi_resume_step` derives `valid_action_ids` from) and
resolves each `SourceSelectionRef`'s `CardHandle` against the live carrier,
mirroring `run_source_multi_step`'s DCGO-parity revalidation.

Unlike the `Material` arm this one cannot use the `(ids, range_start)` shape:
`select_own_sources` sweeps EVERY permanent of the owner and keeps only the
sources its predicate passes, so the accepted ids are SPARSE and may span
several field slots. The arm therefore matches explicit `(action id, card id)`
pairs. Duplicate ids resolve in candidate order, and the installer drops a
picked card from the recomputed candidates, so `cards: [X, X]` takes two
different sources. A prompt with no data frame still fails loudly rather than
guessing. Tests: `source_multi_*` in `runners/selection_resolve.rs` — five of
the seven fail before the arm with exactly the error this file recorded
(`card pick 'SRC-B' not found in SourceMulti { min: 0, max: 2, picked: 0 }
prompt … (zone owner 0, valid [2000, 2001, 62])`).

So `select: { cards: [<trait source>, <trait source>] }` now answers the
payment. The clause is **authorable**; this stage did not author it, so it
stays `unreachable` only until a line is written — and the reason is no longer
"the format cannot express it".

### A SHORTER route than the T9 attack line above

The printed timing is `[When Digivolving] [When Attacking]`, and the
**digivolving** half needs no extra turn. Measured facts that make it work off
the `EX7-073-effect0.yaml` prefix:

- The two `[Three Musketeers]`-TRAIT cards this pool can get under EX7-073 are
  **BeelStarmon BT25-085** (`Wizard/Three Musketeers/Iliad/TS`) and
  **Bind Red Trigger P-180** (`Three Musketeers`) — checked against
  `data/card_bundles/`. BT25-082 / BT25-078 / BT25-083 carry `TS` and `Iliad`,
  which are different traits; EX7-051 carries none.
- `effect0`'s line already puts BT25-085 under EX7-073. Putting **P-180 in P0's
  hand** (the `effect1` line does it at `stack[13]`, the card BlackGatomon's own
  T5 digivolve draws — after the T5 main phase has started, so Sparrowmon's
  `[Start of Your Main Phase]` gate stays vacuous) makes EX7-073's OTHER
  `[When Digivolving]` clause (#effect#1, "use 1 Option card with [Three
  Musketeers] in its text from your hand without paying the cost") place P-180
  as EX7-073's BOTTOM digivolution card — a SECOND trait source, at the same
  timing, before this clause resolves if the `MultipleSkills` / `TriggerOrder`
  ordinal runs #effect#1 first.
- P1 already has Agumon ST1-03 on the field from T2 and 5 security, so
  "delete 1 … with the highest level, and trash their top security card" has a
  target and a witness without another turn.

**The one thing to check first**: P-180 prints *"When effects trash this card
from digivolution cards, delete 1 of your opponent's 7000 DP or lower Digimon"*,
so paying the cost WITH P-180 fires a second delete that races this clause's
own. Either author the ordering explicitly on both wires, or keep P-180 out of
the payment and find the second trait source elsewhere. That is a rules
question for the next pass, not a tooling one.

### Scenario authored 2026-09-20 (close-out job `three-musketeers-2`) -- PREDICTED DIVERGENCE

`EX7-073-effect2.yaml` is on disk and lowers sim-only clean (27 steps, 13
assertions green). It reads the **[When Digivolving]** half (DCGO registers
the two timings as two independent ActivateClasses -- `#region When
Digivolving` / `#region When Attacking` -- and our YAML mirrors that with
clause 2 / clause 3), and it is the SHORT route this file predicted, not the
T9 attack line.

**The line** is `EX7-073-effect0.yaml`'s with three changes:

1. `stack[15]` -- P0's T7 TURN draw -- is P-180. NOT `stack[13]`/`[14]` (the
   two T5 digivolution draws): P-180 in hand during the T5 grant digivolve
   would give BeelStarmon BT25-085's own "[When Digivolving] ... use 1 [Three
   Musketeers] or [TS] trait Option card from your hand" a candidate and open
   a prompt pair that measures nothing here.
2. P1 hard-plays **Vermilimon BT4-014** (Lv.5, **8000 DP**, printed vanilla)
   on T4 instead of Agumon on T2. This is what disarms the race this file
   flagged as "the one thing to check first": paying the cost trashes P-180,
   whose own "When effects trash this card from digivolution cards, delete 1
   of your opponent's 7000 DP or lower Digimon" would otherwise fire. Its
   `CanActivateCondition` (`P_180.cs:60-66`) needs an opponent Digimon with
   `DP <= 7000`; 8000 > 7000, so the condition is false whether it is checked
   before or after this clause's delete and DCGO opens no widget. The clause's
   own delete is unaffected -- it selects on LEVEL, and Vermilimon is P1's
   only Digimon. Memory: T4 P1 3 -> -2 hands P0 2, the T5 digivolutions run
   2 -> 0 -> -4, and P0 still opens T7 on 3 and pays 1.
3. The T7 rows after the digivolve carry the clause instead of declining it.

The second trait source arrives at the SAME timing from this card's OTHER
clause: `#effect#1` uses P-180 for free, and P-180's `[Main]` tucks it under
EX7-073. `ordinal: 0` on the `MultipleSkills` row runs `#effect#1` first --
the ordinal `EX7-073-effect0.yaml` already carried through a clean oracle run.

**ENGINE FINDING (predicted divergence, measured sim-side before any Unity
time).** DCGO resolves clause 1 to COMPLETION, the used Option's whole
lifecycle included, before clause 2 comes up; clause 2's
`CanActivateCondition` (`EX7_073.cs:179-186`) then counts TWO trait sources
and the cost is payable. OUR engine queues the used Option's DISPOSAL as a
sibling of clause 2 and parks a two-entry `TriggerOrder [EX7-073, P-180]`
(`5aef07fa8`, G-ENGINE-OPTION-TRASH-TURN-PLAYER-ORDER). Neither branch
reproduces DCGO, and both were measured:

| branch | what our engine does |
|---|---|
| `EX7-073` (authored) | clause 2 runs FIRST against the lone source BT25-085 -> the two-card cost is unpayable, the pick auto-resolves with no prompt, no delete and no security trash. P-180 is then placed by clause 1 and stays a source. |
| `P-180` | the disposal is processed first, so P-180 is already in the trash when its own `[Main]`'s "place this card as the bottom digivolution card" runs -- the placement silently no-ops and the card is LOST (`p0.trash: [P-180]`, never a source). Clause 2 is unpayable just the same. |

The authored branch is the one that keeps clause 1's printed outcome intact,
so the oracle diff isolates clause 2. Expected reading: ours `p0.trash: []`,
`p1.field: [BT4-014]`, `p1.security: 4`; DCGO `p0.trash: [BT25-085, P-180]`,
`p1.field: []`, `p1.security: 3`.

**Triage lead for whoever runs the oracle** (do NOT pre-judge it here): the
question is whether an effect created DURING the resolution of one trigger --
the used Option's `[Main]` and its disposal -- may be ordered against a
trigger that was already queued. `general_rule.pdf` 15-4-3 orders
SIMULTANEOUS items, and these are not simultaneous; that reading favours DCGO
and makes this our bug rather than a DCGO quirk. It is a sibling of, but not
the same as, the two Option-trash-order gaps closed earlier in this job.

**The wire is coherent on DCGO regardless**, which is what makes the line
worth running: `MultipleSkills` -> `SelectHandEffect(P-180)` ->
`SelectPermanentEffect(place)` -> `OptionalSkill(accept)` ->
`SelectCardEffect([BT25-085, P-180])` -> `SelectPermanentEffect(delete)`. The
last three rows answer no live prompt on our side and the harness keeps them
for the wire; the clause-2 gate is authored as its own `dcgo_only` accept row
rather than as an `expect: {prompt: OptionalSkill}` fold, because the fold
only splits a LIVE sim prompt and ours has already auto-resolved.

**`effect#2` moves `unreachable` -> `unmeasured`.** Denominator now
**3 clauses: 2 confirmed + 1 authored-unmeasured, 0 unreachable.**
