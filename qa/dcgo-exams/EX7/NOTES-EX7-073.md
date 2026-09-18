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
