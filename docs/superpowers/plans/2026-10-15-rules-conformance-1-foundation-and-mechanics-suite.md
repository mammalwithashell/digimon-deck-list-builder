# Rules Conformance 1 — Foundation and Mechanics Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the per-rule obligation denominator for Comprehensive Rules Ver. 4.3 and a chapter-indexed Rust conformance test target whose tests cite fingerprinted rule text, then write the mechanics chapters (effect states, effect categories, optional conditions, rule checks, pending/overwrite processing, attack procedure, keyword lifecycles) discover-then-pin.

**Architecture:** Python tooling under `code/tools/rules/` parses the 4.3 text into rule units and merges a hand-maintained status sidecar into a generated `qa/rules/cr_obligations.json`, with a coverage report and a `--check`. A Rust integration test target `tests/rules_conformance/` embeds that JSON, exposes a `cite!` macro that fails on fingerprint drift, and holds one module per rules chapter written with `DebugRunner` and inline YAML DSL cards. A red test is filed as a gap and `#[ignore]`d with the gap id; it is never weakened.

**Tech Stack:** Python 3.13 (stdlib only; pytest under `code/tests/tools/`), Rust 2021 (`digimon-engine` dev-deps: `serde_json`, `sha2` — check `Cargo.toml`; add `sha2` as a dev-dependency if absent), cargo integration test targets.

