# Exam Ledger — Fleet Safety Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reshape the exam verdict store so several nodes can run archetype campaigns in parallel and merge their results through git without conflicting or repeating each other's work.

**Architecture:** Three files replace one blob. Verdicts split into one JSON file per card (disjoint writers never touch the same file). An append-only JSONL attempt log records history under a git union merge driver. Advisory per-card claim files with expiry stop two nodes starting the same card. A generated Markdown index rolls all three up for a human. The in-memory `VerdictStore` API is unchanged — only persistence and three new modules are added.

**Tech Stack:** Rust 2021 (`code/tools/dcgo-harness`, serde + serde_json + chrono), Python 3 stdlib (`code/tools/clause_coverage`), git merge drivers.

**Spec:** `docs/superpowers/specs/2026-08-27-archetype-campaign-fleet-design.md` §1.

## Global Constraints

- **Per-worktree `CARGO_TARGET_DIR`** (CLAUDE.md rule 31). If the session predates the env change, prefix every cargo command with `CARGO_TARGET_DIR='D:\cargo-target-wt\agent-card-authoring-skills-f0579a'`. A phantom compile error in a file you did not touch means target-dir contamination — suspect it before debugging your change.
- **Python tools are standard-library only**, matching the rest of `code/tools/clause_coverage/`.
- **`dcgo-harness` is dev/test tooling**: never imported by `server.*` or `digimon_gym.*`, never bundled into a production build.
- **Clause identity is never invented here.** A clause id is `clause_coverage.models.Clause.id` == `{card_id}#{zone}#{idx}` (e.g. `EX12-073#security#0`).
- **`unmeasured` is a real outcome.** Nothing in this plan may make a card read as "passed" on a partial denominator.
- **A missing ledger is not an error** (fresh checkout) — it means everything is `unmeasured`. A missing *explicitly named* file still is an error, so a typo'd path can never masquerade as an empty store. Preserve that distinction exactly as `VerdictStore::load` does today.
- Existing public API that must keep working unchanged: `VerdictStore::{load, save, to_json, from_json, record, get, get_validated, set_current_text_sha, is_invalidated, iter, len, is_empty, summary}`.

## File Structure

| File | Responsibility |
|---|---|
| `code/tools/dcgo-harness/src/exam/verdict.rs` (modify) | Add directory persistence (`load_dir` / `save_dir` / `card_file_name`) beside the existing single-file `load`/`save`. In-memory shape unchanged. |
| `code/tools/dcgo-harness/src/exam/ledger.rs` (create) | The attempt log (`Attempt`, `append_attempt`, `read_attempts`) and claims (`Claim`, `claim_cards`, `release_cards`, `read_claims`). |
| `code/tools/dcgo-harness/src/exam/mod.rs` (modify) | Register `pub mod ledger;`. |
| `code/tools/dcgo-harness/src/main.rs` (modify) | `exam migrate-verdicts` subcommand; default `--verdicts` path points at the directory. |
| `code/tools/clause_coverage/exam_binding.py` (modify) | `load_verdict_store` accepts a directory as well as a file. |
| `code/tools/clause_coverage/exam_index.py` (create) | Render `exam-index.md` from binding + ledger. |
| `.gitattributes` (modify) | Union merge driver for `exam-log.jsonl`. |
| `qa/qa-reports/exam-verdicts/<CARD-ID>.json` (created by migration) | The per-card verdict files. |
| `qa/qa-reports/exam-log.jsonl` (created on first append) | Append-only attempt history. |
| `qa/qa-reports/exam-claims/<CARD-ID>.claim` (created on claim) | Advisory leases. |
| `code/tests/tools/test_clause_coverage_exam_binding.py` (modify) | Directory-store cases. |
| `code/tests/tools/test_exam_index.py` (create) | Index determinism. |

---

### Task 1: Per-card verdict persistence

**Files:**
- Modify: `code/tools/dcgo-harness/src/exam/verdict.rs`
- Test: inline `#[cfg(test)] mod tests` in the same file (the existing pattern)

**Interfaces:**
- Consumes: the existing `VerdictStore`, `ClauseVerdict`, `Verdict` types — unchanged.
- Produces:
  ```rust
  impl VerdictStore {
      /// Load every `<CARD-ID>.json` under `dir` and merge them.
      /// A missing directory yields an empty store (fresh checkout).
      pub fn load_dir(dir: &Path) -> Result<VerdictStore, String>;
      /// Write one file per card under `dir`, creating it if needed.
      /// Cards with no verdicts have their file removed.
      pub fn save_dir(&self, dir: &Path) -> Result<(), String>;
  }
  /// File name a card's verdicts live in: `"EX12-035"` -> `"EX12-035.json"`.
  pub fn card_file_name(card_id: &str) -> String;
  ```

- [ ] **Step 1: Write the failing tests**

Add to the existing `mod tests` block at the bottom of `verdict.rs` (around
line 448). **Reuse the `v(clause, verdict, sha)` helper already defined there** —
it derives `card_id` from the clause id, which is exactly the behaviour these
tests want, including the misfiled-row case where a `BT8-084` record is written
into `EX12-035.json`. Do not add a second helper.

```rust
    #[test]
    fn save_dir_writes_one_file_per_card() {
        let tmp = std::env::temp_dir().join("exam_verdicts_per_card");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut store = VerdictStore::default();
        store.record(v("EX12-035#effect#0", Verdict::Confirmed, "sha-ex12-035"));
        store.record(v("EX12-035#effect#1", Verdict::Unreachable, "sha-ex12-035"));
        store.record(v("BT8-084#effect#0", Verdict::Confirmed, "sha-bt8-084"));

        store.save_dir(&tmp).expect("save_dir");

        assert!(tmp.join("EX12-035.json").exists());
        assert!(tmp.join("BT8-084.json").exists());
        let files: Vec<_> = std::fs::read_dir(&tmp).unwrap().filter_map(|e| e.ok()).collect();
        assert_eq!(files.len(), 2, "one file per card, not per clause");
    }

    #[test]
    fn load_dir_round_trips_every_row() {
        let tmp = std::env::temp_dir().join("exam_verdicts_round_trip");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut store = VerdictStore::default();
        store.record(v("EX12-035#effect#0", Verdict::Confirmed, "sha-ex12-035"));
        store.record(v("BT8-084#effect#0", Verdict::Diverged, "sha-bt8-084"));
        store.save_dir(&tmp).expect("save_dir");

        let back = VerdictStore::load_dir(&tmp).expect("load_dir");
        assert_eq!(back.len(), 2);
        assert_eq!(back.get("EX12-035#effect#0").unwrap().verdict, Verdict::Confirmed);
        assert_eq!(back.get("BT8-084#effect#0").unwrap().verdict, Verdict::Diverged);
    }

    #[test]
    fn load_dir_missing_directory_is_empty_not_an_error() {
        let tmp = std::env::temp_dir().join("exam_verdicts_absent_dir");
        let _ = std::fs::remove_dir_all(&tmp);
        let store = VerdictStore::load_dir(&tmp).expect("missing dir is a fresh checkout");
        assert!(store.is_empty());
    }

    #[test]
    fn load_dir_rejects_a_row_filed_under_the_wrong_card() {
        let tmp = std::env::temp_dir().join("exam_verdicts_misfiled");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        // BT8-084's verdict written into EX12-035.json: a hand-edit or a bad
        // merge. Silently accepting it would file a verdict under a card that
        // never earned it.
        let mut store = VerdictStore::default();
        store.record(v("BT8-084#effect#0", Verdict::Confirmed, "sha-bt8-084"));
        std::fs::write(tmp.join("EX12-035.json"), store.to_json().unwrap()).unwrap();

        let err = VerdictStore::load_dir(&tmp).expect_err("must reject a misfiled row");
        assert!(err.contains("EX12-035"), "error names the file: {err}");
        assert!(err.contains("BT8-084"), "error names the offending card: {err}");
    }

    #[test]
    fn save_dir_removes_a_card_file_that_no_longer_has_verdicts() {
        let tmp = std::env::temp_dir().join("exam_verdicts_pruned");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut store = VerdictStore::default();
        store.record(v("EX12-035#effect#0", Verdict::Confirmed, "sha-ex12-035"));
        store.record(v("BT8-084#effect#0", Verdict::Confirmed, "sha-bt8-084"));
        store.save_dir(&tmp).unwrap();

        // A re-extraction dropped BT8-084 entirely.
        let mut store2 = VerdictStore::default();
        store2.record(v("EX12-035#effect#0", Verdict::Confirmed, "sha-ex12-035"));
        store2.save_dir(&tmp).unwrap();

        assert!(tmp.join("EX12-035.json").exists());
        assert!(!tmp.join("BT8-084.json").exists(), "stale card file must be pruned");
    }

    #[test]
    fn card_file_name_is_the_card_id_plus_json() {
        assert_eq!(card_file_name("EX12-035"), "EX12-035.json");
        assert_eq!(card_file_name("P-130"), "P-130.json");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p dcgo-harness --lib exam::verdict -- --nocapture`

