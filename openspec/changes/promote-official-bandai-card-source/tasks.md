## 1. Refresh official data (full-pool)

- [x] 1.1 Build the full Digimon-pool id list from `data/cards.json` and run `code/tools/build_card_bundles.py --ids-file <ids>` to refresh `data/card_official.json` + `data/card_bundles/`. — Full-pool scrape ran but the process was killed at 3175/4220 (session/MCP reconnect). Recovered via a targeted 52-card scrape of the exact reconciliation set (Appmon + guard-violation cards), merged with the prior 438 → `card_official.json` now **490 cards covering every reconciliation target** (0 failures). NOTE: a clean full-pool (~4220) refresh is deferred to a follow-up; it is not required for this fix (the guard proves all DSL cards are covered).
- [x] 1.2 Verify the refresh: record `count`, confirm `failed` empty. — 490 cards, 0 failed; all 82 needed cards present.

## 2. Recover Rule-granted traits/attributes

- [x] 2.1 Parse step scanning `card_official.json` `text_sections` with `\(?Rule\)?\s*Trait:\s*Has\s*\[([^\]]+)\]\s*(Type|attribute)`. — folded into `code/tools/audit_digivolve/reconcile_traits.py` (comprehensive: Rule grants + dropped `form`/Appmon + `(App Name)` cleanup).
- [x] 2.2 Diff recovered grants/divergences against `cards.json`; human-reviewable report. — `--out-report` → 99 corrections + 4 YAML-fix flags (reviewed).
- [x] 2.3 Adjudicate discrepancies vs the card-image mirror. — BT13-095 Marcus Damon confirmed via image (Tamer, no trait line → `Hero` is an authoring error); the other 3 confirmed against the official DB.
- [x] 2.4 **(scope-expanded)** Full guard-violation set reconciled against the official `type`/`form`/`attribute` split — covers Appmon `form` traits (`Stnd./Appmon`) and `(App Name)` cleanup, not just Rule grants. Split (a) cards.json missing a real trait → override; (b) DSL declares a trait the official DB lacks → DSL fix.

## 3. Propagate via overrides

- [x] 3.1 Built full `type_eng`/`form_eng`/`attribute_eng` as the order-preserving union (cards.json ∪ official ∪ Rule grants); wrote changed fields into `data/card_overrides.json` (99 cards: 96 type, 66 form, 2 attribute).
- [x] 3.2 Superset safety check (no dropped traits) built into the tool — e.g. P-215 keeps `Ice-Snow` and gains `Mineral`.
- [x] 3.3 (b)-bucket YAML fixes: BT10-003 `Mini Dragon`→`Minor`, BT10-029 `Lesser`→`Major`, BT13-095 `Hero`→`[]` (Tamer), BT25-070 `Logging`→`Logoff`.
- [x] 3.4 Ran `apply_overrides` to bake into `data/cards.json` (canonical `save_cards_json`; diff is trait fields only, plus one pre-existing AD1-025 override sync).

## 4. Engine guard + optional reconciliation

- [x] 4.1 Guard test `tests/dsl/trait_parity.rs` (production `CardData` traits ⊇ compiled YAML `traits:`), **no allowlist**, large-stack thread (no `RUST_MIN_STACK` dep). Failed pre-fix with the 50 expected violations; **now GREEN** after §3.
- [~] 4.2 (Optional, defense-in-depth) thread `compiled.traits` into `CardData` in `dsl_bridge.rs`. — DEFERRED per design D4 (guard-only first; the data fix already makes production correct, and no DSL-only card needs it). Left as a documented option.

## 5. Verify the reported defect