**Spec:** `docs/superpowers/specs/2026-10-15-engine-rules-conformance-validation-design.md` (§0 steer, §5 Phases 1–2, §5b gate, §6 decisions). Companion plan: `2026-10-15-rules-conformance-2-mechanics-gate.md` (the training-gate switch; depends on Task 3's artifact).

## Global Constraints

- Rules version is **Ver. 4.3 (2026-09-18)**; cite 4.3 rule numbers. Source text until Phase 0 lands: the vendored `qa/rules/aegis/comprehensive-4.3.md` (Task 1); after Phase 0, switch `--source` default to `docs/digimon-rules/cr-4.3.txt` (one constant).
- Fingerprints are SHA-256 over **normalised** text, steps in this order: whitespace collapsed to single spaces; trailing glued page numbers removed (regex `([a-z.)])\d{1,2}(\s|$)` → `\1\2`); `’“”` folded to ASCII; U+3000 (ideographic space) → space; lower-cased; `(For details, refer to …)` cross-references removed with a regex equivalent to `\(?\s*for details,? refer to [^)\n]*\)?\s*(\d{1,2}(?:-\d+)*)?` (the extraction sometimes drops the opening parenthesis, and the trailing rule number may follow after a space); remaining bare rule numbers (`\b\d{1,2}(-\d+)+\b`) → `#`; **whitespace collapsed again and trimmed** (amended 2026-10-15 after Task 2: without the final collapse, cross-reference removal left double spaces). The parser also drops page-number lines (a line that is a form feed followed only by digits, regex `\f\d{1,3}$`, applied before line splitting; a BARE digit line is kept because it can be a wrapped cross-reference tail, e.g. 4.3 line 1990; amended 2026-10-16 after Task 5 so our own `pdftotext` extractions fingerprint identically to the Aegis one) and skips chapter-heading lines (`^(1[0-8]|[1-9])\.\s+[A-Z]`, e.g. `17. Rule Checks`), which are neither rules nor continuation text. The same `normalise()` lives in Python (`code/tools/rules/parse_rules.py`) and Rust (`tests/rules_conformance/cite.rs`); Task 6 proves they agree.
- Every `#[test]` in `tests/rules_conformance/` opens with at least one `cite!` (meta-test in Task 6). Doc comments quote the rule text being asserted.
- **Discover-then-pin:** a failing test is `#[ignore = "G-ENGINE-<SLUG> (§x-y-z)"]` plus a gap entry in `docs/RUST_ENGINE_GAPS.md` plus `status: gap` in the sidecar. Never assert what the engine does.
- Mechanics set (spec §5b): `effects-and-ordering, keywords, attack-and-timing, security, battle, blocking, rule-checks, turn-flow, digivolution-and-costs, play-and-costs, card-use-and-costs, link`.
- Test target registration mirrors `judge_quiz`: `[[test]] name = "rules_conformance", path = "tests/rules_conformance/main.rs", required-features = ["dsl-yaml-loader"]` (the feature is in `default`, so plain `cargo test --test rules_conformance` runs it).
- Engine behaviour is **not** changed in this plan except where a task says "fix" (none do). Fixes are Plan 3 and the dispatched F2/F3 sessions.
- Commits: one per task, message cites the rule numbers covered, ends with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`. Do not push.
- Run Rust tests with the per-worktree `CARGO_TARGET_DIR` (CLAUDE.md rule 31); never `git stash` (shared stash).

## Review Focus

1. **A fingerprint that passes against the vendored extraction but fails against our own `pdftotext` of the real PDF** — poppler builds differ in hyphenation and page-number placement; a person expects a `cite!` to survive the Phase 0 source switch. Pinned by Task 2 `test_normalise_is_stable_across_extractions` (both scratch extractions of 4.3 §15-4-3-5-1 normalise to the same string) and Task 6's Rust/Python agreement test.
2. **A renumbered rule cited by its old number binding silently to a different rule** (e.g. `16-6` was Piercing in 3.6 and is Draw in 4.3) — the person expects the test to fail loudly. Pinned by Task 6 `cite_with_wrong_fingerprint_panics`.
3. **A sidecar `proven` entry naming a test function that no longer exists** — the coverage report would over-count. Pinned by Task 4 `test_check_fails_when_proven_test_missing`.
4. **A conformance test that asserts the engine's current output instead of the rule** — the whole suite's value depends on this. Pinned structurally by Task 6's meta-test (every test cites) and by the discover-then-pin rule; reviewers read each test's doc comment against the quoted rule.
5. **A chapter module that passes only because the synthetic card's YAML was authored to the engine's dialect rather than the printed phrasing** — e.g. authoring `optional: true` on a mandatory keyword. Pinned by Task 7 onward: every synthetic card's YAML is committed next to the test with the printed-style text in a `text:` field, and the `dsl-lint` run is part of each chapter task's verification step.

---

### Task 1: Vendor the Aegis rules inputs

**Files:**
- Create: `qa/rules/aegis/README.md`, `qa/rules/aegis/LICENSE-MIT.txt`, `qa/rules/aegis/comprehensive-4.3.md`, `qa/rules/aegis/mechanisms-by-rule.json`, `qa/rules/aegis/conformance-scenario-index.md`

**Interfaces:**
- Produces: `qa/rules/aegis/comprehensive-4.3.md` (verbatim copy of `D:/tmp_aegis/a/data/kb/rules/comprehensive.md` @ `ec0cd22f0`), `mechanisms-by-rule.json` = `{ "<rule-id>": ["<mechanism>", ...] }` for all 1,119 rule-kind obligations.

- [ ] **Step 1: Copy the extraction and licence**

```bash
cp D:/tmp_aegis/a/data/kb/rules/comprehensive.md qa/rules/aegis/comprehensive-4.3.md
cp D:/tmp_aegis/a/LICENSE qa/rules/aegis/LICENSE-MIT.txt
```

- [ ] **Step 2: Extract the per-rule mechanism map**

Write a one-off in the scratchpad (not committed) that loads `D:/tmp_aegis/a/data/kb/rule-obligations.json`, keeps `kind == "rule"`, and writes `{ section: mechanisms }` sorted by rule id to `qa/rules/aegis/mechanisms-by-rule.json`. Expected: 1,119 keys; 18 distinct mechanism strings.

- [ ] **Step 3: Write the scenario index**

`conformance-scenario-index.md`: for each `D:/tmp_aegis/a/apps/api/src/engine/conformance/*.test.ts`, one line per `it(...)` title with the file name (the grep from the investigation). This is the checklist Tasks 7–13 read; it is documentation, not code.

- [ ] **Step 4: README**

State: source repo + commit hash, MIT attribution, what each file is, that these files are **inputs** (never edited by hand), and the Phase 0 switch note for `comprehensive-4.3.md`.

- [ ] **Step 5: Commit**

```bash
git add qa/rules/aegis
git commit -m "qa/rules: vendor Aegis 4.3 rules extraction, mechanism map and scenario index (MIT, ec0cd22f0)"
```

---

### Task 2: Rule-unit parser and normaliser

**Files:**
- Create: `code/tools/rules/__init__.py`, `code/tools/rules/parse_rules.py`
- Test: `code/tests/tools/test_rules_parse.py`

**Interfaces:**
- Produces: `parse_rules(path: Path) -> list[RuleUnit]` where `RuleUnit` is a dataclass `(id: str, chapter: int, text: str, text_norm: str, fingerprint: str)`; `normalise(text: str) -> str`; `fingerprint(text_norm: str) -> str` (hex SHA-256); CLI `python -m tools.rules.parse_rules <path> --json` printing the list.

- [ ] **Step 1: Write the failing tests**

```python
def test_parses_first_and_last_rule(tmp_path):
    units = parse_rules(REPO / "qa/rules/aegis/comprehensive-4.3.md")
    ids = [u.id for u in units]
    assert ids[0] == "1-1" and "18-3-3-3" in ids and len(units) == 1119

def test_toc_update_history_excluded():
    units = parse_rules(SRC)
    assert all(not u.text.startswith("Update History") for u in units)
    assert "1-1" in {u.id for u in units} and ids.count("1-1") == 1

def test_normalise_is_stable_across_extractions():
    a = "15-4-3-5-1 The turn player chooses 1 of their effects to\nactivate from among the effects that\ntriggered simultaneously.26"
    b = "15-4-3-5-1 The turn player  chooses 1 of their effects to activate from among the effects that triggered simultaneously."
    assert normalise(a) == normalise(b)

def test_fingerprint_is_sha256_of_norm():
    assert fingerprint(normalise("x")) == hashlib.sha256(normalise("x").encode()).hexdigest()
```

- [ ] **Step 2: Run to verify they fail**

Run: `python -m pytest code/tests/tools/test_rules_parse.py -q`
Expected: FAIL, `ModuleNotFoundError: tools.rules`

- [ ] **Step 3: Implement `parse_rules.py`**

Port the parser from the investigation scratchpad (`diff_rules.py`): rule line regex `^((?:1[0-8]|[1-9])(?:-\d+)+)\.\s*(.*)$`, start at the first `1-1.` line, stop at `Update History` only after start, continuation lines appended with a space. `normalise` per Global Constraints.

- [ ] **Step 4: Run to verify they pass**

Run: `python -m pytest code/tests/tools/test_rules_parse.py -q`
Expected: 4 passed

- [ ] **Step 5: Commit**

```bash
git add code/tools/rules code/tests/tools/test_rules_parse.py
git commit -m "tools/rules: Comprehensive Rules unit parser + stable normaliser/fingerprint"
```

---

### Task 3: Obligation denominator generator with status sidecar

**Files:**
- Create: `code/tools/rules/build_obligations.py`, `qa/rules/cr_obligations_status.yaml` (sidecar, hand-maintained), `qa/rules/cr_obligations.json` (generated)
- Test: `code/tests/tools/test_rules_build_obligations.py`

**Interfaces:**
- Consumes: Task 2 `parse_rules`, Task 1 `mechanisms-by-rule.json`.
- Produces: `qa/rules/cr_obligations.json` = `{ "version": 1, "source": {"file", "rules_version": "4.3", "sha256"}, "mechanics_set": [...12 tags], "obligations": { "<id>": { "chapter": int, "text": str, "fingerprint": str, "mechanisms": [str], "status": "gap"|"proven"|"not-testable", "blocks_training": bool, "tests": [str], "gap_id": str|null, "note": str|null } }, "summary": { "<status>": n, "by_chapter": {...}, "mechanics": {"proven": n, "gap_blocking": n, "gap_nonblocking": n, "not_testable": n} } }`.
- Sidecar schema (`cr_obligations_status.yaml`): `{ "<id>": { status, tests?: [str], gap_id?, blocks_training?, note? } }`; absent id ⇒ `gap`, `blocks_training: false`.
- CLI: `python -m tools.rules.build_obligations [--source PATH] [--check]`; `--check` exits 1 if the committed JSON differs from a fresh build (same contract as `build_tested_cards.py`).

- [ ] **Step 1: Write the failing tests**

```python
def test_default_status_is_gap_and_mechanisms_attached(tmp_path):
    out = build(source=SRC, sidecar={}, mechanisms=MECH)
    o = out["obligations"]["15-4-3-5-1"]
    assert o["status"] == "gap" and o["blocks_training"] is False
    assert "effects-and-ordering" in o["mechanisms"]

def test_sidecar_overrides_status_and_tests():
    out = build(SRC, {"15-4-3-5-1": {"status": "proven", "tests": ["ch15_04_effect_states::turn_player_orders_first"]}}, MECH)
    assert out["obligations"]["15-4-3-5-1"]["tests"] == ["ch15_04_effect_states::turn_player_orders_first"]

def test_not_testable_seeded_for_headings():
    out = build(SRC, {}, MECH)
    assert out["obligations"]["15-4"]["status"] == "not-testable"   # bare heading "Effect States"

def test_summary_mechanics_counts():
    out = build(SRC, {"13-1-2": {"status": "gap", "blocks_training": True}}, MECH)
    assert out["summary"]["mechanics"]["gap_blocking"] == 1

def test_check_detects_drift(tmp_path):
    write(tmp_path/"cr.json", build(SRC, {}, MECH)); assert check(tmp_path/"cr.json", SRC, {}, MECH) == 0
    assert check(tmp_path/"cr.json", SRC, {"1-1": {"status": "proven", "tests": ["x::y"]}}, MECH) == 1
```

- [ ] **Step 2: Run to verify they fail**

Run: `python -m pytest code/tests/tools/test_rules_build_obligations.py -q` — Expected: FAIL (import error)

- [ ] **Step 3: Implement `build_obligations.py`**

`build(source, sidecar, mechanisms) -> dict`, `check(committed, source, sidecar, mechanisms) -> int`, `main()`. Heading rule for `not-testable` seeding: a unit whose text has no sentence terminator and ≤ 6 words (e.g. "Effect States", "Deletion Targets"). Serialise with `sort_keys=True, indent=2, ensure_ascii=False`.

- [ ] **Step 4: Run to verify they pass; generate the artifact**

Run: `python -m pytest code/tests/tools/test_rules_build_obligations.py -q` — Expected: 5 passed
Run: `PYTHONPATH=code python -m tools.rules.build_obligations && PYTHONPATH=code python -m tools.rules.build_obligations --check` — Expected: exit 0; `summary.gap` ≈ 1,000.

- [ ] **Step 5: Commit**

```bash
git add code/tools/rules/build_obligations.py qa/rules/cr_obligations_status.yaml qa/rules/cr_obligations.json code/tests/tools/test_rules_build_obligations.py
git commit -m "qa/rules: 4.3 obligation denominator (1,119 rules) with status sidecar and --check"
```

---

### Task 4: Coverage report and proven-test existence check

**Files:**
- Create: `code/tools/rules/coverage_report.py`
- Modify: `scripts/verify` (tier 0 python checks, next to `keyword_semantics_matrix_check` at ~line 150)
- Test: `code/tests/tools/test_rules_coverage_report.py`

**Interfaces:**
- Consumes: `qa/rules/cr_obligations.json`.
- Produces: `report(obligations: dict, rust_test_fns: set[str]) -> str` (Markdown table per chapter + mechanics summary, denominator always printed) and `check_proven_tests_exist(obligations, rust_test_fns) -> list[str]` (missing `module::fn` names). `collect_rust_test_fns(root: Path) -> set[str]` scans `code/digimon-engine/tests/rules_conformance/*.rs` for `#[test]` + `fn name` and returns `module::name`. CLI `python -m tools.rules.coverage_report [--check]`; `--check` exits 1 when any proven test is missing.

- [ ] **Step 1: Write the failing tests**

```python
def test_report_prints_denominator_and_chapters():
    txt = report(OBL, set())
    assert "| 15 |" in txt and "1119" in txt

def test_check_fails_when_proven_test_missing():
    obl = with_status(OBL, "15-4-3-5-1", "proven", tests=["ch15_04_effect_states::nope"])
    assert check_proven_tests_exist(obl, {"ch15_04_effect_states::turn_player_orders_first"}) == ["ch15_04_effect_states::nope"]

def test_collect_rust_test_fns(tmp_path):
    (tmp_path/"ch99_x.rs").write_text("#[test]\nfn a_b() {}\n#[test]\n#[ignore = \"G-X\"]\nfn c_d() {}\n")
    assert collect_rust_test_fns(tmp_path) == {"ch99_x::a_b", "ch99_x::c_d"}
```

- [ ] **Step 2: Run to verify they fail** — `python -m pytest code/tests/tools/test_rules_coverage_report.py -q` → FAIL

- [ ] **Step 3: Implement; wire into `scripts/verify` tier 0 as a **report-only** step (run, print, never fail) plus the `--check` as a failing step**

Follow the existing `keyword_semantics_matrix_check` entry shape exactly.

- [ ] **Step 4: Run tests; run `scripts/verify --tier 0`** — Expected: tests pass; tier 0 prints the table.

- [ ] **Step 5: Commit** — `git commit -m "tools/rules: coverage report (per chapter + mechanics) and proven-test existence check in verify tier 0"`

---

### Task 5: Renumber map 3.6 → 4.3

**Files:**
- Create: `code/tools/rules/renumber_map.py`, `docs/digimon-rules/renumber-3.6-to-4.3.json`
- Test: `code/tests/tools/test_rules_renumber_map.py`

**Interfaces:**
- Consumes: Task 2 `parse_rules`; a 3.6 text (`pdftotext -raw` of the base-repo `general_rule.pdf` or `general_rule_v3.6.pdf` once Phase 0 renames it; the tool takes `--old PATH`).
- Produces: JSON `{ "renumbered": {"<old>": "<new>"}, "reworded": [{"old","new","similarity"}], "added": [ids], "removed": [ids] }` via `build_map(old_units, new_units) -> dict`; pairing = exact normalised text, then fuzzy ≥ 0.6 (`difflib.SequenceMatcher`) with the ≥ 0.97 threshold separating `renumbered` from `reworded`.

- [ ] **Step 1: Write the failing tests**

```python
def test_known_renumbers():
    m = build_map(OLD, NEW)["renumbered"]
    assert m["16-40"] == "16-41" and m["16-6-4"] == "16-7-4" and m["16-23"] == "16-24"

def test_reversal_is_reworded_not_renumbered():
    rew = {(r["old"], r["new"]) for r in build_map(OLD, NEW)["reworded"]}
    assert ("15-7-4", "15-7-4") in rew

def test_added_includes_new_keywords():
    assert {"16-42", "16-43", "16-44", "16-45", "16-46", "16-47"} <= set(build_map(OLD, NEW)["added"])
```

- [ ] **Step 2: Run to verify they fail** → FAIL
- [ ] **Step 3: Implement** — port `diff_rules2.py` from the investigation scratchpad onto `RuleUnit`.
- [ ] **Step 4: Run tests; generate the committed map** — `PYTHONPATH=code python -m tools.rules.renumber_map --old <3.6 txt> --new qa/rules/aegis/comprehensive-4.3.md --out docs/digimon-rules/renumber-3.6-to-4.3.json`
- [ ] **Step 5: Commit** — `git commit -m "tools/rules: renumber map 3.6→4.3 (367 renumbered, 59 reworded)"`

---

### Task 6: `rules_conformance` test target and the `cite!` macro

**Files:**
- Create: `code/digimon-engine/tests/rules_conformance/main.rs`, `code/digimon-engine/tests/rules_conformance/cite.rs`, `code/digimon-engine/tests/rules_conformance/meta.rs`
- Modify: `code/digimon-engine/Cargo.toml` (new `[[test]]`; add `sha2` to `[dev-dependencies]` if absent), `qa/verification/tier1_suites.json` (add suite `rules_conformance`, required)

**Interfaces:**
- Produces: `cite!(id: &str, fingerprint: &str)` and `cite!(id)` (unpinned, allowed only during a chapter's first draft; the meta-test counts them and the chapter task's last step pins them). `pub fn rule_text(id: &str) -> &'static str`; `pub fn normalise(s: &str) -> String` (must equal Python's); `pub fn fingerprint(norm: &str) -> String`. Obligations loaded via `include_str!("../../../../qa/rules/cr_obligations.json")`.
- Registration: mirror `judge_quiz` (`required-features = ["dsl-yaml-loader"]`).

- [ ] **Step 1: Write the failing tests (`meta.rs`)**

```rust
#[test] fn cite_known_rule_with_correct_fingerprint_passes() { cite!("15-4-3-5-1", /* fingerprint from cr_obligations.json */ "…"); }
#[test] #[should_panic(expected = "fingerprint mismatch")] fn cite_with_wrong_fingerprint_panics() { cite!("15-4-3-5-1", "deadbeef"); }
#[test] #[should_panic(expected = "unknown rule id")] fn cite_unknown_id_panics() { cite!("99-9-9"); }
#[test] fn rust_normalise_matches_python_fingerprints() { for (id, o) in obligations() { assert_eq!(fingerprint(&normalise(o.text)), o.fingerprint, "{id}"); } }
#[test] fn every_test_in_this_target_cites_a_rule() { /* scan include_str! of each chNN_*.rs: every `#[test]` block contains `cite!(` */ }
```

- [ ] **Step 2: Run to verify compile failure** — `cargo test --manifest-path code/digimon-engine/Cargo.toml --test rules_conformance` → error: target not found / macro undefined.

- [ ] **Step 3: Implement `cite.rs`** — `normalise` mirrors Task 2 exactly (same regexes); `cite!` expands to `$crate_cite::check(id, Some(fp))`, which panics with `"unknown rule id {id}"` or `"fingerprint mismatch for {id}: text is now {norm}"`. Register the target; add `rules_conformance` to `tier1_suites.json`.

- [ ] **Step 4: Run** — Expected: 5 passed (the every-test-cites meta-test passes vacuously until Task 7).

- [ ] **Step 5: Commit** — `git commit -m "tests: rules_conformance target with fingerprinted cite! macro; tier-1 suite"`

---

### Task 7: ch15-04 — Effect states (§15-4 trigger states)

**Files:**
- Create: `code/digimon-engine/tests/rules_conformance/ch15_04_effect_states.rs`, `code/digimon-engine/tests/rules_conformance/fixtures/ch15_04.yaml` (inline YAML cards, loaded with `include_str!` + `from_dsl_yaml`)
- Modify: `main.rs` (`mod ch15_04_effect_states;`), `qa/rules/cr_obligations_status.yaml` (statuses for every rule cited), `docs/RUST_ENGINE_GAPS.md` (one entry per red test)

**Interfaces:**
- Consumes: `DebugRunner::builder().from_dsl_yaml(FIXTURES)?.memory(n).start()`, `r.place_on_field(player, card_id, Some(slot))`, `r.attack_player(..)`, `r.pending_selection()`, `r.pending_kind()`, `digimon_engine::selection::SelectionKind::TriggerOrder`, `r.end_turn()`.
- Fixture cards (names fixed so later tasks can reuse): `WA_MEM` (`[When Attacking] Gain 1 memory`), `WA_DRAW` (`[When Attacking] <Draw 1>`), `WA_ALLIANCE` (printed `<Alliance>`), `OPP_WA_OBS` (`[All Turns] When an opponent's Digimon attacks, gain 1 memory`), `OP_PLAY_LV3` (`[On Play] Play 1 level 3 Digimon card from your hand without paying the cost`), `OD_MEM` (`[On Deletion] Gain 1 memory`), `WA_HYBRID_DIGI` (`[When Attacking] This Digimon may digivolve into a Digimon card with the [Hybrid] trait in the hand with the digivolution cost reduced by 1` — the §15-4-4-4 example), `WD_DEL_RET` (the §15-4-1-2 example: delete ≤8000 then return a level 6+).

