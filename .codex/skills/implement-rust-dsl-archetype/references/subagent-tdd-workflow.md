# Subagent TDD Workflow

Use this reference after the target card pool is resolved, classified, and grouped into batches.

## Controller Rules

- Process the entire requested queue, one batch at a time; default batch size is 4.
- Dispatch one fresh worker per non-skipped card in the current batch.
- Provide exact card context; do not make workers rediscover the whole archetype.
- Do not dispatch parallel workers that touch the same YAML file, Rust test file, Rust module, selection/action internals, or shared tracker.
- Keep workers confined to `code/digimon-engine/cards/<set>/<CARD-ID>.yaml` and `code/digimon-engine/tests/cards_behavioral/<set>/<card_id_lower>.rs`.
- Keep the main session responsible for `main.rs`, `mod.rs`, `validated_cards_dsl.json`, gap trackers, QA artifacts, integration, and final verification.
- Review each batch in two stages: spec compliance first, code quality second.
- After a batch passes review and targeted tests, immediately continue to the next planned batch. Do not pause for user confirmation, and do not send a long per-batch report unless blocked.
- Send brief progress updates while working; reserve full tables and totals for the final response after all planned batches complete or a true stop condition prevents further progress.

If subagents are unavailable in the current environment, do not silently downgrade the workflow. Tell the user and ask whether to execute the same TDD gates locally.

## Batch Plan Template

Before implementation, print a plan in this shape:

```text
Target: <archetype/card list/pool>
Total cards in pool: <N>
IMPLEMENT (no YAML yet): <n>
AUDIT (existing YAML): <n>
SKIP (complete prior verdict): <n>
BLOCKED before implementation: <n>

To process: <m> cards in <b> batches of up to <batch_size>

Batch 1: <CARD-ID [I|A]>, <CARD-ID [I|A]>, ...
Batch 2: ...

Note: [I] = IMPLEMENT, [A] = AUDIT
```

## Orchestrator Pre-Wire

Before dispatching the first batch, pre-wire test discovery for every non-skipped card:

1. Ensure `code/digimon-engine/tests/cards_behavioral/<set>/mod.rs` exists and contains `mod <card_id_lower>;`.
2. Ensure `code/digimon-engine/tests/cards_behavioral/main.rs` contains `mod <set>;`.
3. Ensure `code/digimon-engine/tests/cards_behavioral/<set>/<card_id_lower>.rs` exists, even if empty.

If a card returns `BLOCKED` and wrote no tests, remove its module line and placeholder file during merge. Workers must not edit registration files themselves.

## Per-Card Context Pack

For each card in a batch, gather:

- Printed text and metadata from `data/cards.json`: name, kind, level, DP, play cost, colors, traits, evo costs, effect, inherited, and security text.
- DCGO C# reference if present at `DCGO/Assets/Scripts/CardEffect/<SET>/*/<CARD_ID_UNDERSCORE>.cs`.
- Prior DSL verdict from `qa/qa-reports/validated_cards_dsl.json`, if present.
- Existing YAML and tests for `AUDIT` mode.

Use printed text as authoritative. DCGO is a behavioral implementation reference and tiebreaker only.

## Optional Scout Pass

Use a read-only scout for complex `IMPLEMENT` cards or cards likely to expose a reusable gap. Ask the scout for:

- Test API pattern rows.
- Required DSL verbs or missing DSL vocabulary.
- Closest exemplar YAMLs.
- Target `EffectContext` APIs.
- Behavioral test scope.
- Gap suspicion: `NONE`, `ENGINE-GAP`, `DSL-GAP`, or `HYBRID`.

If the scout identifies a clear missing API or DSL verb, the orchestrator may mark the card `BLOCKED` after reviewer confirmation instead of sending it to an implementer.

## Implement Worker Template

Each `IMPLEMENT` worker task should include:

```text
Use TDD to implement <CARD-ID> as a Rust engine YAML DSL card.

Context:
- Printed text: <effect/inherited/security text from data/cards.json>
- Required docs: AGENTS.md, docs/RUST_DSL_AGENT_GUIDE.md, docs/RUST_DSL_TEST_API.md, docs/RUST_ENGINE_API.md
- Existing examples to inspect: <YAML/test paths>
- Scout brief: <brief or "none">
- Known gaps to respect: <tracker entries or "none">
- Allowed write paths:
  - code/digimon-engine/cards/<set>/<CARD-ID>.yaml
  - code/digimon-engine/tests/cards_behavioral/<set>/<card_id_lower>.rs

RED:
- Add/enable this failing test: <exact path and behavior>
- Run: <targeted cargo test command>
- Expected failure: <missing behavior, not parser typo>

GREEN:
- Implement the minimal YAML/DSL/engine change.
- Preserve PendingSelection/action-mask visibility for choices.
- Do not use no-op raw_rust or auto-selection.
- Run: <targeted cargo test command>

TRACKERS:
- Do not edit trackers directly. Report any gap closure, narrowing, or new blocker for the orchestrator to apply.

Final response:
- Files changed
- RED command and observed failure
- GREEN command and observed pass
- Verdict: IMPLEMENTED | PARTIAL | BLOCKED
- Gap kind if blocked: engine | dsl | hybrid | rules | test | data
- Remaining gaps or concerns
```

