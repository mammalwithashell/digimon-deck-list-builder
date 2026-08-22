# Exam `select:` support — design brief

Closes the `needs-selection` gap: clauses whose resolution prompts a selection
become authorable and oracle-drivable. Written after read-only recon of the
existing machinery; the load-bearing discovery is that **most of the hard part
already exists** and this work is mostly plumbing identities through.

## The one principle

**The wire carries identities, never engine-internal indices.** Our selection
action ids are our encodings; DCGO's RPCs want DCGO-internal values
(`ActiveCardList` CardIndex, frame ids). Both engines resolve identities against
their *own* candidate lists — the same philosophy as the job spec's "card IDs,
never deck codes" and `remap_board_slots`' identity matching. Duplicates resolve
in occurrence order, documented as a limitation.

## What already exists (verified by reading, with evidence)

- `digimon_engine::runners::selection_resolve::resolve_next(game, &SelectionRow,
  picks_done) -> Result<Option<u16>, String>` — maps a semantic selection
  payload onto the live `PendingSelection`, one pick at a time:
  - card-identity picks matched against zone card-id lists, first accepted wins
    (occurrence order; `valid_action_ids` shrinks per pick)
  - `FrameTarget { player, frame }` permanent picks; `frame == -1` is the
    attack-the-player sentinel mapping to `SECURITY_TARGET`
  - cancel → PASS on optional prompts, `Err` on mandatory ones
  - multi-select kinds get one trailing PASS, never a loop
  - `Ok(None)` when the engine auto-resolved and never parked a prompt — a row
    to *skip*, not an error
  (`selection_resolve.rs:201+`)
- `StepSpec.selection: Option<SelectionRow>` — the replay driver already
  consumes semantic selection steps this way (`replay.rs` "task 3.5" path,
  ~:1718/:1816). **`ScenarioAdapter` should emit exactly these StepSpecs**, not
  invent a parallel mechanism.
- `SelectionRow` (`dcgo_recording.rs:284`): `prompt`, `targets`, `card_ids`,
  `indexes`, `count`, `candidates`, int/bool payloads, `cancel`.
- DCGO side: every selection RPC already consults
  `InputDriver.TryAnswer(actor, Kind*, count, candidates, out int)` — but one
  int cannot express a multi-pick or an identity, which is the gap.

## Sim side (Rust)

1. **Scenario schema** — enrich `StepAction::Select`:
   ```yaml
   do: { select: { cards: [EX12-020, EX12-020] } }       # identity picks, occurrence order
   do: { select: { targets: [opp.field.0, own.field.1] } } # permanent picks (OUR slots)
   do: { select: { value: 3 } }                            # count/int prompts (the VALUE)
   do: { select: { yes: true } }                           # optional (yes/no) prompts
   do: { select: { decline: true } }                       # cancel an optional prompt
   ```
   Exactly one of these forms per step; validate loudly.
2. **Lowering / `ScenarioAdapter::from_scenario`**:
   - A `select:` step builds a `SelectionRow` and pushes
     `StepSpec { selection: Some(row), action_id: placeholder, .. }`.
   - The lowering loop advances its own game through the selection with
     `resolve_next` + `decode_action` until `Ok(None)`, so later steps lower
     against the post-selection state. Reuse, do not reimplement.
   - `targets:` references resolve at lowering time against OUR live game →
     `(player, slot)` FrameTargets **plus** the targeted permanent's top-card id
     (for the wire; see below).
   - **New invariant**: after any non-select step, if `game.pending_selection`
     is `Some`, the NEXT scenario step MUST be a `select:` — otherwise fail
     lowering with "our engine asks a selection here; the scenario must answer
     it". This is the sim-side mirror of the oracle's prompt-mismatch finding.
   - Sim-side `expect.prompt` on select steps: assert loosely (map the obvious
     `SelectionKind`→DCGO-prompt pairs; leave unmapped kinds unasserted and say
     so in the run output). The strict assertion happens in DCGO.

## Wire (job JSON) + emit-job

3. `HarnessJobStep` grows optional fields (absent = not a selection step):
   - `select_card_ids: string[]` — identity picks. For PERMANENT prompts these
     are the targeted permanents' TOP-CARD ids (our lowering knows them);
     identity matching dissolves the compact-ordering divergence between the
     engines (`GetFieldPermanents` order vs our battle_area order — they
     disagree ROUTINELY, per the recon facts).
   - `select_value: int` (sentinel `int.MinValue` = absent) — count VALUE,
     generic int, attack-target encoding (−1 = player).
   - `select_bool_present / select_bool: bool` — OptionalSkill / generic_bool.
   - `select_cancel: bool` — decline.
   JsonUtility: normalize absent arrays to empty in `HarnessJob.Parse`, keep
   forward-compat (unknown fields ignored).
4. `--emit-job` maps select steps from the SYMBOLIC scenario data (not from
   resolved engine ids), so the job carries identities.

## DCGO side (C#)

5. `InputDriver` gains `TryAnswerStep(actor, kind, count, candidates, out
   HarnessJobStep step)` returning the full step; the existing int overload
   stays for main-phase/breeding callers.
6. Each selection RPC hook maps the step to its own payload, resolving
   identities against ITS candidate list:
   - `SelectHandEffect` / `SelectCardEffect`: `select_card_ids` → matching
     candidates' `CardIndex` ints, occurrence order, each candidate consumed
     once. Unmatched id → **abort the job** (a finding: DCGO does not offer
     what our engine offered).
   - `SelectPermanentEffect`: `select_card_ids` matched against candidates'
     `TopCard.CardID` → `(isTurnPlayer, UnitIndex)` arrays.
   - `SelectCountEffect`: `select_value` (SetCount already takes the value).
   - `OptionalSkill` / `generic_bool`: `select_bool`.
   - `SelectAttackEffect`: card id → candidate slot; `select_value == -1` →
     player; `select_cancel` → decline (−2).
   - `MultipleSkills` / `SelectDigiXros` / `generic_int`: `select_value`.
7. Prompt-kind assertion stays exactly as today (mismatch aborts as a finding).

## Alignment / diff

No differ changes expected: DCGO's recorder increments `_stepIndex` per
selection row and `StateDumper` dumps there; our trace gains one projection per
`select:` step. One scenario select step == one DCGO selection row (a
multi-pick is answered in ONE RPC). The mulligan aligner is untouched.
**Watch empirically** for mid-effect state representation differences at
selection boundaries (e.g. staged zones); do not pre-engineer for them.

## Known limitations to state, not hide

- Two same-identity candidates are distinguished only by occurrence order.
- `indexes`-based payloads and DigiXros material declarations are out of scope
  for this pass (classify affected clauses honestly).
- `generic_int` / `generic_bool` prompts carry no candidate list, so an
  `expect.candidates` on them can never be asserted (existing InputDriver rule:
  absent candidates = NOT MEASURED, fails loudly if asserted).

## Verification ladder

1. Rust unit tests: schema forms, SelectionRow construction, the
   pending-selection-must-be-answered invariant, lowering through a real
   selection (a card whose on-play prompts a pick, e.g. an ST1 option's target).
2. Unity EditMode tests for the step-mapping logic (pure part factored out of
   the hooks, same `Tests~` pattern).
3. End-to-end: ONE scenario with a selection, sim-only clean, then the oracle
   run producing a CLEAN diff — before any Toho fan-out relies on it.
