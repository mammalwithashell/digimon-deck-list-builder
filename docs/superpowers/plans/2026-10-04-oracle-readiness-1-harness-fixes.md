# Oracle Readiness Plan 1 — Harness Defect Fixes and Structured Triage

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the exam-tooling defects that burned oracle round-trips in Rocks Track A, and give diverged verdicts a structured `triage` field the readiness gate can read.

**Architecture:** Small, independent fixes in the shared selection resolver (`digimon-engine/src/runners/selection_resolve.rs`), the exam harness (`code/tools/dcgo-harness`), and the Python clause binding (`code/tools/clause_coverage`). Each task is TDD with its own commit. The last task re-measures the Track A clause the defects blocked.

**Tech Stack:** Rust (digimon-engine, dcgo-harness, serde, clap), Python 3.11+ stdlib (clause_coverage), pytest.

**Spec:** `docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md` §4.1, §4.6. This is plan 1 of 5; plan 2 (readiness gate) consumes the `triage`/`citation` fields this plan adds.

## Global Constraints

- Work only in the worktree `C:\Users\james\Documents\digimon-deck-list-builder-1\.claude\worktrees\dcgo-effect-translation`. First command of every session: `git rev-parse --show-toplevel` must end in `.claude/worktrees/dcgo-effect-translation`.
- Rust builds use the per-worktree target dir (CLAUDE.md rule 31): prefix cargo with `CARGO_TARGET_DIR="D:/cargo-target/dcgo-effect-translation"` if `echo $CARGO_TARGET_DIR` is empty or points elsewhere.
- DCGO C# is read-only at the BASE repo: `C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO` (rule 29). Never init the worktree `DCGO/` submodule.
- Python tools run from the worktree root as `PYTHONPATH=code python -m tools.clause_coverage.<module>`; tests as `python -m pytest <path>`.
- The engine's no-approximations policy (rule 17) is unaffected: these are tooling fixes; no engine rule changes in this plan.
- Never commit unrelated verdict files. After any `--verdicts` run, `git status --short qa/qa-reports/exam-verdicts/` must list only the cards you meant to change.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## File Map

| File | Change | Responsibility |
|---|---|---|
| `code/digimon-engine/src/runners/selection_resolve.rs` | Modify | Trailing-PASS guard; UnionZone digivolution-source identity |
| `code/tools/dcgo-harness/src/exam/adapter.rs` | Modify | Name the actor that answered a prompt |
| `code/tools/dcgo-harness/src/exam/verdict.rs` | Modify | Dirty-only writes, line-ending preservation, `Triage` + `citation`, `set_triage`, book `cards` |
| `code/tools/dcgo-harness/src/main.rs` | Modify | `verdict-triage` subcommand; `exam --backfill` |
| `code/tools/dcgo-harness/src/mcp/handlers.rs` | Modify | Auto-extract unknown cards; default harness root for `node_health` |
| `code/tools/clause_coverage/exam_binding.py` | Modify | Pass `triage`/`citation` through `bind()` |
| `code/tools/clause_coverage/book.py` | Create | Merge newly extracted cards into the clause-text book |
| `code/tests/tools/test_clause_coverage_exam_binding.py` | Modify | Triage passthrough test |
| `code/tests/tools/test_clause_coverage_book.py` | Create | Book merge tests |
| `qa/qa-reports/exam-verdicts/{BT6-060,BT7-056,BT25-064,EX7-008,EX10-025,EX10-028}.json` | Modify (via CLI) | Backfill triage on the six diverged rows |

---

### Task 1: Trailing PASS must not decline a fresh "up to N" prompt

`resolve_next` sends PASS whenever the payload is exhausted and the live prompt is a `CountCappedMultiSelect` with PASS legal. When the row's last pick *installed* a fresh count-capped prompt (Close EX8-067's "up to 2" trash pick after its OwnField gate), that PASS declines an unrelated prompt. `SourceMulti` already has the guard (`G-TOOLING-EXAM-TRAILING-PASS-EATS-NEXT-PROMPT`).

**Files:**
- Modify: `code/digimon-engine/src/runners/selection_resolve.rs` (the `multiselectish` match inside `resolve_next`, ~L528-551; tests module after `park_attack_target_selection`, ~L1017)

**Interfaces:**
- Consumes: `resolve_next(game: &Game, payload: &SelectionRow, picks_done: usize) -> Result<Option<u16>, String>`
- Produces: unchanged signature; new behavior: `CountCappedMultiSelect { picked: 0, .. }` after an exhausted payload returns `Ok(None)`.

- [ ] **Step 1: Write the failing tests**

In the `#[cfg(test)]` module of `selection_resolve.rs`, directly after `fn park_attack_target_selection` (which already has `PendingSelection`, `SelectionKind`, `GamePhase`, `CardHandle`, `EffectSourceKind`, `DebugRunner` in scope), add:

```rust
    fn park_count_capped(game: &mut Game, picked: u8) {
        game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::CountCappedMultiSelect {
                min: 0,
                max: 2,
                picked,
                distinct: true,
            },
            selecting_player: 0,
            previous_phase: GamePhase::Main,
            valid_action_ids: vec![TRASH_EFFECT_START, PASS],
            is_optional: true,
            prompt: "Choose up to 2 cards".to_string(),
            effect_choices: None,
            source_card: CardHandle(0),
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(|_, _| {}),
            on_decline: None,
            zone_owner: None,
        });
    }

    fn one_card_row(card_id: &str) -> SelectionRow {
        SelectionRow {
            step: 0,
            actor: 0,
            prompt: "SelectPermanentEffect".to_string(),
            phase: String::new(),
            targets: None,
            card_ids: Some(vec![card_id.to_string()]),
            indexes: None,
            count: None,
            candidates: None,
            int_value: None,
            bool_value: None,
            cancel: None,
            board_p0: None,
            board_p1: None,
            memory: None,
            mechanic: None,
            zone: None,
        }
    }

    #[test]
    fn trailing_pass_does_not_decline_a_fresh_count_capped_prompt() {
        // The row's one pick went to the PREVIOUS prompt (Close EX8-067's OwnField
        // gate); resolving it installed a fresh "up to 2" trash pick with nothing
        // picked yet. Sending PASS here would decline that unrelated prompt.
        let mut runner = DebugRunner::new();
        park_count_capped(&mut runner.game, 0);
        let payload = one_card_row("EX8-047");
        assert_eq!(resolve_next(&runner.game, &payload, 1), Ok(None));
    }

    #[test]
    fn trailing_pass_still_closes_an_open_count_capped_prompt() {
        // Same prompt after >= 1 accepted pick: this IS the row's own prompt
        // awaiting its stop, so PASS is still the right answer.
        let mut runner = DebugRunner::new();
        park_count_capped(&mut runner.game, 1);
        let payload = one_card_row("EX8-047");
        assert_eq!(resolve_next(&runner.game, &payload, 1), Ok(Some(PASS)));
    }
```