- [x] 5.1 Confirmed Ice-Snow Rule-granted cards carry `Ice-Snow` in production `CardData` (EX7-021 `[Dragonkin, Ice-Snow]`, P-215 `[Ice-Snow, Mineral]`, EX11-014/EX8-019, …); Appmon cards now carry `Appmon` (BT21-009 form `[Stnd., Appmon]`); `[Free]` recovered for BT16-102/BT17-077.
- [x] 5.2 Production-path coverage: the new guard test verifies the recovered traits on the `cards.json`-built path for **every** DSL card (the data was the only gap; trait_has matching is already unit-tested with Ice-Snow carriers in `ex7_021.rs`). No bespoke per-card digivolve test added — the guard subsumes it.
- [x] 5.3 Suites green: `dsl` (782 passed) incl. the guard, `archetypes` (211 passed) incl. `ice_snow`, and the 4 changed-YAML cards' behavioral tests (16 passed). 0 failures.
- [x] 5.4 Archetype digivolution cost-choice suite `tests/digivolve_cost_choice_archetypes.rs` (18 tests, green): the recovered [Ice-Snow] trait gates a cheaper printed alt-evo, so a base satisfying both routes surfaces the cost choice — Penguinmon {1,0}, Frigimon {3,2}, PolarBearmon {4,3}, Skadimon {4,3} (both EX11/EX8 printings) + negatives. Capstone `ex11_017_skadimon_from_production_cryspaledramon` drives the {4,3} choice off the REAL EX7-021 CardData built from `cards.json` — fails if the trait recovery regresses. Appmon `BT21-023`/`BT21-073` pin the negative contract (same-cost routes collapse → no spurious cost prompt). Verified by an adversarial per-card workflow against the official bundles.

## 6. Promote the source in docs/process

- [x] 6.1 `CLAUDE.md` "Source priority" — added the "Printed card data — the official Bandai DB is authoritative" lane (outranks DCGO + cards.json for printed data) + cross-refs; no renumber.
- [x] 6.2 `digimon-card-lookup` SKILL.md — added the Traits/type/attribute trust bullet + the `reconcile_traits.py` tooling.
- [x] 6.3 Source-priority feedback memory — two-lane rewrite (behavior vs printed-data) + MEMORY.md index hook.

## 7. Record follow-up

- [x] 7.1 Logged the engine attribute-predicate matching gap (`predicate.rs` `attribute_is` always false) to `docs/RUST_ENGINE_GAPS.md` so recovered `[Free]`-attribute grants (BT16-102, BT17-077) become matchable in a later change.
- [ ] 7.2 (follow-up, not blocking) Run a clean full-pool (~4220) `build_card_bundles.py` refresh when convenient, then re-run `reconcile_traits.py` to catch any non-DSL Appmon/Rule-grant cards (the guard only covers DSL-authored cards).
- [x] 7.3 P-215 Icemon's printed alt "[Digivolve] Lv.3 w/[Ice-Snow]/[Mineral]/[Rock] trait: Cost 2" was authored COLOUR-gated (over-permissive cheaper evo) — FIXED: re-authored as three `trait_has`-gated alt_paths (Ice-Snow/Mineral/Rock) + explicit standard cost-3 paths; added 3 cost-choice tests ({3,2} + negatives). New `code/tools/audit_digivolve/audit_alt_evo.py` systematically audits every implemented card's `alt_paths` vs the official `special_digivolution_condition` — on the partial mirror it flagged 7 gate-mismatch (P-215-class) + 32 missing/cost candidates for the broad sweep (7.5).
- [ ] 7.5 (broad sweep, in progress) Full-pool official refresh running; then run `audit_alt_evo.py` pool-wide and a verify-and-fix workflow over the gate-mismatch / missing-alt candidates (most are same-cost gate inaccuracies; some "[X] in text" alts are engine gaps). Triage genuine cost-difference / over-permissive alts (P-215 class) as fixes; record engine-gap alts separately.
- [ ] 7.4 (follow-up, surfaced by the cost-choice workflow's completeness critic) Two UNIMPLEMENTED cards print a genuine distinct-cost digivolution alt that will create a cost choice once authored as YAML `alt_paths`: BT21-074 Satellamon (Appmon: "[Three Musketeers] in text: Cost 3" vs standard 4 → {4,3}) and BT24-026 Hyogamon (Ice-Snow: "Lv.3 w/[Demon]/[TS]: Cost 2" vs standard 3 → {3,2}). When implemented, add their cost-choice tests to `digivolve_cost_choice_archetypes.rs`. (Also noted: BT21-023.yaml encodes only the Red standard route, dropping the Yellow circle — moot here since both are cost 4, but a card-data gap.)