- [ ] **Step 1: Write the failing tests** (one per obligation; names are the sidecar `tests` entries)

```rust
/// §15-4-2-3 "Effects that are pending activation must be activated 1 at a time."
#[test] fn pending_effects_activate_one_at_a_time() { cite!("15-4-2-3", "…"); /* WA_MEM + WA_DRAW on one attacker: after attack, a TriggerOrder prompt with 2 options; resolve one → memory +1 and hand unchanged; then the second */ }
/// §15-4-3-2 all triggers before one resolution are simultaneous — Alliance joins the pool
#[test] fn when_attacking_and_alliance_share_one_pool() { cite!("15-4-3-2"); cite!("16-24-2"); /* WA_MEM with printed <Alliance> + a second own Digimon: exactly ONE TriggerOrder prompt offering both; no separate Alliance window first */ }
/// §15-4-3-5-1/2 turn player orders all theirs, then non-turn player
#[test] fn turn_player_orders_first_then_non_turn_player() { /* attacker with WA_MEM+WA_DRAW vs defender with OPP_WA_OBS: prompts are P0, P0, then P1's fires; assert memory deltas in that order */ }
/// §15-4-4-3 pending effect lost when the card becomes a new card
#[test] fn pending_on_play_lost_when_card_leaves_before_activation() { /* OP_PLAY_LV3 played while an opponent [All Turns] "when a Digimon is played, return it to hand" is pending-first… assert OP_PLAY_LV3's play never happens */ }
/// §15-4-4-4 (manual's own example) digivolving via the 2nd [When Attacking] cancels the 1st
#[test] fn digivolving_mid_pending_cancels_sibling_when_attacking() { /* WA_HYBRID_DIGI + WA_DRAW: choose the digivolve first → the draw can no longer activate (hand count unchanged apart from the digivolve draw) */ }
/// §15-4-4-5 trigger condition no longer met → cannot activate
#[test] fn pending_effect_whose_condition_lapsed_is_dropped() { /* an [On Deletion] "if you have 3 or more security" where a sibling pending effect first trashes security to 2 */ }
/// §15-4-5-2 derived trigger activates before older pending
#[test] fn derived_trigger_resolves_before_pending_siblings() { /* two pending [When Attacking]; the first resolved deletes an opp OD_MEM → OD_MEM fires before the second pending */ }
/// §15-4-5-3 derived trigger for the NON-turn player still goes first
#[test] fn non_turn_player_derived_trigger_precedes_turn_player_pending() { }
/// §15-4-1-2 whole effect resolves even if the card becomes new mid-effect
#[test] fn activated_effect_completes_after_source_becomes_new_card() { /* WD_DEL_RET: deleting the ≤8000 Digimon triggers an immediate effect that deletes the source; the level-6 return still happens */ }
```

