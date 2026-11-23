# Subagent TDD Workflow

Use this reference after the target card pool is resolved and tasks are sliced.

## Controller Rules

- For IMPLEMENT and AUDIT, keep YAML, existing tests, scout implementation analysis and DCGO bodies out of the initial worker context. First write and run bundle/rules-derived `card_text/<id>.rs` tests; only then release deferred references. Scout briefs must not embed YAML for the first phase.

- Dispatch one fresh implementer subagent per independent task.
- Provide exact task text; do not make implementers rediscover the whole archetype.
- Do not dispatch parallel tasks that touch the same YAML, Rust module, test module, selection/action internals, or tracker section.
- Review each task in two stages: spec compliance first, code quality second.
- Keep the main session responsible for integration, tracker consistency, and final verification.

If subagents are unavailable in the current environment, do not silently downgrade the workflow. Tell the user and ask whether to execute the same TDD gates locally.

## Task Slice Template

Each implementer task should include:

```text
Use TDD to implement <CARD-ID> <clause/primitive>.

Context:
- Printed text and official Q&A: data/card_bundles/<CARD-ID>.md
- Required docs: AGENTS.md, docs/RUST_DSL_AGENT_GUIDE.md, docs/RUST_DSL_TEST_API.md, docs/RUST_ENGINE_API.md
- Deferred examples (open only after first test run): <YAML/test paths>
- Known gaps to respect: <tracker entries or "none">

RED (before reading YAML or existing tests):
- Write code/digimon-engine/tests/cards_behavioral/card_text/<card_id_lower>.rs from the bundle and rules alone. Quote printed sections and official Q&A; cover every clause, every may-decline and each Q&A through real actions. Follow card_text/bt17_015.rs and docs/RUST_DSL_TEST_API.md, Card-text tests.
- Assert and answer each prompt with explicit helpers; no auto_resolve*, wrappers or enqueue_triggered. Structural assertions are optional and discouraged.
- First test behavior: <exact printed clause>
- Run: <targeted cargo test command>
- Expected failure: <missing behavior, not parser typo>

GREEN (only after recording the first test run):
- Obtain deferred scout context, open references/YAML/old tests, then implement the minimal change.
- Preserve PendingSelection/action-mask visibility for choices.
- Do not use no-op raw_rust or auto-selection.
- Run: <targeted cargo test command>

TRACKERS:
- Update <docs/RUST_ENGINE_GAPS.md or qa/dsl-vocab-gaps.md> if the reusable gap closes or narrows.

Final response:
- Files changed
- RED command and observed failure
- GREEN command and observed pass
- Remaining gaps or concerns
```

## Spec Compliance Reviewer Prompt

```text
Review this completed task for spec compliance only.

Check:
- Printed card text is faithfully represented for the claimed scope.
- The task did not implement extra unrequested card behavior.
- Every player-visible choice is exposed through action masks or PendingSelection.
- Optionality, PASS, filters, event subjects, replacement causes, and OPT are covered where relevant.
- Gap trackers accurately distinguish closed, partial, and still-open work.

Return findings first with file/line references. If no issues, say "Spec compliant" and list residual unclaimed scope.
```

## Code Quality Reviewer Prompt

```text
Review this completed task for code quality and maintainability.

Check:
- YAML follows existing DSL idioms and docs/RUST_DSL_AGENT_GUIDE.md.
- Rust changes use EffectContext/DSL lowering rather than reaching around engine APIs.
- Tests are minimal, behavioral, and not overfit to implementation details.
- Shared surfaces such as selection/action masks, replacement flow, event payloads, and formulas remain coherent.
- No unrelated refactors or metadata churn.

Return findings first with file/line references. If no issues, say "Code quality approved".
```

## Integration Review

After all task slices:

1. Re-run targeted tests for every changed card and primitive.
2. Run broader suites for touched shared surfaces:
   - DSL parser/lowering: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl -- <pattern> --nocapture`
   - Selection/action masks: relevant `selection`, `mask_and_tensor`, or `action` tests.
   - Card behavior: `cargo test --manifest-path code\digimon-engine\Cargo.toml --test cards_behavioral -- <card_or_set> --nocapture`
3. Re-scan for placeholders:
   - `rg "process: \\[\\]|raw_rust|TODO|BLOCKED" code/digimon-engine/cards/<sets> code/digimon-engine/tests/cards_behavioral/card_text`
4. Run `git diff --check`.