If `SelectionRow` or `TRASH_EFFECT_START` is not in scope in that module, add `use crate::dcgo_recording::SelectionRow;` and `use crate::action::space::{PASS, TRASH_EFFECT_START};` at the top of the test module.

- [ ] **Step 2: Run the tests to verify the first fails**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib trailing_pass_`
Expected: `trailing_pass_does_not_decline_a_fresh_count_capped_prompt` FAILS with `left: Ok(Some(<PASS id>))`, `right: Ok(None)`; the second test passes.

- [ ] **Step 3: Apply the guard**

In `resolve_next`, replace the `multiselectish` match arms

```rust
                SelectionKind::SourceMulti { picked, .. } => picked > 0,
                SelectionKind::CountCappedMultiSelect { .. }
                | SelectionKind::RevealBucket { .. }
                | SelectionKind::DpBudget { .. }
                | SelectionKind::PlayCostBudget { .. } => true,
                _ => false,
```

with

```rust
                // A count-capped prompt with NO accepted pick is a FRESH prompt the
                // row's last pick installed (Close EX8-067: the OwnField gate pick
                // installs the "up to 2" trash pick). Same trap as SourceMulti above.
                SelectionKind::SourceMulti { picked, .. }
                | SelectionKind::CountCappedMultiSelect { picked, .. } => picked > 0,
                SelectionKind::RevealBucket { .. }
                | SelectionKind::DpBudget { .. }
                | SelectionKind::PlayCostBudget { .. } => true,
                _ => false,
```

- [ ] **Step 4: Run the new tests and the existing resolver tests**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib selection_resolve`
Expected: all pass, including the existing CountCapped family (~L1718-1951).

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/runners/selection_resolve.rs
git commit -m "exam: trailing PASS no longer declines a fresh count-capped prompt (EX8-067#effect#1)" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Pick a digivolution source by identity inside a UnionZone prompt

A `UnionZone` prompt that includes `UnionZoneSet::MATERIAL` offers each own Digimon's non-top sources as `encode_source_select(field_index, source_index)` (installer: `effect_context/selections.rs` ~L1444-1468). The resolver's UnionZone arm searches hand and trash only, so a scenario cannot name a source (`card pick 'EX10-028' not found in UnionZone`).

**Files:**
- Modify: `code/digimon-engine/src/runners/selection_resolve.rs` (new helper next to `zone_card_candidate_ids`; the `SelectionKind::Hand | SelectionKind::UnionZone { .. }` arm ~L680-689; tests module)

**Interfaces:**
- Produces: `fn union_zone_source_pick(game: &Game, owner: PlayerId, want: &str, valid: &[u16], carrier: Option<usize>) -> Result<Option<u16>, String>` (private).

- [ ] **Step 1: Write the failing tests**

Add to the tests module (same place as Task 1):

```rust
    fn park_union_zone_material(game: &mut Game, valid_action_ids: Vec<u16>) {
        game.pending_selection = Some(PendingSelection {
            kind: SelectionKind::UnionZone {
                zones: crate::selection::UnionZoneSet::MATERIAL,
            },
            selecting_player: 0,
            previous_phase: GamePhase::Main,
            valid_action_ids,
            is_optional: false,
            prompt: "Trash 1 card from your Digimon's digivolution cards".to_string(),
            effect_choices: None,
            source_card: CardHandle(0),
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(|_, _| {}),
            on_decline: None,
            zone_owner: None,
        });
    }

    #[test]
    fn union_zone_resolves_a_digivolution_source_by_identity() {
        use crate::action::space::encode_source_select;
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("SRC-A", "SRC-A"))
            .add_card(make_test_card("SRC-B", "SRC-B"))
            .add_card(make_test_card("TOP", "TOP"))
            .memory(0)
            .start();
        // place_stack is bottom -> top; the top card is never a candidate.
        runner.place_stack(0, &["SRC-A", "SRC-B", "TOP"]);
        let a = encode_source_select(0, 0).unwrap();
        let b = encode_source_select(0, 1).unwrap();
        park_union_zone_material(&mut runner.game, vec![a, b]);
        assert_eq!(resolve_next(&runner.game, &one_card_row("SRC-B"), 0), Ok(Some(b)));
    }

    #[test]
    fn union_zone_source_pick_is_ambiguous_across_two_carriers() {
        use crate::action::space::encode_source_select;
        let mut runner = DebugRunner::builder()
            .add_card(make_test_card("SRC", "SRC"))
            .add_card(make_test_card("TOP1", "TOP1"))
            .add_card(make_test_card("TOP2", "TOP2"))
            .memory(0)
            .start();
        runner.place_stack(0, &["SRC", "TOP1"]);
        runner.place_stack(0, &["SRC", "TOP2"]);
        let x = encode_source_select(0, 0).unwrap();
        let y = encode_source_select(1, 0).unwrap();
        park_union_zone_material(&mut runner.game, vec![x, y]);
        let err = resolve_next(&runner.game, &one_card_row("SRC"), 0).unwrap_err();
        assert!(err.contains("ambiguous"), "got: {err}");
    }
```

If `make_test_card` is not in scope there, add `use crate::debug_runner::make_test_card;` (the CountCapped tests use it — copy their import line).

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib union_zone_`
Expected: both FAIL with `card pick 'SRC-B' not found in UnionZone ...` (the second fails the same way instead of reporting ambiguity).

- [ ] **Step 3: Implement the helper and wire it into the arm**

Add next to `zone_card_candidate_ids`:

```rust
/// A UnionZone prompt that includes `UnionZoneSet::MATERIAL` offers each of the
/// owner's Digimon's NON-top digivolution cards as
/// `encode_source_select(field_index, source_index)` — mirror of the installer
/// in `effect_context/selections.rs`. Match by card identity; `carrier` (a
/// battle-area index from the row's `targets:`) disambiguates two carriers
/// holding the same card.
fn union_zone_source_pick(
    game: &Game,
    owner: crate::PlayerId,
    want: &str,
    valid: &[u16],
    carrier: Option<usize>,
) -> Result<Option<u16>, String> {
    use crate::action::space::encode_source_select;
    let mut hits: Vec<u16> = Vec::new();
    for (pi, perm) in game.player(owner).battle_area.iter().enumerate() {
        if carrier.is_some_and(|c| c != pi) {
            continue;
        }
        let candidates = perm.card_sources.len().saturating_sub(1);
        for (si, src) in perm.card_sources.iter().take(candidates).enumerate() {
            if src.card_id(&game.card_data) != want {
                continue;
            }
            if let Some(id) = encode_source_select(pi as u16, si as u16) {
                if valid.contains(&id) {
                    hits.push(id);
                }
            }
        }
    }
    match hits.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some(*one)),
        many => Err(format!(
            "source pick '{want}' is ambiguous -- {} carriers hold it ({many:?}); \
             name the carrier with `targets: [own.field.<i>]`",
            many.len()
        )),
    }
}
```

Replace the arm

```rust
            SelectionKind::Hand | SelectionKind::UnionZone { .. } => find_id(
                &zone_card_ids(&game.player(zone_owner).hand),
                PLAY_HAND_START,
            )
            .or_else(|| {
                find_id(
                    &zone_card_ids(&game.player(zone_owner).trash),
                    TRASH_EFFECT_START,
                )
            }),