- [ ] **Step 2: Run; record which fail** — `cargo test --manifest-path code/digimon-engine/Cargo.toml --test rules_conformance ch15_04` — Expected: compile OK; some tests red (F4 predicts `when_attacking_and_alliance_share_one_pool`).

- [ ] **Step 3: Triage each red test** — read `effect_queue.rs` / `combat/mod.rs` to confirm the divergence is the engine's, not the fixture's; `#[ignore = "G-ENGINE-<SLUG> (§id)"]`; add the gap entry (rule text, our code path, DCGO path from `AutoProcessing.cs`/`MultipleSkills.cs`, Aegis note if any); sidecar `status: gap, blocks_training: true, gap_id`. Green tests → sidecar `proven` with the `module::fn` names; pin every `cite!` with its fingerprint.

- [ ] **Step 4: Verify** — `cargo test … --test rules_conformance` all green or ignored; `cargo run -p dsl-lint -- code/digimon-engine/tests/rules_conformance/fixtures`; `PYTHONPATH=code python -m tools.rules.build_obligations && python -m tools.rules.coverage_report --check`.

- [ ] **Step 5: Commit** — `git commit -m "rules_conformance: ch15-04 effect states (§15-4-1-2..15-4-5-3); N proven, M gaps filed"`

---

### Task 8: ch15-08 — Effect categories (§15-8-3 trigger-type, §15-8-4 activation-type, §15-8-5 immediate-type)

