version: 2
# Widen the engine or DSL substrate to fix {item_id}

You are one stateless worker in the card-authoring loop. Do exactly this task in
your working directory, then return the JSON result. Do not invoke slash-command
skills and do not spawn sub-agents. This change lands on its own branch for human
review.

## Why
The exam `{item_id}` ({subject}) diverged from DCGO and our side is wrong in a way
the card spec alone cannot fix:
{triage}
Divergence:
```json
{divergence_json}
```
Scenario(s): {scenario_paths}
Known gaps: {gaps}

## Task
Add the missing DSL vocabulary (lowered in `code/digimon-dsl/`) or engine
primitive (`code/digimon-engine/src/`), then make the card use it. Widen the
substrate; do not route around it (CLAUDE.md rule 28).
- A test that FAILS at the current base and passes with the change: an engine or
  DSL unit test, or a DebugRunner test (`docs/RUST_DSL_TEST_API.md`). Engine API:
  `docs/RUST_ENGINE_API.md`.
- Clone safety: no closure-based pending selection; selections go through the
  resumable VM (CLAUDE.md rule 28).
- Log what you closed in `qa/dsl-vocab-gaps.md` or `docs/RUST_ENGINE_GAPS.md`.
- Cite why the old behaviour was wrong: a `general_rule.pdf` section
  (`docs/digimon-rules/`), a ruling `qa:<Q-number>`, or DCGO {dcgo_scripts}
  `:<line>`. No citation, no fix.
- No approximations: no stubs, no auto-selections, no special cases for one card.
- A YAML spec you change keeps `# produced_by: {attempt_id}` as its first line.
- Run the scoped tests and paste the verbatim `test result:` lines. List under
  `gaps` anything still missing after your change.
{previous}
## Result
Return only JSON matching `fix_engine`:
`{"files": [...], "tests": [...], "test_result_lines": [...], "gaps": [{"kind": "dsl" or "engine", "id": "...", "summary": "..."}], "citation": {"kind": "rule" or "ruling" or "dcgo", "ref": "..."}, "notes": "..."}`

## Worktree hygiene

Keep scratch out of the worktree: write temporary files (notes, probes, helper
scripts) under the system temp directory, never inside the repository. Only
the files you deliver may be left behind; anything else is dropped at merge.
