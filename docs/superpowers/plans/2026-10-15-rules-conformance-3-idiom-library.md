# Rules Conformance 3 — DSL Idiom Library Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `idiom:` clauses to the YAML DSL so repeated standard effects (end-of-turn DNA digivolve, security self-play, Tamer memory floor, …) are defined once in Rust, proven once in the rules-conformance suite, and referenced by name from cards, with a behaviour-preserving migration of existing long-form clauses.

**Architecture:** `digimon-dsl` gains an `idioms` module: each idiom is a Rust function `fn expand(params) -> Result<ClauseSpec, IdiomError>`; the loader accepts `- idiom: <name>` + params and `compile_clause` expands it before lowering, so `CompiledClause` IR and the engine are unchanged. A `dsl-lint --idioms` pass detects long-hand clauses that match an idiom's expansion; `code/tools/rules/migrate_idioms.py` rewrites them only when the compiled IR of both forms is byte-identical. Each idiom has a conformance test module in `tests/rules_conformance/`.

**Tech Stack:** Rust (`digimon-dsl`: `spec.rs`, `clause.rs`, `step.rs`, `compile.rs`, `pretty.rs`, `schema.rs`; `tools/dsl-lint`), Python 3.13 for the migration tool (`code/tools/rules/`), `validate_yaml` example binary for compile checks.

**Spec:** `docs/superpowers/specs/2026-10-15-engine-rules-conformance-validation-design.md` §5c. Depends on Plan 1 Task 6 (`rules_conformance` target + `cite!`).

## Global Constraints

- Idioms are a **closed Rust-defined set**; no YAML anchors/includes/templating. Each idiom appears in the exported JSON schema (`cargo run -p dsl-schema-export`) with its parameters.
- Expansion must produce **exactly** the `ClauseSpec` an author would write long-hand; the compiled IR of `idiom` and long forms must be identical (`assert_eq!` on `CompiledClause`), which is what makes migration safe.
- Every idiom ships with: the Rust expander, a unit test of its expansion, a `rules_conformance` module proving the rules it touches (with `cite!`), and at least one migrated real card.
- The DSL pack version bumps once for the new clause form (see `pack.rs` versioning; follow the existing bump procedure in `docs/RUST_DSL_TEST_API.md`).
- Migration rewrites go through `pretty::format_spec`; a migrated card's `cards_behavioral` tests must stay green with no edits.
- Commits per task; `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`; do not push; per-worktree `CARGO_TARGET_DIR`.

## Review Focus

1. **A card whose long form differs subtly from the idiom (e.g. `optional: false` on the DNA attack tail)** — migrating it would change behaviour. Pinned by Task 4 `test_migration_refuses_when_ir_differs` and the IR-equality gate.
2. **An idiom parameter omitted in YAML silently taking a default the printed text does not say** — pinned by Task 1's `deny_unknown_fields` + required-field tests (`test_idiom_missing_required_param_is_error`).
3. **The `summary`/localisation key changing for migrated cards** (UI shows clause summaries) — pinned by Task 4 `test_migration_preserves_summary_and_scope`.
4. **An idiom proven only in its default parameterisation** — pinned by Task 3's conformance tests covering each parameter the first cards use (`lte: 2/4/0`, `set_to: 3`).
5. **Schema export drift** — pinned by Task 1's `schema.rs` test that every idiom name is present in the exported schema.

---

### Task 1: `idiom` clause form, registry, and the first idiom (`start_of_turn_memory_floor`)

**Files:**
- Create: `code/digimon-dsl/src/idioms/mod.rs`, `code/digimon-dsl/src/idioms/start_of_turn_memory_floor.rs`
- Modify: `code/digimon-dsl/src/clause.rs` (new `ClauseSpec::Idiom(IdiomClause)` variant with `idiom: String`, `params: serde_yml::Value` or typed per-idiom enum, plus the shared `scope`/`summary` fields), `code/digimon-dsl/src/compile.rs` (`compile_clause`: expand `Idiom` → recurse), `code/digimon-dsl/src/schema.rs` (idiom names + params in the schema), `code/digimon-dsl/src/pack.rs` (version bump)
- Test: `code/digimon-dsl/src/idioms/tests.rs` (unit), `code/digimon-engine/tests/dsl/idiom_clause_form.rs`

