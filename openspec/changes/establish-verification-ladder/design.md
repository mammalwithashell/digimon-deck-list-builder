# Design: Establish the engine verification ladder

## Context

Current verification reality: the full `cards_behavioral` binary (~7,400 tests) is the only comprehensive signal; it takes 50–95 minutes, carries a known mid-run STATUS_STACK_OVERFLOW flake (mitigated by `RUST_MIN_STACK` + thread caps), and its link step is heavy enough that saturated agent machines can't get any signal. Scoped filters are chosen by hand-grepping consumers. Several binaries run in no CI gate and rot on main (documented memory). Existing assets: `Game::reset_for_replay` + the reset-and-replay contract (`RUST_ENGINE_API.md` §"Reset-and-replay contract"); seeded-RNG discipline (the only `from_entropy` sites are the no-seed fallback and the random policy driver); the 2192-action space in which **every** decision — mulligan, selections, interrupts, concede, BO3 play order — is an action id; `judge_quiz` (43 tests); `docs/digimon-rules/keyword-semantics.md` (verified §16 derivation); official Q&A in card bundles; `data/deck_library.json` meta decks; training recordings in `runs/`; the `dcgo-replay` oracle and `ReplaySession`/`LiveGame` replay core; clone-safety CI.

Because of that action-space completeness plus seeded RNG, `(seed, deck refs, [(seat, action_id)])` fully determines a native game today — the missing piece is packaging, not engine capability.

## Goals / Non-Goals

**Goals:**
- A change-scoped answer to "what did I affect?" in ≤5 minutes with justified coverage.
- Behavioral drift detection that catches cross-card/integration changes unit tests miss, localized to a step number.
- Keep the determinism property true permanently (guarded, not assumed).
- End the not-in-CI binary rot.
- Remove the stack-overflow flake class from the inner loop.

**Non-Goals:**
- Replacing per-card behavioral tests or the review pipeline (the ladder complements them).
- Snapshot-based state restore (reset-and-replay remains the contract; `Game: Clone` work is tracked elsewhere).
- DCGO parity expansion (the existing dcgo-replay funnel stays as-is in tier 3).
- Observation-tensor/parity verification changes (rule 30 territory, untouched).

## Decisions

**D1 — Recording format: seed + per-seat action ids, pinned.**
`qa/replay-goldens/<name>.replay.json`: `{format_version, engine_schema_version, cards_hash (content hash of cards.json), deck_refs (decklist ids or inline lists), seed, actions: [(seat, action_id)], digests: [u64 per step], final_digest}`. Rationale for pins: card-data changes legitimately alter masks (the 826-circle reconciliation would have); a recording must know its world so divergence classifies as engine-change vs data-change. Alternative considered: store full event logs — rejected as the primary format (bulky, derivable); the digest stream is the compact stand-in and the replay runner can dump full event logs on demand for a diverging step.

**D2 — Replay checks: mask-legality first, digest second, first-divergence-only.**
At each step the runner recomputes the action mask and asserts the recorded action is legal (the sharpest cheap detector — any decision-surface change trips it with an exact step), applies it, then compares the state digest. On first divergence: report game, step, check type (legality vs digest), and stop that game (post-divergence actions are meaningless in the new world). The per-step digest is a hash over a canonical state serialization (zones by stable order, memory, turn state, modifier registry summary) — implemented as a `Game::verification_digest()` that explicitly avoids iteration-order-nondeterministic containers.

**D3 — Determinism is guarded, not assumed.**
CI tier-1 test: replay one corpus recording twice in-process and once cross-process; digests must be byte-identical. This converts "the engine happens to be deterministic" into a maintained invariant (catches future `HashMap` iteration leaks, stray `thread_rng`, or parallelism creeping into resolution). Additionally a grep-level lint: `from_entropy`/`thread_rng` allowed only in the two known driver sites.

**D4 — Corpus: harvested + generated, both from existing assets.**
(a) Harvest: convert a sample of `runs/` training recordings (already action-id sequences) into the format. (b) Generate: seeded greedy-vs-greedy over `deck_library.json` meta decklists filtered to implemented cards (the archetype-static-tests coverage gate already computes implementability) — this is what makes "historical meta decks and key cards" a living regression asset. Corpus size target: enough for <2 minutes tier-2 wall time (~50–100 games); rotate/extend as sets land (EX12 games join once slices ship).