**Files:** `ch15_08_effect_categories.rs`, `fixtures/ch15_08.yaml`; sidecar; gaps.

**Interfaces:** reuses Task 7 fixtures; adds `SCAPEGOAT_LIKE` (the §15-8-5-4 example: `[All Turns] When this Digimon would be deleted, by deleting 1 other Digimon with [Sukamon] in its name, it isn't deleted`), `INH_OD_MEM` (inherited `[On Deletion] Gain 1 memory`), `EOT_MEM` (`[End of Your Turn] Gain 2 memory`), `MAIN_PAY2_DEL` (the §15-8-4-1 example), `IMM_COND` (immediate effect with a processing condition met only later in the window).

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn trigger_type_waits_for_causing_effect_to_finish()            // §15-8-3-2 — an [All Turns] "when a card is trashed from under your Tamer" observer fires AFTER the trashing effect's remaining text (owner ruling 1; #726 territory)
#[test] fn inherited_on_deletion_is_pending_on_the_former_top_card()  // §15-8-3-5 — INH_OD_MEM under a Digimon deleted in battle: trigger offered once, attributed to the deleted top card
#[test] fn end_of_turn_trigger_persists_through_memory_change()        // §15-8-3-6 — EOT_MEM triggers; another EOT effect moves memory back to ≥0 (§6-6-4) — EOT_MEM still resolves
#[test] fn trigger_reference_uses_state_at_processing()                // §15-8-3-7
#[test] fn trigger_reference_tied_to_condition_uses_trigger_time_state() // §15-8-3-8 — "level ≤ the played Digimon" uses level at play even if it digivolved before activation
#[test] fn processing_condition_checked_at_activation_not_trigger()    // §15-8-3-9-2 (open gap F-ENGINE-TRIGGER-GATE-IS-TRIGGER-TIME-NOT-ACTIVATION-TIME predicts red)
#[test] fn removal_trigger_processing_condition_uses_trigger_state()   // §15-8-3-9-4 — "[On Deletion] When this Digimon has digivolution cards" evaluated at deletion
#[test] fn activation_type_declarable_only_when_optional_condition_payable() // §15-8-4-4-1 — <Training> not offered when suspended; once declared, must suspend
#[test] fn immediate_effect_interrupts_before_cause()                  // §15-8-5-2 — SCAPEGOAT_LIKE: prevention happens, card never reaches trash, no OnDeletion fires
#[test] fn immediate_effects_only_simultaneous_with_immediate_effects() // §15-8-5-3 — an [On Deletion] trigger never shares a prompt with a would-be-deleted replacement
#[test] fn immediate_effect_once_per_interrupting_cause()               // §15-8-5-4 — SCAPEGOAT_LIKE used once; a second deletion attempt from the SAME cause cannot use it again, a new [Main] can
#[test] fn immediate_effect_with_condition_met_later_can_activate()     // §15-8-5-5-2
```

- [ ] **Step 2–5:** as Task 7 (run, triage red → ignore+gap+sidecar, lint, commit `"rules_conformance: ch15-08 effect categories (§15-8-3..15-8-5)"`).

---

### Task 9: ch15-07 — Optional processing conditions, incl. the 4.3 reversal

**Files:** `ch15_07_optional_conditions.rs`, `fixtures/ch15_07.yaml`.

**Interfaces:** `WA_TRASH_SEC_MINUS5000` (the §15-7-4/15-7-5 example), `OD_TRASH_HAND_MEM` (§15-7-1 example), `WD_TRASH2_UNSUSPEND_THEN` (§15-7-2 example), `OD_RETURN_KIMERA_MACHINE_PLAY_MILLENNIUM` (§15-7-3 example).

- [ ] **Step 1: Tests**

```rust
#[test] fn optional_condition_is_a_choice_then_mandatory_payload()    // §15-7-1 — prompt yes/no; on yes the memory gain is not a second prompt
#[test] fn declining_condition_skips_everything_after_it()            // §15-7-2 — incl. the "After" sentence
#[test] fn partial_payment_of_compound_condition_is_impossible()      // §15-7-3 — only Kimeramon in trash → no prompt at all
#[test] fn unpayable_optional_condition_cannot_be_chosen()            // §15-7-4 (4.3 REVERSED) — 0 security → no prompt; `pending_selection()` is None (open gap "unpayable optional costs still park a prompt" predicts red)
#[test] fn payable_condition_offered_even_when_payload_has_no_target()// §15-7-5 — opponent has no Digimon → prompt still offered; accepting trashes security
#[test] fn optional_condition_decided_before_processing_effect()      // §15-1-2 (4.3) — decision prompt precedes any payload selection prompt
#[test] fn mandatory_invoker_does_not_force_invoked_optional_condition() // §15-15-7-4 (4.3 reversed) — "activate 1 [On Play] effect … as this Digimon's effect" whose target has a "by …" condition: the borrower may decline
```

- [ ] **Step 2–5:** as Task 7; commit `"rules_conformance: ch15-07 optional processing conditions incl. 4.3 §15-7-4 reversal"`.

---

### Task 10: ch17 — Rule checks, and ch15-04-3-3 rule-check triggers

**Files:** `ch17_rule_checks.rs`, `fixtures/ch17.yaml`.

**Interfaces:** `OP_MINUS3000_SA_MINUS1` (§17-1-2-2 example), `NO_DP_DIGIMON` (a Digimon card with no DP), a vanilla Option, `LINK_LIMIT_HOST` + two link cards, a `[Start of Your Turn]` effect card.

- [ ] **Step 1: Tests**

```rust
#[test] fn rule_check_not_during_effect_processing()         // §17-1-2-2 — 3000-DP opp Digimon: after OP_MINUS3000_SA_MINUS1 resolves it is deleted AND it had received SA-1 first (observe via event log order)
#[test] fn rule_check_not_during_rule_processing()            // §17-1-2-1 — an immediate effect during a rule check plays a 0-DP Digimon; it is deleted by the NEXT check, after the current one resolves
#[test] fn zero_dp_digimon_in_battle_area_is_deleted()        // §17-1-3-1-1
#[test] fn digimon_without_dp_in_battle_area_is_trashed_not_deleted() // §17-1-3-2-1 — no [On Deletion] fires
#[test] fn option_in_battle_area_not_placed_by_effect_is_trashed()    // §17-1-3-2-2 — and one placed by effect stays (§4-28)
#[test] fn non_digimon_in_breeding_area_is_trashed()          // §17-1-3-2-3
#[test] fn face_down_card_on_field_is_trashed()               // §17-1-3-2-4
#[test] fn excess_link_cards_trashed_player_chooses_which()   // §17-1-3-2-5 + Q6370 — a selection prompt, not a tail trim (predicted red)
#[test] fn link_card_failing_requirement_is_trashed()         // §17-1-3-2-6
#[test] fn rule_check_deletion_trigger_is_simultaneous_with_timing_triggers() // §15-4-3-3 — 0-DP at start of turn: [On Deletion] and [Start of Your Turn] share one TriggerOrder prompt
#[test] fn all_rule_check_targets_processed_simultaneously()  // §17-1-3 — a 0-DP Digimon and an orphan Option: both gone after one checkpoint; the [On Deletion] fires once, after both
```

- [ ] **Step 2–5:** as Task 7; commit `"rules_conformance: ch17 rule checks (§17-1-2, §17-1-3-*) + §15-4-3-3"`.

---

### Task 11: ch18 + §9-1-5 — Pending processing, overwrite processing, loops, Option disposal

**Files:** `ch18_pending_overwrite.rs`, `fixtures/ch18.yaml`.

**Interfaces:** `WA_GAIN3_LOSE3_EOT` (§18-1-1 example), `MAIN_DEL_6000_OR_LOWEST_IF_GREYMON` (§18-2-1 example), an Option with two `[Main]` sentences, a self-referential loop pair for §18-3.

- [ ] **Step 1: Tests**

```rust
#[test] fn pending_processing_fires_at_its_timing_like_a_trigger()      // §18-1-1 — memory −3 at end of turn
#[test] fn pending_processing_orders_with_simultaneous_triggers()        // §18-1-2 — shares the EOT prompt with EOT_MEM
#[test] fn instead_overwrites_the_normal_processing()                    // §18-2-1
#[test] fn used_option_trashed_immediately_after_first_main_uninterrupted() // §9-1-5 (4.3) — no rule check / trigger between [Main] and trash
#[test] fn infinite_loop_is_stopped_not_silently_cleared()               // §18-3-3-2/3 — the engine must emit an observable event / return an explicit outcome and keep other pending triggers (F5: MAX_CHAIN_DEPTH clear predicts red)
#[test] fn piercing_is_pending_processing_before_end_of_attack()         // §16-7-4 — [On Deletion] of the battled Digimon resolves BEFORE the Piercing check; both before [End of Attack]
```

- [ ] **Step 2–5:** as Task 7; commit `"rules_conformance: ch18 pending/overwrite processing, §9-1-5, §16-7-4"`.

---

### Task 12: ch11–14 — Attack, block, security check, battle procedure

**Files:** `ch11_14_attack_procedure.rs`, `fixtures/ch11_14.yaml`.

**Interfaces:** `EOA_OBS` (`[All Turns] [End of Attack] Gain 1 memory` observer), `BLOCKER`, `BLOCKER_CANNOT_SUSPEND`, `SA_PLUS1`, `PIERCING_X2` (two instances), `SEC_CHECK_OBS_MINUS_DP` (`[All Turns] When a security check is performed, the attacking Digimon gets −3000 DP for the turn` — F2 observer), `JAMMING`, `EOB_OBS` (`[End of Battle]`).

- [ ] **Step 1: Tests**

```rust
#[test] fn attack_timings_continue_when_attacker_leaves_mid_resolution()  // §11-1-5
#[test] fn removed_target_stays_the_target_and_attack_fails()            // §11-2-6
#[test] fn unsuccessful_attack_reaches_end_of_attack_timing()            // §11-5-1-4-1 (4.3) — EOA_OBS fires after the defender was deleted pre-battle
#[test] fn attack_does_not_end_until_all_processing_resolved()           // §11-6-2
#[test] fn one_block_per_attack()                                        // §12-1-2
#[test] fn digimon_that_cannot_suspend_cannot_block()                    // §12-1-4
#[test] fn attack_target_cannot_block()                                  // §12-1-5
#[test] fn one_security_check_per_attack_sa_plus_counts_inside_it()      // §13-1-2 — SA+1 checks 2 cards within one check; a second Piercing instance adds none (§16-7-5)
#[test] fn checking_digimon_leaving_stops_further_checks()               // §13-1-5
#[test] fn check_triggered_effects_resolve_before_security_battle()      // §13-1-8-2 then 13-1-8-3 — F2 (expected red until the fix session lands; then proven)
#[test] fn security_effect_activates_before_other_check_triggers()       // §15-16-10-2
#[test] fn non_digimon_security_card_skips_battle_and_is_trashed()       // §13-1-8-3-2, 13-1-8-4
#[test] fn battle_compares_dp_then_deletes_loser_then_end_of_battle()    // §14-2-1..14-2-5 — EOB_OBS fires after deletion triggers
#[test] fn jamming_survives_security_battle_only()                       // §16-9 (4.3 numbering; verify) — not vs a normal Digimon battle
```

- [ ] **Step 2–5:** as Task 7; commit `"rules_conformance: ch11–14 attack/block/security/battle procedure"`.

---

### Task 13: ch16 — Keyword lifecycles (four modules)

**Files:** `ch16_new_keywords.rs`, `ch16_replacement_keywords.rs`, `ch16_attack_keywords.rs`, `ch16_processing_keywords.rs`; `fixtures/ch16_*.yaml`. Each keyword gets a fixture card carrying only that keyword (plus whatever the rule text's own example needs).

**Interfaces:** per keyword, three standard tests named `<kw>_kind_matches_rule` (persistent / trigger / activation / immediate per §16-N-2), `<kw>_optionality_matches_rule` (optional vs mandatory vs optional-cost→mandatory per §16-N-3, cross-checked with `qa/rules/keyword_semantics_matrix.json`), `<kw>_timing_matches_rule` (the §16-N-4.. timing sentence), plus one test per numbered sub-rule that states behaviour (e.g. `alliance_dp_and_sa_persist_after_suspended_digimon_leaves` §16-24-5, `blitz_attack_proceeds_after_memory_returns_to_zero_or_more` §16-16-4, `blitz_cannot_attack_when_memory_already_zero_or_more` §16-16-5, `de_digivolve_multi_card_trash_is_simultaneous_but_sequential` §16-12-5, `material_save_allows_duplicate_specified_cards` §16-21-5, `blast_dna_cannot_ignore_dna_requirements` §16-31-5, `ascension_optional_places_on_top_of_security` §16-43, `engage_optional_eot_attack` §16-44, `guard_immediate_optional_other_digimon_leave_by_opp_effect` §16-45, `detach_optional_cost_then_mandatory_stay` §16-46, `succession_gains_all_effects_of_topmost_specified_source` §16-47, `use_req_ignores_color_requirements_with_specified_cards` §16-42, `overflow_turn_player_orders_then_non_turn_player` §4-19-5-1/2).

- [ ] **Step 1: Enumerate** — from `qa/rules/cr_obligations.json`, list every `16-*` id whose status is `gap` and whose text states behaviour; write one test per id, in the module matching the keyword's family. Order: new keywords module first (never verified), then replacement (immediate-type) keywords, then attack keywords, then processing keywords.
- [ ] **Step 2–5:** as Task 7, one commit per module: `"rules_conformance: ch16 <family> keywords (§16-a..§16-b)"`.

---

### Task 14: Chapter triage close-out and gap digest

**Files:**
- Modify: `qa/rules/cr_obligations_status.yaml`, `qa/rules/cr_obligations.json` (regenerated), `docs/RUST_ENGINE_GAPS.md`
- Create: `qa/rules/mechanics-gaps-2026-10.md` (digest: every `blocks_training` gap with rule id, test name, DCGO stance, Aegis stance, one-line fix shape) — this is the input to Plan 3 (fixes) and the batched rulings worksheet for cases where the PDF is silent.

- [ ] **Step 1:** Regenerate and check: `PYTHONPATH=code python -m tools.rules.build_obligations --check && PYTHONPATH=code python -m tools.rules.coverage_report --check`.
- [ ] **Step 2:** Write the digest; group gaps by mechanism; mark each `PDF-explicit` (fix without asking) or `PDF-silent` (goes to the owner's worksheet).
- [ ] **Step 3:** Run the whole target and tier 1: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test rules_conformance` (all pass or ignored), `scripts/verify --tier 1`.
- [ ] **Step 4:** Commit — `git commit -m "qa/rules: mechanics gap digest + obligation statuses after ch15/16/17/18/11–14"`.

---

## Self-review notes

- Spec coverage: Phase 1 → Tasks 2–5; Phase 2 chapters 15, 16, 17, 18, 11–14 → Tasks 7–13; §6 "vendor Aegis" → Task 1; §6 "fingerprint never blocks card PRs" → Task 6 (separate target, tier 1 only); Phase 2's ch06–09 and ch01–05/10 are **deferred to a follow-up plan** by the owner's mechanics-first steer (§0); Phase 3 (fixes) is Plan 3; §5b gate is Plan 2; Phase 4 (Q&A) and Phase 5 CI widening are later plans.
- Names used consistently: `parse_rules`, `normalise`, `fingerprint`, `build`, `check`, `report`, `collect_rust_test_fns`, `cite!`, fixture card ids as listed per task.
- Proportion: test bodies are given as names + the rule they assert + the observable; implementers write staging from the fixture list and the DebugRunner API named in Task 7's Interfaces.