**Interfaces:**
- Produces: `pub trait Idiom { const NAME: &'static str; type Params: DeserializeOwned + JsonSchema; fn expand(p: Self::Params, common: IdiomCommon) -> Result<ClauseSpec, IdiomError>; }`, `pub struct IdiomCommon { scope: ClauseScope, summary: Option<String>, once_per_turn: bool }`, `pub fn expand_idiom(name: &str, params: Value, common: IdiomCommon) -> Result<ClauseSpec, IdiomError>`, `pub fn idiom_names() -> &'static [&'static str]`.
- YAML form: `- idiom: start_of_turn_memory_floor` / `lte: 2` / `set_to: 3` / `summary: "..."`.

- [ ] **Step 1: Failing tests**

```rust
#[test] fn memory_floor_expands_to_long_form() { assert_eq!(expand_idiom("start_of_turn_memory_floor", yaml!{lte: 2, set_to: 3}, common()), parse_clause(LONG_FORM_YAML)); }
#[test] fn memory_floor_compiles_identically_to_long_form() { assert_eq!(compile_card(IDIOM_CARD).effects[0], compile_card(LONG_CARD).effects[0]); }
#[test] fn test_idiom_missing_required_param_is_error() { assert!(matches!(expand_idiom("start_of_turn_memory_floor", yaml!{lte: 2}, common()), Err(IdiomError::MissingParam{..}))); }
#[test] fn unknown_idiom_name_is_error() {}
#[test] fn every_idiom_is_in_exported_schema() {}
```

- [ ] **Step 2: Run → FAIL (variant/module missing).**
- [ ] **Step 3: Implement** — variant, registry (`match name`), expander producing the exact `TriggeredClause` used by the 30 memory-floor cards (`when: start_of_your_turn`, `condition: {memory_lte: lte}`, `process: [{set_memory: set_to}]`), schema entry, pack bump.
- [ ] **Step 4: Run** — `cargo test -p digimon-dsl`, `cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl idiom_clause_form`, `cargo run -p dsl-schema-export`, `cargo run -p dsl-lint -- code/digimon-engine/cards`.
- [ ] **Step 5: Commit** — `"dsl: idiom clause form + start_of_turn_memory_floor idiom (expands to identical IR)"`.

---

### Task 2: Idioms `security_play_self`, `security_place_self_as_delay`, `delay_gain_memory`, `start_of_main_gain_memory_if_opp_digimon`, `when_attacking_draw_if_hand_lte`

**Files:** one `code/digimon-dsl/src/idioms/<name>.rs` each; registry entries; unit tests as Task 1 (expansion equality + IR equality against one real long-form card each: AD1-019, BT13-110, BT22-099, AD1-019, EX12-020).

- [ ] **Steps 1–5** as Task 1 per idiom, one commit per idiom.

---

### Task 3: Idiom `end_of_turn_dna_digivolve` (unifies the two authoring forms)

**Files:**
- Create: `code/digimon-dsl/src/idioms/end_of_turn_dna_digivolve.rs`, `code/digimon-engine/tests/rules_conformance/idioms_dna.rs`, `fixtures/idioms_dna.yaml`