**D5 — Blessing workflow.**
Intended behavior changes re-run the corpus with `--bless`: the runner regenerates digests, and for games whose actions became illegal it re-plays the *policy* (greedy) to produce a replacement game, marking the old one retired-with-reason. The committed diff (digest deltas + retirements) is reviewed like code — a behavioral changelog per engine change.

**D6 — Impact map from the DSL registry, not heuristics.**
The pack build already parses every YAML; extend it (or a sibling tool) to emit `verb→[cards]` and `predicate→[cards]` indexes. The engine side is a maintained table `lowering file/arm → verbs` (checked by the existing eval-arm coverage test pattern so it can't drift silently). `impact_scope.py`: diff → touched files → verbs (plus raw-engine fallbacks: touched non-lowering engine files map to "run tier-2 fuzz + goldens + named side binaries") → emits the `cards_behavioral` filter string + binary list. When the diff touches core files with no mapping (game.rs, combat.rs), the tool honestly answers "full suite" rather than under-scoping.

**D7 — Conformance suites are generated, versioned, and fast.**
Keyword matrix: a data-driven test consuming a checked-in table derived from `keyword-semantics.md` (kind, timing, optional/mandatory, OPT semantics) instantiating each keyword on synthetic cards — one row per keyword including new ones (Guard/Engage land with rows). FAQ conformance: a generator that turns curated Q&A entries (bundle sections; mirrored rulings for sets without bundles) into DebugRunner scenarios — curation is manual-approve (a Q&A entry becomes a test only when an author marks it scenario-izable), so the suite grows deliberately rather than by scraping noise. Judge quiz stays as-is but moves into tier 1's always-run set.

**D8 — nextest for the behavioral suite; per-set split deferred.**
cargo-nextest runs each test in its own process: the stack-overflow flake stops killing whole runs, retries are per-test policy, and partitioning gives CI sharding. Adopt it for `cards_behavioral` + `dsl` first (CI and `verify.sh`), keep plain `cargo test` working locally. Splitting the monolith per set is a bigger refactor (shared fixtures/mod tree) — measure link-time pain after nextest lands before committing to it.

## Risks / Trade-offs

- **[Digest over-sensitivity (hashing incidental state) → noisy divergences]** → digest covers gameplay-meaningful state only (zones, memory, turn, modifiers, pending-selection kind), explicitly excluding caches/counters; tune on the corpus before CI-gating.
- **[Impact map under-scopes and green-lights a breaking change]** → conservative fallbacks (unmapped file ⇒ full suite; core files ⇒ tier 3), plus tier 2 always includes goldens+fuzz which are change-location-agnostic; nightly tier 3 remains the backstop.
- **[Golden corpus goes stale as decks/cards evolve]** → corpus generation is scripted and re-runnable; the coverage gate ties deck selection to implemented cards; blessing retires unreconstructible games explicitly.
- **[FAQ generation produces wrong scenarios from ambiguous rulings]** → manual-approve curation; each generated test cites its Q&A verbatim so review checks the reading.
- **[nextest changes test semantics (per-process env, RUST_MIN_STACK)]** → encode the env in nextest config; run both harnesses in parallel for one release before switching CI.

## Migration Plan

Additive throughout. Order: digest hook + recording format + runner (usable standalone) → determinism guard + corpus v1 → impact map → verify.sh tiers + CI wiring → nextest adoption → conformance suites (matrix first, FAQ generator second). No rollback complexity: each piece is a new tool/test; CI gates are added one at a time after a week of green nightly runs.

## Open Questions

- Digest algorithm/serialization choice (stable hash over a canonical bincode of a trimmed state struct vs incremental zobrist) — decide during the digest-hook task with a micro-benchmark.
- Whether tier-2 goldens run on PRs or only pre-merge/nightly initially (depends on measured corpus wall time on CI runners).
- Per-set behavioral-binary split: measure post-nextest before deciding.