## Audit Worker Template

Each `AUDIT` worker task should include:

```text
Audit <CARD-ID> for Rust DSL faithfulness and test coverage.

Context:
- Printed text: <effect/inherited/security text from data/cards.json>
- Existing YAML: code/digimon-engine/cards/<set>/<CARD-ID>.yaml
- Existing tests: code/digimon-engine/tests/cards_behavioral/<set>/<card_id_lower>.rs
- DCGO reference: <path/body or "absent">
- Required docs: docs/RUST_DSL_AGENT_GUIDE.md, docs/RUST_DSL_TEST_API.md, docs/RUST_ENGINE_API.md
- Allowed write path:
  - code/digimon-engine/tests/cards_behavioral/<set>/<card_id_lower>.rs

Audit:
- Walk every printed clause and compare it to YAML.
- Check optionality, branches, conditions, once-per-turn, source/target filters, and player-visible choices.
- Add missing behavioral tests when YAML is faithful but coverage is incomplete.
- Do not modify existing YAML in audit mode; report drift instead.

Final response:
- Verdict: AUDITED-OK | AUDITED-MISSING-TESTS | AUDITED-DRIFT | BLOCKED
- Files changed
- Tests added or run
- Drift diff proposal if any
- Gap kind if blocked: engine | dsl | hybrid | rules | test | data
```

## Batch Spec Compliance Reviewer Prompt

```text
Review this completed batch for spec compliance only.

Check:
- Printed card text is faithfully represented for the claimed scope.
- The task did not implement extra unrequested card behavior.
- Every player-visible choice is exposed through action masks or PendingSelection.
- Optionality, PASS, filters, event subjects, replacement causes, and OPT are covered where relevant.
- AUDIT-mode YAML drift is reported, not silently rewritten.
- Gap trackers accurately distinguish closed, partial, and still-open work.

Return findings first with file/line references. If no issues, say "Spec compliant" and list residual unclaimed scope per card.
```

## Batch Code Quality Reviewer Prompt

```text
Review this completed batch for code quality and maintainability.

Check:
- YAML follows existing DSL idioms and docs/RUST_DSL_AGENT_GUIDE.md.
- Rust changes use EffectContext/DSL lowering rather than reaching around engine APIs.
- Tests are minimal, behavioral, and not overfit to implementation details.
- Shared surfaces such as selection/action masks, replacement flow, event payloads, and formulas remain coherent.
- No unrelated refactors or metadata churn.
- Worker outputs stayed within assigned files; shared registration and trackers are orchestrator-owned.

Return findings first with file/line references. If no issues, say "Code quality approved".
```

## Batch Merge And Verification

After each batch:

1. Copy or apply only the approved YAML and per-card test files.
2. Reconcile pre-wired `mod.rs` and `main.rs` registrations.
3. Run targeted tests:
   - `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- <set_or_card_filter> --nocapture`
4. If targeted tests fail, run one bounded fix pass for the same card paths. If they still fail, stop and report the failure.
5. Update `qa/qa-reports/validated_cards_dsl.json` if present.
6. Append gap entries to `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, or `qa/archetype-qa/engine-gaps.md` as appropriate.
7. Append a batch row/table to `qa/archetype-qa/dsl/<archetype_slug>.md` or the relevant QA artifact.
8. Record the batch summary for final reporting, but continue to the next batch immediately:

```text
Batch <N> complete (<processed>/<total> cards)
| Card ID | Mode | Verdict | Review | Tests | Notes |
| ... |

Running totals: IMPLEMENTED=<n> AUDITED-OK=<n> PARTIAL=<n> BLOCKED=<n>
```

Only surface this table before the final response when the batch is blocked, review found unresolved issues, or tests still fail after the bounded fix pass.

## Final Integration Review

After all batches:

1. Re-run targeted tests for every changed card and primitive.
2. Run broader suites for touched shared surfaces:
   - DSL parser/lowering: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- <pattern> --nocapture`
   - Selection/action masks: relevant `selection`, `mask_and_tensor`, or `action` tests.
   - Card behavior: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- <card_or_set> --nocapture`
3. Re-scan for placeholders:
   - `rg "process: \\[\\]|raw_rust|TODO|BLOCKED" code/digimon-engine/cards/<sets> code/digimon-engine/tests/cards_behavioral/<sets>`
4. Run `git diff --check`.