Expected: FAIL — `no function or associated item named 'save_dir' found`, `cannot find function 'card_file_name'`.

- [ ] **Step 3: Implement directory persistence**

Add to `verdict.rs`, immediately after the existing `save` method inside `impl VerdictStore`:

```rust
    /// Load every `<CARD-ID>.json` under `dir` and merge them into one store.
    ///
    /// A **missing directory is not an error** — it is a fresh checkout, and
    /// every clause then honestly reports `unmeasured`. (Contrast [`load`],
    /// which takes an explicitly named file and must fail on a typo.)
    ///
    /// A row whose `card_id` does not match the file it was found in is
    /// rejected rather than merged: that is the per-card analogue of the
    /// key-vs-`clause_id` check in [`from_json`], and it catches a bad merge
    /// or a hand-edit that would otherwise file a verdict under a card that
    /// never earned it.
    pub fn load_dir(dir: &Path) -> Result<VerdictStore, String> {
        let mut merged = VerdictStore::default();
        if !dir.exists() {
            return Ok(merged);
        }
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("failed to read verdict directory {}: {e}", dir.display()))?;
        let mut paths: Vec<PathBuf> = Vec::new();
        for entry in entries {
            let entry =
                entry.map_err(|e| format!("failed to read {}: {e}", dir.display()))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                paths.push(path);
            }
        }
        // Deterministic order so an error is reproducible.
        paths.sort();

        for path in paths {
            let expected_card = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let one = VerdictStore::load(&path)?;
            for (clause_id, cv) in one.clauses.into_iter() {
                if cv.card_id != expected_card {
                    return Err(format!(
                        "verdict file {} holds a verdict for card {:?} (clause {:?}); \
                         each file holds exactly one card's verdicts",
                        path.display(),
                        cv.card_id,
                        clause_id
                    ));
                }
                merged.clauses.insert(clause_id, cv);
            }
            if one.last_updated > merged.last_updated {
                merged.last_updated = one.last_updated;
            }
        }
        Ok(merged)
    }

    /// Write one file per card under `dir`.
    ///
    /// Disjoint writers never touch the same file, which is what makes two
    /// nodes' branches merge cleanly. Card files that no longer have any
    /// verdicts are removed, so a re-extraction that drops a card cannot
    /// leave a stale verdict behind to be read back later.
    pub fn save_dir(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("failed to create verdict directory {}: {e}", dir.display()))?;

        let mut by_card: BTreeMap<String, VerdictStore> = BTreeMap::new();
        for (clause_id, cv) in self.clauses.iter() {
            let per = by_card.entry(cv.card_id.clone()).or_default();
            per.clauses.insert(clause_id.clone(), cv.clone());
            if cv.recorded_at > per.last_updated {
                per.last_updated = cv.recorded_at.clone();
            }
        }

        for (card_id, per) in by_card.iter() {
            per.save(&dir.join(card_file_name(card_id)))?;
        }

        // Prune files for cards we no longer carry.
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
                if !by_card.contains_key(&stem) {
                    std::fs::remove_file(&path).map_err(|e| {
                        format!("failed to prune stale verdict file {}: {e}", path.display())
                    })?;
                }
            }
        }
        Ok(())
    }
```

Add this free function next to `sha256_hex` near the bottom of the file:

```rust
/// File name a card's verdicts live in.
///
/// Card ids are already filesystem-safe (`[A-Z0-9-]`), so this is a plain
/// suffix rather than a sanitizer — if that ever stops being true, this is the
/// one place that has to learn about it.
pub fn card_file_name(card_id: &str) -> String {
    format!("{card_id}.json")
}
```

At the top of the file, widen the `std::path` import:

```rust
use std::path::{Path, PathBuf};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p dcgo-harness --lib exam::verdict`

Expected: PASS — all existing `verdict` tests plus the six new ones.

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/verdict.rs
git commit -m "exam: per-card verdict files, so two nodes never write the same file

The single blob was a merge-conflict magnet: any two nodes recording any two
clauses collided. Splitting persistence by card makes disjoint writers touch
disjoint files, which is the whole of the fleet-safety story for verdicts.

In-memory shape is unchanged -- load_dir/save_dir sit beside load/save, and a
row filed under the wrong card is rejected rather than merged, the per-card
analogue of the existing key-vs-clause_id check."
```

---

### Task 2: Migrate the existing store

**Files:**
- Modify: `code/tools/dcgo-harness/src/main.rs`
- Test: inline `#[cfg(test)] mod tests` in `code/tools/dcgo-harness/src/exam/verdict.rs`

**Interfaces:**
- Consumes: `VerdictStore::{load, save_dir}` from Task 1.
- Produces: CLI `dcgo-harness migrate-verdicts --from <blob.json> --to <dir>`; prints `migrated N verdicts across M cards`.

- [ ] **Step 1: Write the failing test**

Add to `mod tests` in `verdict.rs`:

