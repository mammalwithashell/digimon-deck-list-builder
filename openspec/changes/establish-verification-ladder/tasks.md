# Tasks: Establish the engine verification ladder

## 1. Replay foundation (determinism made first-class)

- [ ] 1.1 `Game::verification_digest()`: canonical gameplay-state hash (zones in stable order, memory, turn state, modifier summary, pending-selection kind); micro-benchmark and iteration-order audit
- [ ] 1.2 Canonical recording format (`qa/replay-goldens/*.replay.json`): schema versions, cards.json content hash, deck refs, seed, per-seat action ids, digest stream; emit hook on HeadlessRunner/DebugRunner games
- [ ] 1.3 Replay runner (extend the ReplaySession core): per-step mask-legality assertion → apply → digest compare; first-divergence report (game, step, type, delta) then stop that game
- [ ] 1.4 Determinism guard test: one recording replayed twice in-process + once cross-process → byte-identical digests; source lint restricting `from_entropy`/`thread_rng` to the two documented sites
- [ ] 1.5 Corpus v1: converter for `runs/` training recordings + generator for seeded greedy-vs-greedy over implemented meta decks from `data/deck_library.json`; size to the tier-2 budget
- [ ] 1.6 `--bless` workflow: regenerate digests for intended changes, replace/retire unreconstructible games with reasons; document the review convention

## 2. Impact map

- [ ] 2.1 Verb/predicate→cards index emitted from the pack build (or sibling tool over all YAMLs)
- [ ] 2.2 Engine lowering-file→verbs map + coverage check (new verb without a map entry fails tier 1)
- [ ] 2.3 `code/tools/impact_scope.py`: git diff → filter string + side-binary list; conservative escalation for unmapped/core files ("full suite required")
- [ ] 2.4 Wire into agent briefs and the authoring merge tool as the default scoped-test source

## 3. Conformance suites

- [ ] 3.1 Keyword semantics matrix: checked-in table derived from `docs/digimon-rules/keyword-semantics.md` + data-driven suite instantiating every keyword on synthetic cards; completeness check (Keyword variant without a row fails)
- [ ] 3.2 FAQ conformance generator: curated Q&A entries → DebugRunner scenarios citing ruling text; seed with existing bundle Q&A (store-champs sets) + mirrored EX12 rulings
- [ ] 3.3 Promote judge_quiz into the always-run tier-1 set

## 4. Invariant fuzz

- [ ] 4.1 Seeded random-policy fuzz driver over implemented-pool decks: no-panic, mask-resolvability, no stranded selections, bounds, clone-then-resolve digest equivalence; seed-only reproduction
- [ ] 4.2 Tune N to the tier-2 budget; wire a one-command repro path (seed+step → debug session)

## 5. Ladder assembly + CI

- [ ] 5.1 `scripts/verify` with `--tier 0..3` (correct env: stack size, thread caps, nextest where adopted); single summary output
- [ ] 5.2 CI: tier-1 gate on engine-affecting PRs; tier-2 gate (or nightly, per measured runtime) — ends the not-in-CI binary rot; tier-3 nightly seal
- [ ] 5.3 Document the ladder in `docs/RUST_ENGINE_API.md` (§ verification) + update agent skill briefs to require tiers 0–2 before commit

## 6. Suite runtime hardening

- [ ] 6.1 Adopt cargo-nextest for `cards_behavioral` + `dsl` (per-test process isolation kills the stack-overflow flake; encode RUST_MIN_STACK in nextest config); run dual-harness for one cycle before switching CI
- [ ] 6.2 Measure link/build times post-nextest; decide (and if warranted, execute) the per-set behavioral-binary split as a follow-up

## 7. Validation

- [ ] 7.1 Dry-run the ladder against a real engine change (the EX12 Guard/Engage keyword round is the intended first consumer): tier 1 catches keyword-matrix effects, tier 2 goldens prove blast radius, and the full-seal comparison confirms no under-scoping
