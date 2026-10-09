version: 3
# Fix {card_name} ({card_id}) so it resolves like the cited source

You are one stateless worker in the card-authoring loop. Do exactly this task in
your working directory, then return the JSON result. Do not invoke slash-command
skills and do not spawn sub-agents.

## Why
The exam `{item_id}` diverged from DCGO and triage judged OUR side wrong:
{triage}
Divergence:
```json
{divergence_json}
```
Scenario(s): {scenario_paths}

## Task
Change the card's YAML spec `{spec_path}` (or its card data) so the card resolves
as the cited source says, and change nothing else.
- First add a regression test to `{test_path}` that FAILS on the current spec and
  passes with your fix (DebugRunner, `docs/RUST_DSL_TEST_API.md`). The fix gate
  re-runs it both ways.
- Cite why the old behaviour was wrong: a `general_rule.pdf` section
  (`docs/digimon-rules/rules-index.json`), an official ruling `qa:<Q-number>`, or
  DCGO `{dcgo_script}:<line>`. No citation, no fix.
- No approximations: no stubs, no auto-selections, no special-casing the scenario.
- If the fix needs DSL vocabulary or an engine primitive that does not exist, do
  not route around it: report it under `gaps` and change nothing else.
- Keep the spec's provenance header on this attempt: `# produced_by: {attempt_id}`.
- Run the new test and the card's existing tests; paste the verbatim
  `test result:` lines.
Printed text: `{bundle_path}`. Rules: `docs/digimon-rules/`.
{previous}
## Result
Return only JSON matching `fix_card`:
`{"files": [...], "tests": [...], "test_result_lines": [...], "gaps": [{"kind": "dsl" or "engine", "id": "...", "summary": "..."}], "citation": {"kind": "rule" or "ruling" or "dcgo", "ref": "..."}, "notes": "..."}`

## Where the fix lives, and what the merge checks

Edit the card's spec at its canonical path `code/digimon-engine/cards/<set>/<ID>.yaml`
(the set directory the id names; never `cards/_examples/`, which holds curated
samples). The merge compiles every YAML into the card pack and refuses a spec
that does not parse, so before returning run `cargo run -p dsl-lint -- <that
file>` and the card's behavioral test, and paste their result lines.

## Worktree hygiene

Keep scratch out of the worktree: write temporary files (notes, probes, helper
scripts) under the system temp directory, never inside the repository. Only
the files you deliver may be left behind; anything else is dropped at merge.