```rust
    #[test]
    fn migration_from_the_blob_preserves_every_row() {
        let tmp = std::env::temp_dir().join("exam_verdicts_migration");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let mut blob = VerdictStore::default();
        blob.record(v("EX12-035#effect#0", Verdict::Confirmed, "sha-ex12-035"));
        blob.record(v("EX12-035#security#0", Verdict::Unreachable, "sha-ex12-035"));
        blob.record(v("BT8-084#effect#0", Verdict::Confirmed, "sha-bt8-084"));
        let blob_path = tmp.join("dcgo_exam_verdicts.json");
        blob.save(&blob_path).unwrap();

        let loaded = VerdictStore::load(&blob_path).unwrap();
        let dir = tmp.join("exam-verdicts");
        loaded.save_dir(&dir).unwrap();
        let back = VerdictStore::load_dir(&dir).unwrap();

        assert_eq!(back.len(), blob.len(), "no row may be lost in migration");
        for (clause_id, before) in blob.iter() {
            let after = back.get(clause_id).expect("every clause survives");
            assert_eq!(after.verdict, before.verdict);
            assert_eq!(after.text_sha256, before.text_sha256);
            assert_eq!(after.recorded_at, before.recorded_at);
        }
    }
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p dcgo-harness --lib exam::verdict::tests::migration_from_the_blob_preserves_every_row`

Expected: FAIL — the test compiles only after Task 1; if Task 1 is done it should PASS immediately, which is the point: migration is a composition of two verified primitives, so the test pins that composition before the CLI wraps it.

If it passes at this step, proceed — the CLI in Step 3 is still required.

- [ ] **Step 3: Add the CLI subcommand**

In `main.rs`, `Commands` is a FLAT enum (`Submit`, `Exam`, `Build`, `Up`, `Down`, `Watch`, ...). Add a sibling variant, which clap renders as the top-level `migrate-verdicts` subcommand:

```rust
    /// Split a single-blob verdict store into per-card files (one-time).
    MigrateVerdicts {
        /// The existing single-file store.
        #[arg(long, default_value = "qa/qa-reports/dcgo_exam_verdicts.json")]
        from: PathBuf,
        /// Destination directory for per-card files.
        #[arg(long, default_value = "qa/qa-reports/exam-verdicts")]
        to: PathBuf,
    },
```

And in the matching `match` arm block:

```rust
        Commands::MigrateVerdicts { from, to } => {
            let store = exam::verdict::VerdictStore::load(&from)?;
            let cards: std::collections::BTreeSet<String> =
                store.iter().map(|(_, cv)| cv.card_id.clone()).collect();
            let rows = store.len();
            store.save_dir(&to)?;
            println!(
                "migrated {rows} verdicts across {} cards: {} -> {}",
                cards.len(),
                from.display(),
                to.display()
            );
            Ok(())
        }
```

- [ ] **Step 4: Run the migration for real and verify the counts**

```bash
cargo run -p dcgo-harness -- migrate-verdicts
```

Expected: `migrated 148 verdicts across 35 cards: qa/qa-reports/dcgo_exam_verdicts.json -> qa/qa-reports/exam-verdicts`

Verify independently, and confirm the two counts agree:

```bash
python -c "
import json,glob,collections
old=json.load(open('qa/qa-reports/dcgo_exam_verdicts.json'))['clauses']
new={}
for f in glob.glob('qa/qa-reports/exam-verdicts/*.json'):
    new.update(json.load(open(f))['clauses'])
print('old',len(old),'new',len(new),'identical' if old.keys()==new.keys() else 'MISMATCH')
print(collections.Counter(v['verdict'] for v in new.values()))
"
```

Expected: `old 148 new 148 identical` and `Counter({'confirmed': 129, 'unreachable': 19})`.

**If the counts disagree, stop.** Do not delete the blob. Report the mismatch.

- [ ] **Step 5: Remove the blob and repoint the default**

```bash
git rm qa/qa-reports/dcgo_exam_verdicts.json
```

In `main.rs`, change the `--verdicts` argument's `default_missing_value` (around line 117) from `"qa/qa-reports/dcgo_exam_verdicts.json"` to `"qa/qa-reports/exam-verdicts"`, and change the code that loads it from `VerdictStore::load(&path)` to `VerdictStore::load_dir(&path)`.

- [ ] **Step 6: Run the full harness suite**

Run: `cargo test -p dcgo-harness`

Expected: PASS, no test still referencing the removed blob.

- [ ] **Step 7: Commit**

```bash
git add code/tools/dcgo-harness/src/main.rs code/tools/dcgo-harness/src/exam/verdict.rs qa/qa-reports/exam-verdicts/
git commit -m "exam: migrate the verdict blob to per-card files (148 rows, 35 cards)

Counts verified identical before the blob was removed: 148 in, 148 out, same
clause ids, 129 confirmed / 19 unreachable either side."
```

---

### Task 3: Python binding reads a directory

**Files:**
- Modify: `code/tools/clause_coverage/exam_binding.py:155-172`
- Test: `code/tests/tools/test_clause_coverage_exam_binding.py`

**Interfaces:**
- Consumes: the per-card layout from Task 1.
- Produces: `load_verdict_store(path)` unchanged in signature — now accepts a **file or a directory**, returning the same `{clause_id: entry}` dict, so `bind()` and every caller are untouched.

- [ ] **Step 1: Write the failing tests**

Append to `code/tests/tools/test_clause_coverage_exam_binding.py`:

```python
def test_load_verdict_store_reads_a_directory_of_per_card_files(tmp_path):
    """The fleet layout: one file per card, merged on read."""
    d = tmp_path / "exam-verdicts"
    d.mkdir()
    (d / "EX12-073.json").write_text(
        json.dumps(
            {
                "version": 1,
                "clauses": {
                    "EX12-073#effect#0": {
                        "clause_id": "EX12-073#effect#0",
                        "card_id": "EX12-073",
                        "verdict": "confirmed",
                        "text_sha256": "abc",
                    }
                },
            }
        ),
        encoding="utf-8",
    )
    (d / "BT8-084.json").write_text(
        json.dumps(
            {
                "version": 1,
                "clauses": {
                    "BT8-084#effect#0": {
                        "clause_id": "BT8-084#effect#0",
                        "card_id": "BT8-084",
                        "verdict": "unreachable",
                        "text_sha256": "def",
                    }
                },
            }
        ),
        encoding="utf-8",
    )

    store = load_verdict_store(d)

    assert set(store) == {"EX12-073#effect#0", "BT8-084#effect#0"}
    assert store["EX12-073#effect#0"]["verdict"] == "confirmed"


def test_load_verdict_store_missing_directory_is_empty(tmp_path):
    """A fresh checkout has no ledger; everything is honestly unmeasured."""
    assert load_verdict_store(tmp_path / "does-not-exist") == {}


def test_load_verdict_store_still_reads_a_single_file(tmp_path):
    """The single-file form stays supported: tests and fixtures use it."""
    p = tmp_path / "verdicts.json"
    p.write_text(
        json.dumps(
            {
                "version": 1,
                "clauses": {
                    "EX12-073#effect#0": {
                        "clause_id": "EX12-073#effect#0",
                        "card_id": "EX12-073",
                        "verdict": "confirmed",
                        "text_sha256": "abc",
                    }
                },
            }
        ),
        encoding="utf-8",
    )
    assert set(load_verdict_store(p)) == {"EX12-073#effect#0"}
```

Ensure `json` and `load_verdict_store` are imported at the top of that test file; add them if absent:

```python
import json

from tools.clause_coverage.exam_binding import bind, load_verdict_store
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest code/tests/tools/test_clause_coverage_exam_binding.py -k directory -v`