```

with

```rust
            SelectionKind::Hand => find_id(
                &zone_card_ids(&game.player(zone_owner).hand),
                PLAY_HAND_START,
            )
            .or_else(|| {
                find_id(
                    &zone_card_ids(&game.player(zone_owner).trash),
                    TRASH_EFFECT_START,
                )
            }),
            SelectionKind::UnionZone { zones } => {
                let in_hand_or_trash = find_id(
                    &zone_card_ids(&game.player(zone_owner).hand),
                    PLAY_HAND_START,
                )
                .or_else(|| {
                    find_id(
                        &zone_card_ids(&game.player(zone_owner).trash),
                        TRASH_EFFECT_START,
                    )
                });
                match in_hand_or_trash {
                    Some(id) => Some(id),
                    None if zones.contains(crate::selection::UnionZoneSet::MATERIAL) => {
                        let carrier = payload
                            .targets
                            .as_ref()
                            .and_then(|t| t.first())
                            .and_then(|t| usize::try_from(t.frame).ok());
                        union_zone_source_pick(game, zone_owner, want, valid, carrier)?
                    }
                    None => None,
                }
            }
```

- [ ] **Step 4: Run tests**

Run: `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib selection_resolve`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/runners/selection_resolve.rs
git commit -m "exam: UnionZone prompts resolve a digivolution source by card identity" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Say who answered a prompt

"our engine auto-resolved" cannot tell "the engine never prompts" from "the driver already sent a trailing PASS" — the Close agent spent ~12 reads finding that out.

**Files:**
- Modify: `code/tools/dcgo-harness/src/exam/adapter.rs` (`advance_through_selection`, ~L1948-1986)

- [ ] **Step 1: Implement the note** (println-only behavior; covered by the Task 9 acceptance run)

In `advance_through_selection`, replace

```rust
            Ok(Some(id)) => {
                game.decode_action(id, actor);
                picks_done += 1;
```

with

```rust
            Ok(Some(id)) => {
                if id == digimon_engine::action::space::PASS
                    && picks_done >= payload_pick_count(row)
                {
                    let kind = game.pending_selection.as_ref().map(|p| p.kind);
                    println!(
                        "  note: step {i} the DRIVER sent a trailing PASS to close {kind:?} \
                         after the row's {} pick(s) -- not the engine",
                        payload_pick_count(row)
                    );
                }
                game.decode_action(id, actor);
                picks_done += 1;
```

If `digimon_engine::action::space::PASS` is not the public path, use the same `PASS` import `selection_resolve.rs` uses (`crate::action::space::PASS` inside the engine; from the harness, check `grep -n "space::PASS" code/tools/dcgo-harness/src/exam/*.rs` and reuse that path).

- [ ] **Step 2: Build and run the harness tests**

Run: `cargo test -p dcgo-harness`
Expected: PASS (no test asserts on stdout).

- [ ] **Step 3: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/adapter.rs
git commit -m "exam: name the driver when it answers a prompt with a trailing PASS" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Verdict writes touch only changed cards and keep line endings

`VerdictStore::save_dir` rewrites every card file (it holds the whole directory) with LF endings; with `core.autocrlf=true` every run dirties ~100 files.

**Files:**
- Modify: `code/tools/dcgo-harness/src/exam/verdict.rs` (`VerdictStore` struct ~L152, `record` ~L342, `save` ~L186, `save_dir` ~L265, tests module)

**Interfaces:**
- Produces: `VerdictStore` gains `#[serde(skip)] dirty: BTreeSet<String>` (card ids). `record` and (Task 5) `set_triage` insert into it. `save_dir` writes only dirty cards. `save` preserves an existing file's CRLF.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)]` module in `verdict.rs`:

```rust
    #[test]
    fn save_dir_rewrites_only_touched_cards_and_keeps_crlf() {
        let dir = std::env::temp_dir().join("verdict-dirty-only-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let row = |card: &str| {
            format!(
                "{{\r\n  \"version\": 1,\r\n  \"last_updated\": \"2026-01-01T00:00:00Z\",\r\n  \
                 \"clauses\": {{\r\n    \"{card}#effect#0\": {{\r\n      \"clause_id\": \"{card}#effect#0\",\r\n      \
                 \"card_id\": \"{card}\",\r\n      \"verdict\": \"confirmed\",\r\n      \"label\": \"Effect\",\r\n      \
                 \"text_sha256\": \"abc\",\r\n      \"recorded_at\": \"2026-01-01T00:00:00Z\"\r\n    }}\r\n  }}\r\n}}\r\n"
            )
        };
        std::fs::write(dir.join("BT1-001.json"), row("BT1-001")).unwrap();
        std::fs::write(dir.join("BT1-002.json"), row("BT1-002")).unwrap();
        let untouched_before = std::fs::read(dir.join("BT1-002.json")).unwrap();

        let mut store = VerdictStore::load_dir(&dir).unwrap();
        store.record(ClauseVerdict {
            clause_id: "BT1-001#effect#0".into(),
            card_id: "BT1-001".into(),
            verdict: Verdict::Diverged,
            label: "Effect".into(),
            text_sha256: "abc".into(),
            scenario_path: None,
            reason: Some("DIVERGED at step 1".into()),
            dcgo_build: None,
            job_id: None,
            triage: None,
            citation: None,
            recorded_at: "2026-02-01T00:00:00Z".into(),
        });
        store.save_dir(&dir).unwrap();

        assert_eq!(std::fs::read(dir.join("BT1-002.json")).unwrap(), untouched_before);
        let touched = std::fs::read_to_string(dir.join("BT1-001.json")).unwrap();
        assert!(touched.contains("\"diverged\""));
        assert!(touched.contains("\r\n"), "an existing CRLF file must stay CRLF");
        assert!(!touched.replace("\r\n", "").contains('\n'), "no bare LF lines");
    }
```

(`triage`/`citation` are added in Task 5; implement Task 4 and Task 5's struct fields together if you prefer, or temporarily drop those two lines and add them back in Task 5.)

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p dcgo-harness save_dir_rewrites_only_touched_cards_and_keeps_crlf`
Expected: FAIL — BT1-002.json bytes changed (LF rewrite).

- [ ] **Step 3: Implement**

1. Add `use std::collections::BTreeSet;` to the imports.
2. In `pub struct VerdictStore`, next to `current_text_shas`, add:

```rust
    /// Card ids whose rows changed since load; `save_dir` writes only these.
    #[serde(skip)]
    dirty: BTreeSet<String>,
```

(If `VerdictStore` is built with a struct literal anywhere — `grep -n "VerdictStore {" code/tools/dcgo-harness/src` — add `dirty: BTreeSet::new()` there; `#[derive(Default)]` covers `or_default()`.)

3. Replace `record`:

```rust
    /// Insert or replace one clause's verdict, marking its card file dirty.
    pub fn record(&mut self, v: ClauseVerdict) {
        self.last_updated = v.recorded_at.clone();
        self.dirty.insert(v.card_id.clone());
        self.clauses.insert(v.clause_id.clone(), v);
    }
```

4. In `save`, replace

```rust
        let mut text = self.to_json()?;
        text.push('\n');
```

with

```rust
        let mut text = self.to_json()?;
        text.push('\n');
        // Keep the file's existing line endings: core.autocrlf checkouts hold
        // CRLF, and an LF rewrite shows up as a modified file for no reason.
        let existing_crlf = std::fs::read(path)
            .map(|b| b.windows(2).any(|w| w == b"\r\n"))
            .unwrap_or(false);
        if existing_crlf {
            text = text.replace('\n', "\r\n");
        }
```

5. In `save_dir`, replace

```rust
        for (card_id, per) in by_card.iter() {
            per.save(&dir.join(card_file_name(card_id)))?;
        }
```

with

```rust
        for (card_id, per) in by_card.iter() {
            let file = dir.join(card_file_name(card_id));
            // Only cards whose rows changed this run are rewritten; a file that
            // does not exist yet is always written.
            if self.dirty.contains(card_id) || !file.exists() {
                per.save(&file)?;
            }
        }
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p dcgo-harness verdict`
Expected: all verdict tests pass (existing save/load/prune tests included).

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/verdict.rs
git commit -m "exam: verdict store rewrites only changed cards and keeps CRLF files CRLF" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Structured triage on diverged verdicts

**Files:**
- Modify: `code/tools/dcgo-harness/src/exam/verdict.rs` (`ClauseVerdict` ~L79-107; new `Triage` enum; `impl VerdictStore` gains `set_triage`; every `ClauseVerdict { .. }` literal gains the two fields)
- Modify: `code/tools/dcgo-harness/src/main.rs` (new `VerdictTriage` subcommand; `needs_root` exemption; dispatch)
- Modify: `code/tools/clause_coverage/exam_binding.py` (clause dict in `bind`, ~L405-419)
- Test: `code/tests/tools/test_clause_coverage_exam_binding.py`

**Interfaces:**
- Produces (Rust): `pub enum Triage { OursWrong, DcgoQuirk, Undetermined }` (serde `snake_case`); `ClauseVerdict.triage: Option<Triage>`, `ClauseVerdict.citation: Option<String>` (both `#[serde(default, skip_serializing_if = "Option::is_none")]`); `pub fn set_triage(&mut self, clause_id: &str, triage: Triage, citation: Option<String>) -> Result<(), String>`.
- Produces (CLI): `dcgo-harness verdict-triage --clause <ID> --triage <ours_wrong|dcgo_quirk|undetermined> [--citation <TEXT>] [--verdicts <DIR>]`.
- Produces (Python): each clause dict from `bind()` carries `"triage"` and `"citation"` (None when absent). Plan 2's readiness generator reads them.

- [ ] **Step 1: Write the failing Rust tests**

Add to the `verdict.rs` tests module:

```rust
    fn diverged_row(card: &str) -> ClauseVerdict {
        ClauseVerdict {
            clause_id: format!("{card}#effect#0"),
            card_id: card.into(),
            verdict: Verdict::Diverged,
            label: "Effect".into(),
            text_sha256: "abc".into(),
            scenario_path: None,
            reason: Some("DIVERGED at step 3".into()),
            dcgo_build: None,
            job_id: None,
            triage: None,
            citation: None,
            recorded_at: "2026-02-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn set_triage_records_class_and_citation_on_a_diverged_row() {
        let mut store = VerdictStore::default();
        store.record(diverged_row("BT6-060"));
        store
            .set_triage("BT6-060#effect#0", Triage::DcgoQuirk, Some("G-EXAM-REVEAL-BUCKET-ADD-TIMING".into()))
            .unwrap();
        let row = store.get("BT6-060#effect#0").unwrap();
        assert_eq!(row.triage, Some(Triage::DcgoQuirk));
        assert_eq!(row.citation.as_deref(), Some("G-EXAM-REVEAL-BUCKET-ADD-TIMING"));
        let json = store.to_json().unwrap();
        assert!(json.contains("\"triage\": \"dcgo_quirk\""));
    }

    #[test]
    fn set_triage_refuses_quirk_or_ours_wrong_without_citation() {
        let mut store = VerdictStore::default();
        store.record(diverged_row("BT6-060"));
        assert!(store.set_triage("BT6-060#effect#0", Triage::DcgoQuirk, None).is_err());
        assert!(store.set_triage("BT6-060#effect#0", Triage::OursWrong, Some("  ".into())).is_err());
        assert!(store.set_triage("BT6-060#effect#0", Triage::Undetermined, None).is_ok());
    }

    #[test]
    fn set_triage_refuses_a_non_diverged_row() {
        let mut store = VerdictStore::default();
        let mut row = diverged_row("BT1-001");
        row.verdict = Verdict::Confirmed;
        store.record(row);
        let err = store
            .set_triage("BT1-001#effect#0", Triage::DcgoQuirk, Some("x".into()))
            .unwrap_err();
        assert!(err.contains("not diverged"), "got: {err}");
    }

    #[test]
    fn old_rows_without_triage_still_parse() {
        let text = r#"{"version":1,"last_updated":"x","clauses":{"BT1-001#effect#0":{
            "clause_id":"BT1-001#effect#0","card_id":"BT1-001","verdict":"diverged",
            "label":"Effect","text_sha256":"abc","recorded_at":"x"}}}"#;
        let store = VerdictStore::from_json(text).unwrap();
        assert_eq!(store.get("BT1-001#effect#0").unwrap().triage, None);
    }
```

(If `VerdictStore::from_json` takes a second argument, copy the call shape from an existing test in the module.)

- [ ] **Step 2: Run to verify they fail to compile**

Run: `cargo test -p dcgo-harness set_triage`
Expected: compile errors — no `Triage`, no `triage`/`citation` fields, no `set_triage`.

- [ ] **Step 3: Implement the Rust side**

1. Next to `pub enum Verdict` add:

```rust
/// Whose bug a `diverged` clause is. `general_rule.pdf` outranks DCGO, so a
/// divergence is not automatically ours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Triage {
    /// Our engine is wrong; the rules or DCGO's correct behavior is cited.
    OursWrong,
    /// DCGO is wrong or differs only in a rules-neutral way; cited.
    DcgoQuirk,
    /// Not yet decided. Blocks readiness.
    Undetermined,
}
```

2. In `pub struct ClauseVerdict`, after `job_id`, add:

```rust
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triage: Option<Triage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
```

3. Fix every struct literal: `grep -rn "ClauseVerdict {" code/tools/dcgo-harness/src` and add `triage: None, citation: None,` to each (including `record_scenario_verdict`).

4. In `impl VerdictStore` add:

```rust
    /// Classify a `diverged` row. `ours_wrong` and `dcgo_quirk` need a citation
    /// (a `general_rule.pdf` section, a DCGO `file:line`, or a gap-tracker entry
    /// that carries one); `undetermined` does not. Marks the card dirty.
    pub fn set_triage(
        &mut self,
        clause_id: &str,
        triage: Triage,
        citation: Option<String>,
    ) -> Result<(), String> {
        let citation = citation.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
        if matches!(triage, Triage::OursWrong | Triage::DcgoQuirk) && citation.is_none() {
            return Err(format!(
                "triage {triage:?} for `{clause_id}` needs --citation (rules section, \
                 DCGO file:line, or gap-tracker id)"
            ));
        }
        let row = self
            .clauses
            .get_mut(clause_id)
            .ok_or_else(|| format!("no stored verdict for `{clause_id}`"))?;
        if row.verdict != Verdict::Diverged {
            return Err(format!(
                "`{clause_id}` is {} -- not diverged; only a diverged row is triaged",
                row.verdict
            ));
        }
        row.triage = Some(triage);
        row.citation = citation;
        self.dirty.insert(row.card_id.clone());
        Ok(())
    }
```

5. In `main.rs`, add to `enum Command` (anywhere among the variants):

```rust
    /// Classify a diverged exam verdict: whose bug is it?
    VerdictTriage {
        #[arg(long)]
        clause: String,
        /// ours_wrong | dcgo_quirk | undetermined
        #[arg(long)]
        triage: String,
        #[arg(long)]
        citation: Option<String>,
        #[arg(long, default_value = "qa/qa-reports/exam-verdicts")]
        verdicts: PathBuf,
    },
```

In `fn needs_root`, add `Command::VerdictTriage { .. }` to the exempt set (same arm as `Exam` / `MigrateVerdicts` / `Mcp`). In `fn run`, add the dispatch arm:

```rust
        Command::VerdictTriage { clause, triage, citation, verdicts } => {
            use dcgo_harness::exam::verdict::{Triage, VerdictStore};
            let class = match triage.as_str() {
                "ours_wrong" => Triage::OursWrong,
                "dcgo_quirk" => Triage::DcgoQuirk,
                "undetermined" => Triage::Undetermined,
                other => return Err(format!(
                    "--triage must be ours_wrong | dcgo_quirk | undetermined, got `{other}`"
                )),
            };
            let mut store = VerdictStore::load_dir(verdicts)?;
            store.set_triage(clause, class, citation.clone())?;
            store.save_dir(verdicts)?;
            println!("verdict-triage: {clause} -> {triage}");
            Ok(ExitCode::SUCCESS)
        }
```

6. Re-run Task 4's test now that the fields exist.

- [ ] **Step 4: Run Rust tests**

Run: `cargo test -p dcgo-harness`
Expected: PASS.

- [ ] **Step 5: Write the failing Python test**

Append to `code/tests/tools/test_clause_coverage_exam_binding.py`:

```python
def test_bind_passes_triage_and_citation_through(tmp_path):
    import json
    from tools.clause_coverage.exam_binding import bind

    verdicts = tmp_path / "exam-verdicts"
    verdicts.mkdir()
    (verdicts / "EX10-025.json").write_text(
        json.dumps(
            {
                "version": 1,
                "last_updated": "x",
                "clauses": {
                    "EX10-025#inherited#0": {
                        "clause_id": "EX10-025#inherited#0",
                        "card_id": "EX10-025",
                        "verdict": "diverged",
                        "label": "Inherited Effect",
                        "recorded_at": "x",
                        "triage": "ours_wrong",
                        "citation": "general_rule.pdf 15-8-3-2",
                    }
                },
            }
        ),
        encoding="utf-8",
    )
    result = bind(["EX10-025"], None, verdicts)
    row = next(
        c for c in result["cards"]["EX10-025"]["clauses"]
        if c["clause_id"] == "EX10-025#inherited#0"
    )
    assert row["verdict"] == "diverged"
    assert row["triage"] == "ours_wrong"
    assert row["citation"] == "general_rule.pdf 15-8-3-2"
```

(No `text_sha256` in the row on purpose: `bind` only invalidates a row whose stored sha differs, so this row stays `diverged`.)

- [ ] **Step 6: Run to verify it fails**

Run: `python -m pytest code/tests/tools/test_clause_coverage_exam_binding.py::test_bind_passes_triage_and_citation_through -v`
Expected: FAIL with `KeyError: 'triage'`.

- [ ] **Step 7: Implement the passthrough**

In `exam_binding.py`, in the clause dict appended inside `bind` (the block ending with `"job_id": entry.get("job_id"),`), add two keys after `job_id`:

```python
                "job_id": entry.get("job_id"),
                "triage": entry.get("triage"),
                "citation": entry.get("citation"),
```

- [ ] **Step 8: Run Python tests**

Run: `python -m pytest code/tests/tools/test_clause_coverage_exam_binding.py -v`
Expected: PASS.

- [ ] **Step 9: Backfill the six existing diverged rows**

Four are the documented DCGO quirk `G-EXAM-REVEAL-BUCKET-ADD-TIMING` (`docs/RUST_ENGINE_GAPS.md`, DCGO `RevealLibrary.cs:291-329`); two are the Track A engine defect (`general_rule.pdf` §15-8-3-2). Build once, then run (each command is separate):

```bash
cargo build -p dcgo-harness
```
```bash
D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe verdict-triage --clause "BT6-060#effect#0" --triage dcgo_quirk --citation "G-EXAM-REVEAL-BUCKET-ADD-TIMING (docs/RUST_ENGINE_GAPS.md); DCGO RevealLibrary.cs:291-329"
```
```bash
D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe verdict-triage --clause "BT7-056#effect#0" --triage dcgo_quirk --citation "G-EXAM-REVEAL-BUCKET-ADD-TIMING (docs/RUST_ENGINE_GAPS.md); DCGO RevealLibrary.cs:291-329"
```
```bash
D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe verdict-triage --clause "BT25-064#effect#1" --triage dcgo_quirk --citation "G-EXAM-REVEAL-BUCKET-ADD-TIMING (docs/RUST_ENGINE_GAPS.md); DCGO RevealLibrary.cs:291-329"
```
```bash
D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe verdict-triage --clause "EX7-008#effect#1" --triage dcgo_quirk --citation "G-EXAM-REVEAL-BUCKET-ADD-TIMING (docs/RUST_ENGINE_GAPS.md); DCGO RevealLibrary.cs:291-329"
```
```bash
D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe verdict-triage --clause "EX10-025#inherited#0" --triage ours_wrong --citation "general_rule.pdf 15-8-3-2 (p.25); G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS"
```
```bash
D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe verdict-triage --clause "EX10-028#inherited#0" --triage ours_wrong --citation "general_rule.pdf 15-8-3-2 (p.25); G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS"
```

Then verify only those six files changed:

Run: `git status --short qa/qa-reports/exam-verdicts/`
Expected: exactly `BT6-060.json`, `BT7-056.json`, `BT25-064.json`, `EX7-008.json`, `EX10-025.json`, `EX10-028.json`.

- [ ] **Step 10: Commit**

```bash
git add code/tools/dcgo-harness/src/exam/verdict.rs code/tools/dcgo-harness/src/main.rs code/tools/clause_coverage/exam_binding.py code/tests/tools/test_clause_coverage_exam_binding.py qa/qa-reports/exam-verdicts/BT6-060.json qa/qa-reports/exam-verdicts/BT7-056.json qa/qa-reports/exam-verdicts/BT25-064.json qa/qa-reports/exam-verdicts/EX7-008.json qa/qa-reports/exam-verdicts/EX10-025.json qa/qa-reports/exam-verdicts/EX10-028.json
git commit -m "exam: structured triage on diverged verdicts; classify the six existing divergences" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: `exam --backfill` writes confirmed state into the scenario

`exam/backfill.rs::backfill_from_diff` exists but nothing calls it, so a confirmed scenario has no `assert:` block and checks nothing in CI.

**Files:**
- Modify: `code/tools/dcgo-harness/src/main.rs` (`Exam` variant ~L89-136; dispatch ~L435; `run_exam` and `exam_one` signatures; the oracle branch of `exam_one` after `let report = diff_paired(...)`)

**Interfaces:**
- Consumes: `dcgo_harness::exam::backfill::backfill_from_diff(scenario_yaml: &str, confirmed: &[StateProjection], report: &DiffReport) -> Result<String, String>`
- Produces: CLI flag `--backfill` (oracle mode only). Plan 3's one-call loop reuses the same code path.

- [ ] **Step 1: Add the flag and thread it through**

1. In `Command::Exam`, add:

```rust
        /// On a CLEAN oracle diff, write the confirmed state into the scenario's
        /// `assert:` block (rows marked `_backfilled`). Oracle mode only.
        #[arg(long)]
        backfill: bool,
```

2. In the `Command::Exam { .. }` dispatch, destructure `backfill` and pass `*backfill` to `run_exam` as a new last argument; add `backfill: bool` as the last parameter of `run_exam` and pass it to every `exam_one(...)` call; add `backfill: bool` as the last parameter of `exam_one`.

3. In `run_exam`, next to the existing `--emit-job requires --sim-only` check, add:

```rust
    if backfill && sim_only {
        return Err("--backfill needs an oracle run (--sidecar); sim-only confirms nothing".into());
    }
```

4. In `exam_one`'s oracle branch, directly after the `if all_diffs { ... } else { ... }` print block, add:

```rust
    if backfill && report.is_clean() {
        // Our projections equal DCGO's on every compared field when the diff is
        // clean; ours carry exactly one row per step plus the trailing state
        // the `assert:` block wants.
        let updated =
            dcgo_harness::exam::backfill::backfill_from_diff(&text, &projections, &report)?;
        std::fs::write(path, updated)
            .map_err(|e| format!("writing backfilled scenario {}: {e}", path.display()))?;
        println!("  backfill: wrote confirmed state into {}", path.display());
    }
```

- [ ] **Step 2: Build**

Run: `cargo build -p dcgo-harness`
Expected: compiles.

- [ ] **Step 3: Acceptance against a preserved Track A sidecar (no Unity needed)**

Find the recording for Close's confirmed security clause:

Run: `python -c "import json;print(json.load(open(r'C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness/done/exam-EX8-067-effect2.result.json'))['recording_path'])"`
Expected: a `...\dcgo_recordings\<stamp>.jsonl` path. The sidecar is the same path ending in `.state.jsonl`.

Run (substitute the sidecar path):
```bash
D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe exam --scenario qa/dcgo-exams/EX8/EX8-067-effect2.yaml --sidecar "<sidecar path>" --cards-json data/cards.json --backfill
```
Expected: `CLEAN (...)` then `backfill: wrote confirmed state into qa/dcgo-exams/EX8/EX8-067-effect2.yaml`. If it instead fails with a backfill step-count refusal (`missing steps 1..=N`), change the `&projections` argument above to `&dcgo` (DCGO's aligned rows) and rebuild — record which one worked in the commit message.

Run: `D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe exam --scenario qa/dcgo-exams/EX8/EX8-067-effect2.yaml --sim-only --cards-json data/cards.json --decks qa/dcgo-exams/EX10/rocks_pool.json`
Expected: `assert: N check(s) over M assertion block(s), 0 failed` with N > 0.

- [ ] **Step 4: Commit**

```bash
git add code/tools/dcgo-harness/src/main.rs qa/dcgo-exams/EX8/EX8-067-effect2.yaml
git commit -m "exam: --backfill writes a clean oracle run's state into the scenario" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: MCP plan/status extract unknown cards instead of reporting zero

`exam_plan`/`exam_status` read `qa/exam-clause-text.json` (98 cards) and report `outstanding_total: 0` for any other card.

**Files:**
- Create: `code/tools/clause_coverage/book.py`
- Create: `code/tests/tools/test_clause_coverage_book.py`
- Modify: `code/tools/dcgo-harness/src/exam/verdict.rs` (`ClauseTextFile`, `ClauseTextBook`)
- Modify: `code/tools/dcgo-harness/src/mcp/handlers.rs` (`load_clause_book` callers in `exam_status` / `exam_plan`)

**Interfaces:**
- Produces (Python): `merge_into_book(book_path: Path, card_ids: list[str], *, extract=extract_run) -> list[str]` returns the ids it added; CLI `python -m tools.clause_coverage.book add --book <path> --card-ids <ids...>`.
- Produces (Rust): `ClauseTextBook::has_card(&self, card_id: &str) -> bool`.

- [ ] **Step 1: Write the failing Python tests**

Create `code/tests/tools/test_clause_coverage_book.py`:

```python
import json

from tools.clause_coverage.book import merge_into_book


def _fake_extract(card_ids, source_desc, **_):
    clauses = [
        {"id": f"{cid}#effect#0", "card_id": cid, "zone": "effect", "label": "Effect",
         "kind": "timing", "timings": [], "keyword": None, "text": f"text {cid}",
         "source": "bundle", "image_path": None}
        for cid in card_ids if cid != "VANILLA-1"
    ]
    return {"generated_at": "t", "source": source_desc, "cards": list(card_ids),
            "clauses": clauses, "denominator": {}}


def _write_book(path, cards, clauses):
    path.write_text(json.dumps({"generated_at": "t", "source": "seed", "cards": cards,
                                "clauses": clauses, "denominator": {}}), encoding="utf-8")


def test_merge_adds_only_missing_cards_and_sorts(tmp_path):
    book = tmp_path / "book.json"
    _write_book(book, ["BT1-001"], [{"id": "BT1-001#effect#0", "label": "Effect", "text": "old"}])
    added = merge_into_book(book, ["BT1-001", "EX10-025"], extract=_fake_extract)
    assert added == ["EX10-025"]
    data = json.loads(book.read_text(encoding="utf-8"))
    assert data["cards"] == ["BT1-001", "EX10-025"]
    assert [c["id"] for c in data["clauses"]] == ["BT1-001#effect#0", "EX10-025#effect#0"]
    assert data["clauses"][0]["text"] == "old", "existing clauses are never re-extracted"


def test_a_zero_clause_card_is_recorded_so_it_is_not_re_extracted(tmp_path):
    book = tmp_path / "book.json"
    _write_book(book, [], [])
    assert merge_into_book(book, ["VANILLA-1"], extract=_fake_extract) == ["VANILLA-1"]
    assert merge_into_book(book, ["VANILLA-1"], extract=_fake_extract) == []
```

- [ ] **Step 2: Run to verify it fails**

Run: `python -m pytest code/tests/tools/test_clause_coverage_book.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'tools.clause_coverage.book'`.

- [ ] **Step 3: Implement `book.py`**

```python
"""Merge newly extracted cards into the clause-text book (`qa/exam-clause-text.json`).

The book is the exam denominator the harness and MCP read. A card absent from
it is invisible to `exam_plan` / `exam_status`, which used to report zero
outstanding work for it. `add` extracts only the missing cards and merges them
deterministically (cards sorted, clauses sorted by id); existing clauses are
never re-extracted.

Usage::

    PYTHONPATH=code python -m tools.clause_coverage.book add \
        --book qa/exam-clause-text.json --card-ids EX10-025 EX8-067
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

_CODE = Path(__file__).resolve().parents[2]
if str(_CODE) not in sys.path:
    sys.path.insert(0, str(_CODE))

from tools.clause_coverage.extract import run as extract_run  # noqa: E402


def merge_into_book(book_path: Path, card_ids: list[str], *, extract=extract_run) -> list[str]:
    data = json.loads(book_path.read_text(encoding="utf-8")) if book_path.exists() else {
        "cards": [], "clauses": []}
    known = set(data.get("cards", []))
    known |= {c["id"].split("#", 1)[0] for c in data.get("clauses", [])}
    missing = sorted({cid for cid in card_ids if cid not in known})
    if not missing:
        return []
    fresh = extract(missing, f"book add ({len(missing)} ids)")
    by_id = {c["id"]: c for c in data.get("clauses", [])}
    for clause in fresh.get("clauses", []):
        by_id.setdefault(clause["id"], clause)
    data["cards"] = sorted(known | set(missing))
    data["clauses"] = [by_id[k] for k in sorted(by_id)]
    data["source"] = f"{data.get('source', '')} + book add {', '.join(missing)}".strip(" +")
    data["denominator"] = {"total_cards": len(data["cards"]), "total_clauses": len(data["clauses"])}
    book_path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return missing


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    add = sub.add_parser("add")
    add.add_argument("--book", type=Path, default=Path("qa/exam-clause-text.json"))
    add.add_argument("--card-ids", nargs="+", required=True)
    args = parser.parse_args(argv)
    added = merge_into_book(args.book, args.card_ids)
    print(json.dumps({"added": added}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 4: Run Python tests**

Run: `python -m pytest code/tests/tools/test_clause_coverage_book.py -v`
Expected: PASS.

- [ ] **Step 5: Teach `ClauseTextBook` which cards it covers (Rust)**

In `verdict.rs`:

```rust
#[derive(Debug, Deserialize)]
struct ClauseTextFile {
    #[serde(default)]
    cards: Vec<String>,
    clauses: Vec<ClauseText>,
}
```

Add a `cards: std::collections::BTreeSet<String>` field to `ClauseTextBook`; in `from_json` fill it from `file.cards` plus the card prefix of every clause id (`c.id.split('#').next()`), and add:

```rust
    /// True when the book's extract covered this card (even with zero clauses).
    pub fn has_card(&self, card_id: &str) -> bool {
        self.cards.contains(card_id)
    }
```

Add a test:

```rust
    #[test]
    fn book_knows_zero_clause_cards() {
        let book = ClauseTextBook::from_json(
            r#"{"cards":["BT1-009","ST1-07"],"clauses":[{"id":"ST1-07#inherited#0","label":"Inherited Effect","text":"x"}]}"#,
            "t",
        )
        .unwrap();
        assert!(book.has_card("BT1-009"));
        assert!(book.has_card("ST1-07"));
        assert!(!book.has_card("EX10-025"));
    }
```

Run: `cargo test -p dcgo-harness book_knows_zero_clause_cards`
Expected: PASS after the change.

- [ ] **Step 6: Auto-extract in the MCP handlers**

In `mcp/handlers.rs`, add a helper beside `exam_keyword_brief`:

```rust
/// Extract and merge any requested card the clause-text book has never seen,
/// so plan/status never report "0 outstanding" for an unknown card.
fn ensure_cards_in_book(book_path: &Path, cards: &[String]) -> Result<Vec<String>, String> {
    let book = crate::exam::verdict::ClauseTextBook::load(book_path)?;
    let missing: Vec<String> = cards.iter().filter(|c| !book.has_card(c)).cloned().collect();
    if missing.is_empty() {
        return Ok(vec![]);
    }
    let mut args = vec![
        "-m".to_string(),
        "tools.clause_coverage.book".to_string(),
        "add".to_string(),
        "--book".to_string(),
        book_path.display().to_string(),
        "--card-ids".to_string(),
    ];
    args.extend(missing.iter().cloned());
    let out = std::process::Command::new("python")
        .args(&args)
        .env("PYTHONPATH", "code")
        .output()
        .map_err(|e| format!("running clause extraction for {missing:?}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "clause extraction failed for {missing:?}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(missing)
}
```

In both `exam_status` and `exam_plan`, after the requested card list is resolved and before `load_clause_book(...)` is called, add:

```rust
    let book_path = clause_text_json_path(params);
    let extracted = ensure_cards_in_book(Path::new(&book_path), &cards)?;
```

(use the variable name the handler already uses for the requested cards and for the book path; if `clause_text_json_path` returns a `PathBuf`, pass `&book_path` directly), and add `"extracted_now": extracted` to the returned JSON object.

- [ ] **Step 7: Run harness tests**

Run: `cargo test -p dcgo-harness`
Expected: PASS. (The handler fixture tests write a book that already contains their card, so no Python is spawned.)

- [ ] **Step 8: Manual check**

Run: `PYTHONPATH=code python -m tools.clause_coverage.book add --book qa/exam-clause-text.json --card-ids EX10-025 EX10-028 EX8-067`
Expected: `{"added": ["EX10-025", "EX10-028", "EX8-067"]}`; `git diff --stat qa/exam-clause-text.json` shows only additions.

- [ ] **Step 9: Commit**

```bash
git add code/tools/clause_coverage/book.py code/tests/tools/test_clause_coverage_book.py code/tools/dcgo-harness/src/exam/verdict.rs code/tools/dcgo-harness/src/mcp/handlers.rs qa/exam-clause-text.json
git commit -m "exam: plan/status extract unknown cards into the clause book instead of reporting zero" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: MCP `node_health` finds the harness root

`node_health` checks `.` when the MCP was started without `--root` (as `.mcp.json` does) and reports NO-GO.

**Files:**
- Modify: `code/tools/dcgo-harness/src/node.rs` (new `default_harness_root`)
- Modify: `code/tools/dcgo-harness/src/mcp/handlers.rs` (`node_health`, ~L544)

**Interfaces:**
- Produces: `pub fn default_harness_root() -> Option<PathBuf>` in `node.rs`: `DCGO_HARNESS_ROOT` env var if set, else `%USERPROFILE%/AppData/LocalLow/DCGO/DCGO/dcgo_harness` if that directory exists, else `None`. Plan 3 reuses it.

- [ ] **Step 1: Write the failing test** (in `node.rs` tests module, or a new `#[cfg(test)] mod tests` at the bottom)

```rust
    #[test]
    fn default_harness_root_honours_the_env_override() {
        let dir = std::env::temp_dir().join("harness-root-env-test");
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("DCGO_HARNESS_ROOT", &dir);
        assert_eq!(super::default_harness_root(), Some(dir.clone()));
        std::env::remove_var("DCGO_HARNESS_ROOT");
    }
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p dcgo-harness default_harness_root`
Expected: compile error — function missing.

- [ ] **Step 3: Implement**

In `node.rs`:

```rust
/// The harness root an oracle node uses when none is passed: the
/// `DCGO_HARNESS_ROOT` override, else the DCGO player's LocalLow directory
/// when it exists.
pub fn default_harness_root() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("DCGO_HARNESS_ROOT") {
        if !p.trim().is_empty() {
            return Some(std::path::PathBuf::from(p));
        }
    }
    let profile = std::env::var("USERPROFILE").ok()?;
    let p = std::path::Path::new(&profile)
        .join("AppData/LocalLow/DCGO/DCGO/dcgo_harness");
    p.is_dir().then_some(p)
}
```

In `handlers.rs::node_health`, replace the `"."` default root with:

```rust
    let root_buf = root
        .map(|r| r.to_path_buf())
        .or_else(crate::node::default_harness_root)
        .ok_or("no harness root: pass --root to the MCP or set DCGO_HARNESS_ROOT")?;
    let root = root_buf.as_path();
```

(adapt to how the handler currently binds `root`; the global verdicts/claims directories are NOT changed — only `node_health` uses this default.)

- [ ] **Step 4: Run tests**

Run: `cargo test -p dcgo-harness`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/tools/dcgo-harness/src/node.rs code/tools/dcgo-harness/src/mcp/handlers.rs
git commit -m "mcp: node_health resolves the oracle's harness root by default" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Acceptance — re-measure Close EX8-067#effect#1 on the oracle

Requires the oracle node. Preflight first.

- [ ] **Step 1: Preflight**

Run: `D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe --root "C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness" node status --build D:/dcgo-build/scripted-v17`
Expected: `GO`. If the player is not running: `... node up --build D:/dcgo-build/scripted-v17` (run in the background; it keeps the player alive).

- [ ] **Step 2: Sim-only with the guard in place**

Run: `D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe exam --scenario qa/dcgo-exams/EX8/EX8-067-effect1.yaml --sim-only --cards-json data/cards.json --decks qa/dcgo-exams/EX10/rocks_pool.json --emit-job C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness/jobs`
Expected: the line completes with no "DRIVER sent a trailing PASS" note at the trash-pick step, and a job file `exam-EX8-067-effect1.json` is written. If our engine now places 0 cards because the scenario still declines, fix the scenario's trash-pick row to `cards: [EX8-047, BT21-055]` (the cards DCGO placed in round-trip 3, per `qa/dcgo-exams/EX8/NOTES-EX8-067.md`) and re-run.

- [ ] **Step 3: Wait for the oracle and diff**

Wait until `C:/Users/james/AppData/LocalLow/DCGO/DCGO/dcgo_harness/done/exam-EX8-067-effect1.result.json` has a newer mtime than the job you emitted; read its `recording_path`; then:

Run: `D:/cargo-target/dcgo-effect-translation/debug/dcgo-harness.exe exam --scenario qa/dcgo-exams/EX8/EX8-067-effect1.yaml --sidecar "<recording_path with .state.jsonl>" --cards-json data/cards.json --verdicts --clause-text-json qa/exam-clause-text.json --backfill`
Expected: `CLEAN` → verdict `confirmed` recorded, scenario backfilled. If `DIVERGED`, triage it with `verdict-triage` (cite the rule or DCGO line) — do not fix engine code in this plan.

- [ ] **Step 4: Verify scope and commit**

Run: `git status --short qa/`
Expected: only `qa/dcgo-exams/EX8/EX8-067-effect1.yaml`, `qa/dcgo-exams/EX8/NOTES-EX8-067.md` (append a dated line with the outcome), and `qa/qa-reports/exam-verdicts/EX8-067.json`.

```bash
git add qa/dcgo-exams/EX8/EX8-067-effect1.yaml qa/dcgo-exams/EX8/NOTES-EX8-067.md qa/qa-reports/exam-verdicts/EX8-067.json
git commit -m "exam: EX8-067#effect#1 re-measured with the trailing-PASS guard" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
