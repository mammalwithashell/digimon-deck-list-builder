# Proposal: Establish the engine verification ladder

## Why

Today the only trustworthy "did my engine change break anything" signal is the full `cards_behavioral` suite: ~7,400 tests, 50–95 minutes, with a known mid-run stack-overflow flake — so it runs rarely, agents guess at scoped filters by hand, and several test binaries (`cost_hooks`, `alt_path_reachability`, …) run in **no CI gate at all** and rot silently on main. Meanwhile the project already owns the raw material for much faster, layered verification: a seed-deterministic engine with a reset-and-replay contract (a game is fully reconstructible from its seed plus both seats' action logs), the judge-quiz faithfulness corpus, the verified keyword-semantics table, official FAQ/Q&A rulings, historical meta decklists (digilab scrapes), and training-run recordings already encoded in the 2192-action space. This change assembles those assets into a tiered ladder where each tier answers a specific question in seconds-to-minutes, and the full suite becomes a seal rather than the inner loop — landing ahead of the EX12 keyword round (Guard/Engage touch shared replacement/EOT machinery, exactly the changes that need cheap blast-radius proof).

## What Changes

- **Tiered verification entrypoint** (`scripts/verify.(sh|py) --tier N`): tier 0 = static lints/drift gates (~10s, exists); tier 1 = fast semantic canaries (~2min: `dsl` binary, judge quiz, FAQ conformance, keyword matrix, parity guards); tier 2 = change-scoped behavior (~5min: impact-scoped `cards_behavioral` filter, golden-replay diff, invariant fuzz); tier 3 = full seal (nightly/pre-merge: full behavioral suite, archetype-static tests, dcgo-replay corpus). Tiers 1–2 wired into CI, ending the untested-binary rot.
- **Impact map tool**: parse all card YAMLs into a verb→cards index and map engine files→lowering arms→verbs, so `git diff` computes the affected-card test filter plus affected side binaries, replacing hand-grepped consumer lists.
- **Game-replay verification** built on the determinism contract: a compact canonical recording format `{schema version, cards.json hash, deck refs, seed, [(seat, action_id)]}`; a replay runner that re-derives each game asserting per-step **mask legality of the recorded action** and comparing per-step **state digests**, reporting first divergence per game; a CI determinism guard (same recording twice → identical digests); a golden corpus harvested from `runs/` recordings plus generated seeded greedy-vs-greedy games over `deck_library.json` meta decks; a blessing workflow where intended behavior changes re-record goldens and the reviewed digest diff doubles as a behavioral changelog.
- **Rules-conformance suites**: the judge-quiz binary promoted to an always-run canary; an FAQ-conformance suite generated from official Q&A/rulings (card bundles where available, mirrored rulings otherwise); a data-driven **keyword semantics matrix** encoding the verified §16 table (kind, timing, optionality, OPT behavior) for every keyword — including Guard/Engage rows when they land.
- **Invariant fuzz smoke**: N seeded random-policy games over the implemented pool asserting no panics, every masked action resolvable, no stranded pending selections, and clone-then-resolve equivalence.
- **Suite runtime hardening**: adopt cargo-nextest for the behavioral suite (per-test process isolation removes the STATUS_STACK_OVERFLOW flake; partitioning enables CI sharding); evaluate splitting the `cards_behavioral` monolith per set to cut link times.

## Capabilities

### New Capabilities
- `verification-ladder`: the tier definitions, the single entrypoint, CI wiring, and the policy of which tier runs when.
- `impact-scoped-testing`: the diff→verbs→cards→filter impact map and its use by agents and CI.
- `game-replay-verification`: recording format, determinism guard, legality+digest replay checks, golden corpus, blessing workflow.
- `rules-conformance-suites`: judge-quiz canary policy, FAQ-conformance generation, keyword semantics matrix.
- `engine-invariant-fuzz`: the seeded random-game smoke and its invariants.

### Modified Capabilities
<!-- none — existing spec-level behavior is unchanged; this adds verification infrastructure around it -->

## Impact

- **Tools**: `scripts/verify.*` (new), `code/tools/impact_scope.py` (new), replay runner + corpus tooling (new crate or extension of `dcgo-replay`/`ReplaySession` core), FAQ/matrix test generators.
- **Engine**: additive — per-step digest hook (canonical state hash), recording emit/load; no behavior changes. Builds on `Game::reset_for_replay` and the seeded-RNG discipline already in place.
- **Tests/CI**: new tier-1/2 workflow gates; nextest adoption for `cards_behavioral`; keyword matrix + FAQ suites under `code/digimon-engine/tests/`.
- **Data/artifacts**: committed golden corpus (recordings + digest files) under `qa/replay-goldens/`; corpus generation script consuming `data/deck_library.json`.
- **Dependencies**: cargo-nextest (dev/CI only).