Expected: FAIL — `load_verdict_store` returns `{}` for a directory because `json.load` on a directory raises `IsADirectoryError`, or the existing `p.exists()` branch yields `{}` silently.

- [ ] **Step 3: Implement**

Replace the body of `load_verdict_store` in `code/tools/clause_coverage/exam_binding.py`:

```python
def load_verdict_store(path: Path | str | None) -> dict:
    """Load the verdict store -> ``{clause_id: entry}``.

    Accepts either the **fleet layout** -- a directory of per-card
    ``<CARD-ID>.json`` files, which is what nodes write, because disjoint
    writers must touch disjoint files -- or a **single file**, which fixtures
    and tests still use.

    A missing path is NOT an error (fresh checkout): it yields an empty store,
    and every clause then honestly reports `unmeasured`.
    """
    if not path:
        return {}
    p = Path(path)
    if not p.exists():
        return {}

    if p.is_dir():
        merged: dict = {}
        for f in sorted(p.glob("*.json")):
            merged.update(_load_verdict_file(f))
        return merged

    return _load_verdict_file(p)


def _load_verdict_file(p: Path) -> dict:
    """One store file -> ``{clause_id: entry}``. Shape errors yield ``{}``."""
    with open(p, encoding="utf-8") as f:
        data = json.load(f)
    clauses = data.get("clauses") if isinstance(data, dict) else None
    if not isinstance(clauses, dict):
        return {}
    return clauses
```

- [ ] **Step 4: Run the tests**

Run: `python -m pytest code/tests/tools/test_clause_coverage_exam_binding.py -v`

Expected: PASS — the three new tests and every pre-existing one.

- [ ] **Step 5: Commit**

```bash
git add code/tools/clause_coverage/exam_binding.py code/tests/tools/test_clause_coverage_exam_binding.py
git commit -m "clause_coverage: bind() reads the per-card verdict directory

Signature is unchanged, so bind() and the CI gate are untouched; the single-file
form stays supported for fixtures."
```

---

### Task 4: The append-only attempt log

**Files:**
- Create: `code/tools/dcgo-harness/src/exam/ledger.rs`
- Modify: `code/tools/dcgo-harness/src/exam/mod.rs`
- Modify: `.gitattributes`
- Test: inline `#[cfg(test)] mod tests` in `ledger.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks (deliberately — this task can start in parallel with Task 1).
- Produces:
  ```rust
  pub struct Attempt {
      pub ts: String, pub job_id: String, pub node: String,
      pub archetype: String, pub card: String, pub clause: String,
      pub verdict_before: String, pub verdict_after: String,
      pub scenario: Option<String>, pub dcgo_build: Option<String>,
      pub outcome: String,
  }
  pub fn append_attempt(path: &Path, a: &Attempt) -> Result<(), String>;
  pub fn read_attempts(path: &Path) -> Result<Vec<Attempt>, String>;
  pub const DEFAULT_LOG: &str = "qa/qa-reports/exam-log.jsonl";
  ```

- [ ] **Step 1: Write the failing tests**

Create `code/tools/dcgo-harness/src/exam/ledger.rs` containing only this test module for now (the types come in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(clause: &str, after: &str) -> Attempt {
        Attempt {
            ts: "2026-08-27T12:00:00+00:00".to_string(),
            job_id: "hunters-01".to_string(),
            node: "oracle-a".to_string(),
            archetype: "Hunters".to_string(),
            card: clause.split('#').next().unwrap().to_string(),
            clause: clause.to_string(),
            verdict_before: "unmeasured".to_string(),
            verdict_after: after.to_string(),
            scenario: Some("qa/dcgo-exams/BT12/x.yaml".to_string()),
            dcgo_build: Some("638f4070".to_string()),
            outcome: "oracle_clean".to_string(),
        }
    }

    #[test]
    fn append_then_read_round_trips() {
        let tmp = std::env::temp_dir().join("exam_log_round_trip.jsonl");
        let _ = std::fs::remove_file(&tmp);
        append_attempt(&tmp, &attempt("BT12-042#effect#0", "confirmed")).unwrap();
        append_attempt(&tmp, &attempt("BT12-043#effect#0", "diverged")).unwrap();

        let back = read_attempts(&tmp).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].clause, "BT12-042#effect#0");
        assert_eq!(back[1].verdict_after, "diverged");
    }

    #[test]
    fn each_attempt_is_exactly_one_line() {
        // Line-orientation is what makes a union merge driver correct. A
        // pretty-printed record would merge into unparseable interleaving.
        let tmp = std::env::temp_dir().join("exam_log_one_line.jsonl");
        let _ = std::fs::remove_file(&tmp);
        append_attempt(&tmp, &attempt("BT12-042#effect#0", "confirmed")).unwrap();
        append_attempt(&tmp, &attempt("BT12-043#effect#0", "confirmed")).unwrap();

        let text = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.ends_with('\n'), "trailing newline keeps appends clean");
        for line in text.lines() {
            serde_json::from_str::<Attempt>(line).expect("each line parses alone");
        }
    }

    #[test]
    fn append_creates_parent_directories() {
        let tmp = std::env::temp_dir()
            .join("exam_log_nested")
            .join("deeper")
            .join("exam-log.jsonl");
        let _ = std::fs::remove_dir_all(tmp.parent().unwrap().parent().unwrap());
        append_attempt(&tmp, &attempt("BT12-042#effect#0", "confirmed")).unwrap();
        assert!(tmp.exists());
    }

    #[test]
    fn read_attempts_missing_file_is_empty_not_an_error() {
        let tmp = std::env::temp_dir().join("exam_log_absent.jsonl");
        let _ = std::fs::remove_file(&tmp);
        assert!(read_attempts(&tmp).unwrap().is_empty());
    }

    #[test]
    fn read_attempts_names_the_line_it_could_not_parse() {
        let tmp = std::env::temp_dir().join("exam_log_corrupt.jsonl");
        let _ = std::fs::remove_file(&tmp);
        append_attempt(&tmp, &attempt("BT12-042#effect#0", "confirmed")).unwrap();
        let mut text = std::fs::read_to_string(&tmp).unwrap();
        text.push_str("{not json}\n");
        std::fs::write(&tmp, text).unwrap();

        let err = read_attempts(&tmp).expect_err("a corrupt line must be reported");
        assert!(err.contains("line 2"), "error names the line: {err}");
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p dcgo-harness --lib exam::ledger`

Expected: FAIL — `cannot find type 'Attempt' in this scope`.

- [ ] **Step 3: Implement the log**

Prepend to `ledger.rs`, above the test module:

```rust
//! The fleet ledger's two mutable halves: the **attempt log** and **claims**.
//!
//! [`verdict::VerdictStore`](super::verdict::VerdictStore) records *current
//! state* — what each clause's verdict is now. This module records the two
//! things that state cannot express:
//!
//! - **History.** `unmeasured` cannot distinguish "nobody has looked" from
//!   "three nodes each burned an afternoon discovering the same dead end".
//!   The attempt log can, and that difference is the whole point of a ledger
//!   shared between nodes.
//! - **Intent.** A claim says a node is working on a card *right now*, so a
//!   second node picks something else.
//!
//! Both are shaped by how they merge. The log is one JSON object per line
//! under a git union merge driver, so concurrent nodes concatenate rather than
//! conflict. Claims are one small file per card, so disjoint claimers touch
//! disjoint files.

use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Default location of the attempt log, relative to the repo root.
pub const DEFAULT_LOG: &str = "qa/qa-reports/exam-log.jsonl";

/// One recorded attempt at one clause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    /// RFC3339 UTC.
    pub ts: String,
    pub job_id: String,
    /// Which node ran it — the operator-facing name, not a hostname UUID.
    pub node: String,
    pub archetype: String,
    pub card: String,
    /// `{card_id}#{zone}#{idx}` — a `clause_coverage` clause id.
    pub clause: String,
    pub verdict_before: String,
    pub verdict_after: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dcgo_build: Option<String>,
    /// Free-form but conventional: `oracle_clean`, `oracle_diverged`,
    /// `sim_failed`, `unreachable`, `abandoned`.
    pub outcome: String,
}

/// Append one attempt as a single line, creating the file and its parents.
///
/// Serialized with `to_string` (never `to_string_pretty`): one record per line
/// is precisely what makes the union merge driver correct.
pub fn append_attempt(path: &Path, a: &Attempt) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create {} for the exam log: {e}", parent.display()))?;
        }
    }
    let mut line = serde_json::to_string(a)
        .map_err(|e| format!("failed to serialize attempt: {e}"))?;
    line.push('\n');

    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("failed to open exam log {}: {e}", path.display()))?;
    f.write_all(line.as_bytes())
        .map_err(|e| format!("failed to append to exam log {}: {e}", path.display()))
}

/// Read every attempt. A missing file is an empty history, not an error.
pub fn read_attempts(path: &Path) -> Result<Vec<Attempt>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read exam log {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let a: Attempt = serde_json::from_str(line).map_err(|e| {
            format!(
                "exam log {} line {}: {e}",
                path.display(),
                i + 1
            )
        })?;
        out.push(a);
    }
    Ok(out)
}
```

Register the module in `code/tools/dcgo-harness/src/exam/mod.rs`, keeping the existing alphabetical order of the `pub mod` block:

```rust
pub mod ledger;
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p dcgo-harness --lib exam::ledger`

Expected: PASS — five tests.

- [ ] **Step 5: Add the union merge driver**

Append to `.gitattributes` at the repo root (create the file if it does not exist):

```
# The exam attempt log is append-only history from parallel oracle nodes.
# Union merge concatenates both sides instead of conflicting; the log is
# one JSON object per line precisely so this is correct.
qa/qa-reports/exam-log.jsonl merge=union
```

- [ ] **Step 6: Verify the union merge actually works**

This is the one behavior a unit test cannot prove, so verify it against real git:

```bash
cd "$(mktemp -d)" && git init -q . && \
  printf 'qa.jsonl merge=union\n' > .gitattributes && \
  printf '{"a":1}\n' > qa.jsonl && git add -A && git -c user.email=t@t -c user.name=t commit -qm base && \
  git checkout -qb nodeA && printf '{"a":1}\n{"b":2}\n' > qa.jsonl && git -c user.email=t@t -c user.name=t commit -qam A && \
  git checkout -q master 2>/dev/null || git checkout -q main; \
  git checkout -qb nodeB master 2>/dev/null || git checkout -qb nodeB main; \
  printf '{"a":1}\n{"c":3}\n' > qa.jsonl && git -c user.email=t@t -c user.name=t commit -qam B && \
  git -c user.email=t@t -c user.name=t merge -q nodeA -m merge && cat qa.jsonl
```

Expected: three lines — `{"a":1}`, `{"c":3}`, `{"b":2}` — and **no conflict markers**. Both nodes' attempts survive.

- [ ] **Step 7: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/ledger.rs code/tools/dcgo-harness/src/exam/mod.rs .gitattributes
git commit -m "exam: append-only attempt log under a union merge driver

The verdict store says what a clause IS; it cannot say what was already tried
and abandoned. Without that, three nodes can each spend an afternoon
rediscovering the same dead end and the store looks identical afterwards.

One JSON object per line is not a style choice -- it is what makes merge=union
correct, so concurrent nodes concatenate instead of conflicting."
```

---

### Task 5: Advisory claims

**Files:**
- Modify: `code/tools/dcgo-harness/src/exam/ledger.rs`
- Test: inline, same file

**Interfaces:**
- Consumes: nothing.
- Produces:
  ```rust
  pub struct Claim {
      pub job_id: String, pub node: String, pub archetype: String,
      pub claimed_at: String, pub expires_at: String,
  }
  pub struct ClaimOutcome { pub granted: Vec<String>, pub held_by_others: Vec<(String, Claim)> }
  pub fn claim_cards(dir: &Path, cards: &[String], c: &Claim, now: &str) -> Result<ClaimOutcome, String>;
  pub fn release_cards(dir: &Path, cards: &[String], job_id: &str) -> Result<usize, String>;
  pub fn read_claims(dir: &Path, now: &str) -> Result<BTreeMap<String, Claim>, String>;
  pub const DEFAULT_CLAIMS: &str = "qa/qa-reports/exam-claims";
  ```
  `now` is passed in rather than read from the clock so expiry is testable without sleeping.

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `ledger.rs`:

```rust
    fn claim(job: &str, expires: &str) -> Claim {
        Claim {
            job_id: job.to_string(),
            node: "oracle-a".to_string(),
            archetype: "Hunters".to_string(),
            claimed_at: "2026-08-27T12:00:00+00:00".to_string(),
            expires_at: expires.to_string(),
        }
    }

    #[test]
    fn claiming_free_cards_grants_all_of_them() {
        let dir = std::env::temp_dir().join("exam_claims_free");
        let _ = std::fs::remove_dir_all(&dir);
        let cards = vec!["BT12-042".to_string(), "BT12-043".to_string()];

        let out = claim_cards(&dir, &cards, &claim("hunters-01", "2026-08-28T12:00:00+00:00"),
                              "2026-08-27T12:00:00+00:00").unwrap();

        assert_eq!(out.granted, cards);
        assert!(out.held_by_others.is_empty());
        assert!(dir.join("BT12-042.claim").exists());
    }

    #[test]
    fn a_card_held_by_another_live_job_is_not_granted() {
        // The overlap case that matters: Beelstar's EX7 cards are a strict
        // subset of Three Musketeers'. Two archetype jobs WILL ask for the
        // same card.
        let dir = std::env::temp_dir().join("exam_claims_contended");
        let _ = std::fs::remove_dir_all(&dir);
        let now = "2026-08-27T12:00:00+00:00";
        claim_cards(&dir, &["EX7-005".to_string()], &claim("musketeers-01", "2026-08-28T12:00:00+00:00"), now).unwrap();

        let out = claim_cards(
            &dir,
            &["EX7-005".to_string(), "EX7-008".to_string()],
            &claim("beelstar-01", "2026-08-28T12:00:00+00:00"),
            now,
        )
        .unwrap();

        assert_eq!(out.granted, vec!["EX7-008".to_string()]);
        assert_eq!(out.held_by_others.len(), 1);
        assert_eq!(out.held_by_others[0].0, "EX7-005");
        assert_eq!(out.held_by_others[0].1.job_id, "musketeers-01");
    }

    #[test]
    fn an_expired_claim_does_not_block() {
        // A node that crashed must not park a card forever.
        let dir = std::env::temp_dir().join("exam_claims_expired");
        let _ = std::fs::remove_dir_all(&dir);
        claim_cards(&dir, &["BT12-042".to_string()],
                    &claim("dead-job", "2026-08-27T00:00:00+00:00"),
                    "2026-08-26T12:00:00+00:00").unwrap();

        let out = claim_cards(&dir, &["BT12-042".to_string()],
                              &claim("live-job", "2026-08-29T00:00:00+00:00"),
                              "2026-08-28T12:00:00+00:00").unwrap();

        assert_eq!(out.granted, vec!["BT12-042".to_string()]);
        assert!(out.held_by_others.is_empty());
    }

    #[test]
    fn reclaiming_your_own_card_is_idempotent() {
        // Resuming a crashed job must not deadlock against itself.
        let dir = std::env::temp_dir().join("exam_claims_reentrant");
        let _ = std::fs::remove_dir_all(&dir);
        let now = "2026-08-27T12:00:00+00:00";
        let c = claim("hunters-01", "2026-08-28T12:00:00+00:00");
        claim_cards(&dir, &["BT12-042".to_string()], &c, now).unwrap();

        let out = claim_cards(&dir, &["BT12-042".to_string()], &c, now).unwrap();

        assert_eq!(out.granted, vec!["BT12-042".to_string()]);
        assert!(out.held_by_others.is_empty());
    }

    #[test]
    fn release_only_removes_your_own_claims() {
        let dir = std::env::temp_dir().join("exam_claims_release");
        let _ = std::fs::remove_dir_all(&dir);
        let now = "2026-08-27T12:00:00+00:00";
        claim_cards(&dir, &["EX7-005".to_string()], &claim("musketeers-01", "2026-08-28T12:00:00+00:00"), now).unwrap();
        claim_cards(&dir, &["EX7-008".to_string()], &claim("beelstar-01", "2026-08-28T12:00:00+00:00"), now).unwrap();

        let removed = release_cards(&dir, &["EX7-005".to_string(), "EX7-008".to_string()], "beelstar-01").unwrap();

        assert_eq!(removed, 1, "only beelstar-01's claim is released");
        assert!(dir.join("EX7-005.claim").exists(), "another job's claim survives");
        assert!(!dir.join("EX7-008.claim").exists());
    }

    #[test]
    fn read_claims_hides_expired_ones() {
        let dir = std::env::temp_dir().join("exam_claims_read");
        let _ = std::fs::remove_dir_all(&dir);
        claim_cards(&dir, &["A-1".to_string()], &claim("live", "2026-08-29T00:00:00+00:00"),
                    "2026-08-27T12:00:00+00:00").unwrap();
        claim_cards(&dir, &["B-2".to_string()], &claim("dead", "2026-08-27T00:00:00+00:00"),
                    "2026-08-26T12:00:00+00:00").unwrap();

        let live = read_claims(&dir, "2026-08-28T12:00:00+00:00").unwrap();

        assert_eq!(live.keys().collect::<Vec<_>>(), vec!["A-1"]);
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p dcgo-harness --lib exam::ledger`

Expected: FAIL — `cannot find type 'Claim' in this scope`.

- [ ] **Step 3: Implement claims**

Add to `ledger.rs` above the test module. Add `use std::collections::BTreeMap;` to the imports at the top of the file:

```rust
/// Default claim directory, relative to the repo root.
pub const DEFAULT_CLAIMS: &str = "qa/qa-reports/exam-claims";

/// An advisory lease on one card.
///
/// **Advisory, deliberately.** Two nodes pushing in the same instant can both
/// claim a card: git is the only coordinator here. That is an accepted
/// trade — at roughly $8 of agent time per authored clause, an occasional
/// duplicate costs far less than a lease server, and the duplicate is
/// *detectable* at merge (two verdicts for one clause, normally agreeing).
/// If this ever starts hurting, the MCP is where a real lease would go.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub job_id: String,
    pub node: String,
    pub archetype: String,
    /// RFC3339 UTC.
    pub claimed_at: String,
    /// RFC3339 UTC. Past this, the claim is ignored — a crashed node must not
    /// park a card forever.
    pub expires_at: String,
}

/// What a [`claim_cards`] call actually got.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimOutcome {
    /// Cards this job may now work on.
    pub granted: Vec<String>,
    /// Cards a *different* live job holds, with that claim, so the caller can
    /// say who has it rather than just skipping silently.
    pub held_by_others: Vec<(String, Claim)>,
}

fn claim_file_name(card_id: &str) -> String {
    format!("{card_id}.claim")
}

/// RFC3339 UTC strings sort lexicographically iff they share an offset. Every
/// writer here emits `+00:00`, so string comparison is the expiry check and no
/// date library is needed.
fn is_live(c: &Claim, now: &str) -> bool {
    c.expires_at.as_str() > now
}

/// Claim `cards` for one job. Cards held by a different live job are reported,
/// not taken. Re-claiming your own card is idempotent, so a resumed job does
/// not deadlock against itself.
pub fn claim_cards(
    dir: &Path,
    cards: &[String],
    c: &Claim,
    now: &str,
) -> Result<ClaimOutcome, String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("failed to create claim directory {}: {e}", dir.display()))?;

    let mut granted = Vec::new();
    let mut held_by_others = Vec::new();

    for card in cards {
        let path = dir.join(claim_file_name(card));
        if path.exists() {
            let existing = read_claim_file(&path)?;
            if is_live(&existing, now) && existing.job_id != c.job_id {
                held_by_others.push((card.clone(), existing));
                continue;
            }
        }
        let text = serde_json::to_string_pretty(c)
            .map_err(|e| format!("failed to serialize claim: {e}"))?;
        std::fs::write(&path, format!("{text}\n"))
            .map_err(|e| format!("failed to write claim {}: {e}", path.display()))?;
        granted.push(card.clone());
    }

    Ok(ClaimOutcome {
        granted,
        held_by_others,
    })
}

/// Release this job's claims on `cards`. Returns how many were removed.
/// Another job's claim is never removed — releasing is not a stealing primitive.
pub fn release_cards(dir: &Path, cards: &[String], job_id: &str) -> Result<usize, String> {
    let mut removed = 0;
    for card in cards {
        let path = dir.join(claim_file_name(card));
        if !path.exists() {
            continue;
        }
        let existing = read_claim_file(&path)?;
        if existing.job_id != job_id {
            continue;
        }
        std::fs::remove_file(&path)
            .map_err(|e| format!("failed to remove claim {}: {e}", path.display()))?;
        removed += 1;
    }
    Ok(removed)
}

/// Every **live** claim, keyed by card id. Expired claims are omitted.
/// A missing directory is no claims, not an error.
pub fn read_claims(dir: &Path, now: &str) -> Result<BTreeMap<String, Claim>, String> {
    let mut out = BTreeMap::new();
    if !dir.exists() {
        return Ok(out);
    }
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("failed to read claim directory {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read {}: {e}", dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("claim") {
            continue;
        }
        let card = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let c = read_claim_file(&path)?;
        if is_live(&c, now) {
            out.insert(card, c);
        }
    }
    Ok(out)
}

fn read_claim_file(path: &Path) -> Result<Claim, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read claim {}: {e}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|e| format!("invalid claim file {}: {e}", path.display()))
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p dcgo-harness --lib exam::ledger`