**Interfaces:**
- Params: `into: PredicateSpec` (hand-card filter, e.g. `{name_contains: "Omnimon Alter-S"}`), `cost: i32` (default none → required), `materials: {left: PredicateSpec, right: PredicateSpec}` (default both `{of: you, kind: digimon}`), `then_may_attack: bool` (AD1-009's tail; default false), `ignore_requirements: bool` (default false).
- Expansion target: the **compact** `may_dna_digivolve_now` step form (BT12-021 family), with the attack tail appended as the existing `optional → select_own_permanent(can_attack) → may_attack_now` sequence when `then_may_attack`.

- [ ] **Step 1: Failing tests** — expansion equality vs BT12-021 (compact) **and** IR equality vs AD1-009 (expanded): if AD1-009's hand-rolled IR is *not* identical to the compact form's IR, that difference is a finding to record in the task's commit message and resolve by reading the printed text (`[End of Your Turn] 2 of your Digimon may DNA digivolve into [Omnimon Alter-S] in the hand. Then, 1 of your Digimon may attack.`) and DCGO `AD1_009.cs`; the idiom follows the printed text.
- [ ] **Step 2: Conformance module `idioms_dna.rs`** — `cite!`-ed tests: `dna_idiom_is_optional_single_declaration` (§15-7-1, §15-9), `dna_idiom_result_is_unsuspended_new_card` (§8-2-2-1-1, -5, -6; expected red until the F3 session lands → `#[ignore]` with its gap id), `dna_idiom_draws_one` (§8-2-3-3), `dna_idiom_fires_when_digivolving` (§15-16-4), `dna_idiom_shares_end_of_turn_pool_with_other_eot_triggers` (§15-4-3-5), `dna_idiom_respects_dna_requirements_unless_ignored` (§8-2-2-2), `dna_idiom_then_may_attack_is_a_separate_optional` (§15-7-2 "Then").
- [ ] **Step 3: Run; triage; commit** — `"dsl: end_of_turn_dna_digivolve idiom + rules_conformance idioms_dna"`.

---

### Task 4: Migration tool and `dsl-lint --idioms`

**Files:**
- Create: `code/tools/rules/migrate_idioms.py`, `code/digimon-dsl/examples/compile_clause_ir.rs` (prints the compiled IR of one clause as canonical JSON, used by the tool for IR equality)
- Modify: `code/tools/dsl-lint/src/main.rs` (`--idioms`: report long-hand clauses whose structural signature equals an idiom expansion with some parameters; print the suggested `idiom:` block)
- Test: `code/tests/tools/test_migrate_idioms.py`, `code/tools/dsl-lint` tests

**Interfaces:**
- `migrate_idioms.py --idiom <name> [--apply] <cards-dir>`: for each candidate, build the idiom clause, compile both via `compile_clause_ir`, require identical JSON, then rewrite via `pretty` (call `validate_yaml`/`format_spec` through the example binary). Output table: card, clause index, `migrated | ir_differs | no_match`.

- [ ] **Step 1: Failing tests** — `test_migration_refuses_when_ir_differs`, `test_migration_preserves_summary_and_scope`, `test_migration_rewrites_identical_ir`, lint test `reports_long_hand_memory_floor_as_idiom_candidate`.
- [ ] **Step 2–4:** implement, run, verify with `--idiom start_of_turn_memory_floor` on the 30 cards (expect 30 migrated), then `cargo test … --test cards_behavioral bt1_085` and the other touched cards' tests green with no test edits.
- [ ] **Step 5: Commit** — `"tools: idiom migration (IR-equality gated) + dsl-lint --idioms"`.

---

### Task 5: Migrate the first idiom families and wire the lint into verify

**Files:** the YAML cards under `code/digimon-engine/cards/**` matched by Tasks 1–3's idioms; `scripts/verify` tier 0 adds `dsl-lint --idioms` as a **report** (non-failing) step; `qa/dsl-vocab-gaps.md` gains an "idiom candidates" section listing shapes ≥3 cards with no idiom yet (from the census).

- [ ] **Step 1:** Run `migrate_idioms.py --apply` per idiom; commit per idiom family with the migrated card list in the message (`"cards: migrate N cards to idiom <name> (IR-identical)"`).
- [ ] **Step 2:** `RUST_MIN_STACK=268435456 cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- --test-threads=8` green; `cargo run -p dsl-lint -- code/digimon-engine/cards` clean.
- [ ] **Step 3:** Commit the verify wiring and the candidates list.

---

## Self-review notes

- Spec §5c covered: clause form (T1), first idioms (T1–T3), one conformance test per idiom (T3 for DNA; T1–T2 idioms get theirs as part of each idiom's module — add a `rules_conformance/idioms_misc.rs` in T2 citing §15-16-11 start-of-turn, §15-16-10 security, §16-17 Delay), IR-equality migration (T4), lint discovery (T4–T5).
- Names consistent: `expand_idiom`, `IdiomCommon`, `idiom_names`, `migrate_idioms.py`, `compile_clause_ir`.
