# Tasks: Release-Set Authoring Workflow

## 1. Set resolution + ingest-diff (Phase 1)

- [x] 1.1 Add a set resolver that maps a set prefix (`BT17`, `EX12`, …) to its distinct card-ID list, collapsing alternate-art printings.
- [x] 1.2 Wrap `code/tools/ingest_cards.py` (`fetch_set_by_card_prefix` / `merge_set_into_cards`) in a diff step: pull `?card=<PREFIX>`, diff IDs and fields against `data/cards.json`, render an added/removed/changed report.
- [x] 1.3 Merge missing/changed cards before authoring; emit a loud fallback warning when `digimoncard.io` is unreachable and continue against the local snapshot.
- [x] 1.4 Tests: resolver on a known set; diff detects a synthetic local-vs-pulled delta (regression-guard the EX12-style drift case).

## 2. DCGO keyword manifest + lexicons (Phase 2 substrate)

- [x] 2.1 Build the DCGO keyword-manifest extractor anchored on the audited canonical surface (see design.md "Fidelity audit"): PRIMARY = union of `CardEffectFactory/KeyWordEffects/*.cs` ∪ `CardEffectCommons/KeyWordEffects/*.cs` (33 keywords; neither dir alone is complete); SECONDARY = `I…Effect` interface names from `CardEffectInterfaces.cs`; TERTIARY = a hand-curated allowlist of core-modeled keywords invisible to directory scanning (`SecurityAttack±`, `DrawX`, `DeDigivolve`, `DigiBurst`). Normalize DCGO→Rust spellings (`Pierce`→`Piercing`, `BlastDigivolution`→`BlastDigivolve`). Emit `data/dcgo_keyword_manifest.json`.
- [x] 2.1b Seed the proactive gap list from the extractor: diff the 33-keyword DCGO registry against the Rust `Keyword` enum and record DCGO-has/Rust-lacks keywords (`Link`, `Ascension`, `Blast DNA Digivolution`) as standing auto-ingest candidates, independent of any single set scan.
- [x] 2.2 Build the complete trait lexicon (every distinct trait across the full card DB) and card-name lexicon; persist as checked-in data.
- [x] 2.3 Document manifest/lexicon refresh in the rule-27 DCGO-rebase checklist.
- [x] 2.4 Tests: extractor produces the known keyword interfaces; lexicon completeness assertion (no per-set sampling).

## 3. Keyword gate (Phase 2 logic)

- [x] 3.1 Implement the bracket-token scanner + set-subtraction (names ∪ traits ∪ timings ∪ grammar ∪ Rust keywords) with numeric-param normalization.
- [x] 3.2 Implement positional trait denoising (followed-by-"trait" rule) and lexicon-miss patching.
- [x] 3.3 Implement the four-way triage (covered / lexicon-miss / auto-ingest / flag-for-human) against the manifest.
- [x] 3.4 Tests: BT17/BT22/BT24/BT25/EX11 fixtures yield the expected residual; `App Fusion`→auto-ingest, `Unchained`→flag.

## 4. Auto-ingest barrier sub-pipeline (Phase 2)

- [ ] 4.1 Define the auto-ingest task contract: port DCGO C# keyword behavior → `Keyword` variant in `enums.rs` + DSL lowering in `digimon-dsl` + wiring in `keyword_effects.rs` + a green DebugRunner test vs DCGO.
- [ ] 4.2 Enforce the barrier: mass-implementation cannot begin while an auto-ingestable keyword the set needs is unimplemented.
- [ ] 4.3 Reclassification path: a "keyword" that is really card-specific behavior routes to the per-card pipeline, not a primitive.
- [ ] 4.4 Action-space hook: a keyword exposing a new player choice triggers rule-27 `ActionSpace.cs` codegen + drift CI.

## 5. Flag-for-human path (Phase 2)

- [x] 5.1 On flag: halt the run, write the gap to `docs/RUST_ENGINE_GAPS.md` + a `.claude/plans/` stub, request context/direction.
- [x] 5.2 Guarantee dependent cards are excluded from mass-implement until the flagged keyword is resolved.

## 6. Multi-signal clusterer (Phase 3)

- [x] 6.1 Implement trait-membership + name-reference-graph connectivity + color/level ordering; emit slices + a labeled orphan-staples bucket.
- [x] 6.2 Name slices: intersect with `deck_library.json` archetypes (inherit canonical name) or synthesize from dominant trait + marquee Digimon for brand-new slices.
- [x] 6.3 Present the partition for user approval before dispatch.
- [x] 6.4 Tests: BT17 fixture recovers the Pulsemon/Eosmon/Argomon/Diaboromon marquee slices; orphans are labeled, not dropped.

## 7. Workflow orchestration (Phases 4–6)

- [x] 7.1 Author the Workflow script sequencing the six phases with the Phase 2 barrier and `pipeline()` fan-out over slices in Phases 4–5.
- [x] 7.2 Phase 4: dispatch each slice (and orphans) to `batch-implement-cards-rust-dsl` in Lv2→Lv7 order.
- [x] 7.3 Phase 5: dispatch each non-orphan slice to `archetype-interaction-test-author`; wire lazy cross-set pull (synthesized fixtures for evo prereqs; single-card pull only on behavioral need).
- [x] 7.4 Phase 6: set coverage gate (all `IMPLEMENTED` + `cargo test` green) + set-level verdict tracker entry + final report.

## 8. Skill/entry wiring + docs

- [x] 8.1 Add the `/author-set <SET>` entry point and register it.
- [x] 8.2 Document the workflow in `docs/` (phase reference, manifest schema, lexicon maintenance) and cross-link from CLAUDE.md card-scripting section.
- [x] 8.3 Dry-run report-only mode (resolve + diff + keyword-gate triage + cluster preview, no agents) for a real set before a full run.

## 9. End-to-end validation

- [x] 9.1 Report-only dry run on a settled set (e.g. BT17) — verify resolution, zero drift, keyword triage, slice partition.
- [ ] 9.2 Full run on a small/partial set (e.g. EX12, which has real drift) end-to-end through the set gate.
- [ ] 9.3 Confirm flywheel: a keyword auto-ingested for one set (e.g. App Fusion) is `COVERED` on the next set that uses it.