Expected: PASS — eleven tests (five log + six claim).

- [ ] **Step 5: Ignore claim files in git status noise, but track them**

Claims must be **committed** — that is how other nodes see them. Confirm nothing ignores them:

```bash
git check-ignore -v qa/qa-reports/exam-claims/ || echo "not ignored (correct)"
```

Expected: `not ignored (correct)`.

- [ ] **Step 6: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/ledger.rs
git commit -m "exam: advisory per-card claims with expiry

Card granularity, not archetype: pools genuinely overlap -- Beelstar's missing
EX7 cards are a strict subset of Three Musketeers' -- so an archetype-level
claim would either block legitimate work or duplicate the shared cards.

Advisory on purpose. Two nodes pushing at the same instant can both claim; at
~\$8 per authored clause that costs less than a coordination server, and the
duplicate is detectable at merge. Expiry means a crashed node cannot park a
card forever, and re-claiming your own card is idempotent so a resumed job does
not deadlock against itself."
```

---

### Task 6: The generated index

**Files:**
- Create: `code/tools/clause_coverage/exam_index.py`
- Create: `code/tests/tools/test_exam_index.py`

**Interfaces:**
- Consumes: `exam_binding.bind(card_ids, scenarios_dir, verdicts_path)` (Task 3 makes it directory-aware); `exam-log.jsonl` from Task 4.
- Produces:
  ```python
  def render_index(rows: list[dict], generated_from: str) -> str
  # rows: [{"archetype": str, "cards": list[str], "binding": dict}]
  ```
  plus `python -m tools.clause_coverage.exam_index --out qa/qa-reports/exam-index.md`.

- [ ] **Step 1: Write the failing test**

Create `code/tests/tools/test_exam_index.py`:

```python
"""The exam index is generated, never hand-edited.

A hand-edit must therefore be a failing build rather than silent drift, which
means rendering has to be deterministic: same ledger in, byte-identical
Markdown out.
"""

from tools.clause_coverage.exam_index import render_index


def _binding(total, confirmed, diverged, unreachable, unavailable, unmeasured):
    return {
        "total_clauses": total,
        "denominator": {
            "by_verdict": {
                "confirmed": confirmed,
                "diverged": diverged,
                "unreachable": unreachable,
                "unavailable": unavailable,
                "unmeasured": unmeasured,
            }
        },
    }


def test_render_is_deterministic():
    rows = [
        {"archetype": "Toho Braves", "cards": ["EX12-035"],
         "binding": _binding(166, 107, 0, 5, 0, 54)},
        {"archetype": "Hunters", "cards": ["BT12-042"],
         "binding": _binding(65, 0, 0, 0, 0, 65)},
    ]
    assert render_index(rows, "ledger") == render_index(rows, "ledger")


def test_every_row_prints_all_five_classes():
    """A card must never read as 'passed' on a partial denominator."""
    rows = [{"archetype": "Toho Braves", "cards": ["EX12-035"],
             "binding": _binding(166, 107, 0, 5, 0, 54)}]
    out = render_index(rows, "ledger")
    for column in ("confirmed", "diverged", "unreachable", "unavailable", "unmeasured"):
        assert column in out
    assert "107" in out and "54" in out


def test_counts_that_do_not_sum_to_the_denominator_are_rejected():
    """by_verdict summing to total is an invariant of bind(); if it ever
    breaks, the index must refuse rather than publish a lie."""
    rows = [{"archetype": "Broken", "cards": ["X-1"],
             "binding": _binding(10, 1, 0, 0, 0, 0)}]
    try:
        render_index(rows, "ledger")
    except ValueError as e:
        assert "sum" in str(e).lower()
    else:
        raise AssertionError("must reject counts that do not sum to the denominator")


def test_archetypes_sort_by_unmeasured_descending():
    """The index exists to answer 'what should I dispatch next'."""
    rows = [
        {"archetype": "Nearly Done", "cards": ["A-1"], "binding": _binding(10, 9, 0, 0, 0, 1)},
        {"archetype": "Untouched", "cards": ["B-1"], "binding": _binding(10, 0, 0, 0, 0, 10)},
    ]
    out = render_index(rows, "ledger")
    assert out.index("Untouched") < out.index("Nearly Done")
```

- [ ] **Step 2: Run to verify failure**

Run: `python -m pytest code/tests/tools/test_exam_index.py -v`

Expected: FAIL — `ModuleNotFoundError: No module named 'tools.clause_coverage.exam_index'`.

- [ ] **Step 3: Implement the renderer**

Create `code/tools/clause_coverage/exam_index.py`:

```python
"""Render the human-facing exam index from the ledger.

This file is **generated**. `qa/qa-reports/exam-index.md` is regenerated from
the per-card verdict files, the scenario corpus and the clause denominator, and
a test asserts the rendering is deterministic -- so a hand-edit shows up as a
failing build rather than as silent drift.

It answers one question: *what should I dispatch next?* Hence the sort by
`unmeasured` descending -- the archetypes with the most unproven clauses first.

Standard library only, matching the rest of `tools/clause_coverage/`.
"""

from __future__ import annotations

import argparse
from pathlib import Path

VERDICT_COLUMNS = (
    "confirmed",
    "diverged",
    "unreachable",
    "unavailable",
    "unmeasured",
)


def render_index(rows: list[dict], generated_from: str) -> str:
    """Render the index.

    `rows` is ``[{"archetype": str, "cards": [card_id], "binding": bind_result}]``.

    Raises ``ValueError`` if a row's five verdict counts do not sum to its
    denominator. That sum is an invariant of ``exam_binding.bind()`` (one class
    is appended per clause in a single loop); if it is ever violated the index
    must refuse rather than publish a total nobody can trust.
    """
    for row in rows:
        binding = row["binding"]
        by_verdict = binding["denominator"]["by_verdict"]
        total = binding["total_clauses"]
        got = sum(by_verdict.get(k, 0) for k in VERDICT_COLUMNS)
        if got != total:
            raise ValueError(
                f"{row['archetype']}: verdict counts sum to {got}, "
                f"denominator is {total} -- refusing to render"
            )

    ordered = sorted(
        rows,
        key=lambda r: (
            -r["binding"]["denominator"]["by_verdict"].get("unmeasured", 0),
            r["archetype"],
        ),
    )

    out: list[str] = []
    out.append("# DCGO exam index")
    out.append("")
    out.append(
        "**Generated — do not hand-edit.** Regenerate with "
        "`python -m tools.clause_coverage.exam_index`."
    )
    out.append("")
    out.append(
        "Every row prints the full denominator. An archetype is never "
        '"passed"; it is a count per verdict class, and `unmeasured` is as '
        "real an outcome as `confirmed`."
    )
    out.append("")
    out.append(f"Source: {generated_from}")
    out.append("")
    out.append(
        "| Archetype | Cards | Clauses | "
        + " | ".join(c.capitalize() for c in VERDICT_COLUMNS)
        + " | Measured |"
    )
    out.append("|---|---|---|" + "---|" * (len(VERDICT_COLUMNS) + 1))

    for row in ordered:
        binding = row["binding"]
        by_verdict = binding["denominator"]["by_verdict"]
        total = binding["total_clauses"]
        measured = total - by_verdict.get("unmeasured", 0)
        pct = f"{(100 * measured / total):.0f}%" if total else "n/a"
        cells = " | ".join(str(by_verdict.get(c, 0)) for c in VERDICT_COLUMNS)
        out.append(
            f"| {row['archetype']} | {len(row['cards'])} | {total} | {cells} | {pct} |"
        )

    out.append("")
    return "\n".join(out)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--out",
        type=Path,
        default=Path("qa/qa-reports/exam-index.md"),
        help="where to write the index",
    )
    parser.add_argument(
        "--verdicts",
        type=Path,
        default=Path("qa/qa-reports/exam-verdicts"),
        help="per-card verdict directory",
    )
    args = parser.parse_args(argv)

    # Archetype -> card list resolution lands with the campaign skill (plan 4).
    # Until then the index renders whatever rows a caller supplies; this
    # entrypoint writes an empty index rather than inventing an archetype map.
    text = render_index([], generated_from=str(args.verdicts))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(text, encoding="utf-8")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run the tests**

Run: `python -m pytest code/tests/tools/test_exam_index.py -v`

Expected: PASS — four tests.

- [ ] **Step 5: Run the whole clause_coverage suite for regressions**

Run: `python -m pytest code/tests/tools -v`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add code/tools/clause_coverage/exam_index.py code/tests/tools/test_exam_index.py
git commit -m "exam: generated index, sorted by what is least proven

Answers one question -- what should I dispatch next -- so it sorts by unmeasured
descending. Rendering is deterministic and a row whose five counts do not sum to
its denominator is refused rather than published: bind() guarantees that sum, and
an index that quietly prints a total nobody can trust is worse than no index."
```

---

### Task 7: Documentation

**Files:**
- Modify: `docs/DCGO_EXAM.md`
- Modify: `.claude/skills/dcgo-exam/SKILL.md`
- Modify: `CLAUDE.md:406`

**Interfaces:**
- Consumes: everything above. Produces no code.

- [ ] **Step 1: Update every reference to the old blob path**

Find them:

```bash
grep -rn 'dcgo_exam_verdicts' --include=*.md --include=*.yml . | grep -v docs/superpowers/plans/2026-08-21
```

Expected hits to fix: `docs/DCGO_EXAM.md:662`, `docs/DCGO_EXAM.md:925`, `.claude/skills/dcgo-exam/SKILL.md:3`, `.claude/skills/dcgo-exam/SKILL.md:60`, `CLAUDE.md:406`, `.github/workflows/dcgo-exam-sim.yml:36`, `docs/RUST_ENGINE_GAPS.md:2377`.

Replace `qa/qa-reports/dcgo_exam_verdicts.json` with `qa/qa-reports/exam-verdicts/` in each. **Do not** edit `docs/superpowers/plans/2026-08-21-dcgo-exam-workflow.md` or `qa/qa-reports/clause-denominator-correction.md` — those are historical records of what was true when written.

- [ ] **Step 2: Document the ledger in `docs/DCGO_EXAM.md`**

Add a section after the existing verdict-store section:

```markdown
## The ledger (fleet layout)

Three files, each shaped by how it merges:

| Path | What | Merge |
|---|---|---|
| `qa/qa-reports/exam-verdicts/<CARD-ID>.json` | Current per-clause verdicts | Disjoint writers touch disjoint files |
| `qa/qa-reports/exam-log.jsonl` | Append-only attempt history | `merge=union` (see `.gitattributes`) |
| `qa/qa-reports/exam-claims/<CARD-ID>.claim` | Advisory leases with expiry | One file per card |
| `qa/qa-reports/exam-index.md` | Generated rollup | Regenerated, never hand-edited |

**Claims are advisory.** Two nodes pushing in the same instant can both claim a
card; git is the only coordinator. That is an accepted trade rather than an
oversight — a duplicate costs one card's authoring and is visible at merge,
where a lease server would cost standing infrastructure. Claims expire so a
crashed node cannot park a card forever.

**The log answers what the store cannot.** `unmeasured` cannot distinguish
"nobody looked" from "three nodes each burned an afternoon on the same dead
end". Check the log before re-attempting a clause that has been `unmeasured`
for a while.
```

- [ ] **Step 3: Verify no stale path survives**

```bash
grep -rn 'dcgo_exam_verdicts' --include=*.md --include=*.yml . | grep -v 'docs/superpowers/plans/2026-08-21' | grep -v 'clause-denominator-correction' || echo "clean"
```

Expected: `clean`.

- [ ] **Step 4: Run the sim-only CI gate locally**

The gate reads the store; prove the repoint did not break it:

```bash
cargo run -p dcgo-harness -- exam --scenario qa/dcgo-exams --sim-only \
    --cards-json data/cards.json --decks qa/dcgo-exams/EX12/toho_pool.json
```

Expected: the corpus lowers as before (144/144 at the time of writing) and the run prints its sim-only disclaimer. **A failure here means the repoint broke the gate — fix before committing.**

- [ ] **Step 5: Commit**

```bash
git add docs/DCGO_EXAM.md .claude/skills/dcgo-exam/SKILL.md CLAUDE.md .github/workflows/dcgo-exam-sim.yml docs/RUST_ENGINE_GAPS.md
git commit -m "docs: the exam ledger is three files now, not one blob

Historical records (the 2026-08-21 plan, the denominator correction) keep the
old path deliberately -- they describe what was true when written."
```

---

## Self-Review

**Spec coverage** (`2026-08-27-archetype-campaign-fleet-design.md` §1):

| Spec requirement | Task |
|---|---|
| §1.1 per-card verdict files, directory loader, migration | 1, 2 |
| §1.1 `bind()` keeps working | 3 |
| §1.2 append-only log, union merge | 4 |
| §1.3 advisory claims, card granularity, expiry | 5 |
| §1.4 generated index, reproducibility test | 6 |
| §Testing "migration preserves all 148 rows" | 2 Step 1 + Step 4 |
| §Testing "union-merge of two divergent logs" | 4 Step 6 |
| §Testing "expired-claim behaviour" | 5 Step 1 |
| §Testing "index reproducible from the ledger" | 6 Step 1 |

Not in this plan, by design — they belong to plans 2–4: MCP tools, `node up`, the campaign skill, archetype→card resolution (which is why `exam_index.main` renders an empty index rather than inventing an archetype map).

**Type consistency:** `card_file_name` (Task 1) returns `<CARD-ID>.json`; `claim_file_name` (Task 5) returns `<CARD-ID>.claim` and is private since nothing outside the module needs it. `load_dir`/`save_dir` are used by Task 2's CLI and Task 3's Python counterpart under the same directory default, `qa/qa-reports/exam-verdicts`. `Attempt.clause` and `ClauseVerdict.clause_id` both hold `{card_id}#{zone}#{idx}`.

**Known ordering constraint:** Tasks 4 and 5 both edit `ledger.rs` and must run in order. Tasks 1–3 and 4–5 are otherwise independent and can proceed in parallel.
