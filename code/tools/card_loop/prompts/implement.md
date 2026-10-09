version: 2
# Implement {card_name} ({card_id}) as a YAML DSL card

You are one stateless worker in the card-authoring loop. Do exactly this task in
your working directory, then return the JSON result. Do not invoke slash-command
skills and do not spawn sub-agents.

## Task
{mode}

- Spec: `{spec_path}` -- the YAML DSL spec (CLAUDE.md rule 28: DSL-first). Its
  first line is `# produced_by: {attempt_id}`.
- Tests: `{test_path}` -- DebugRunner behavioral tests, written FIRST so they fail
  before the spec exists (CLAUDE.md rule 18). Register the module beside its set's
  other tests.
- Printed text, traits and costs: `{bundle_path}` (official Bandai DB; it outranks
  `data/cards.json`).
- Behaviour: DCGO C# `{dcgo_script}` (read-only reference).
- Nearby specs to imitate: {examples}
- DSL vocabulary and test helpers: `docs/RUST_DSL_AGENT_GUIDE.md`,
  `docs/RUST_DSL_TEST_API.md`, `docs/RUST_ENGINE_API.md`. Rules and keyword
  semantics: `docs/digimon-rules/` (`keyword-semantics.md` first).

## Non-negotiables
- No approximations: implement every clause of the printed text. No stubs, no
  auto-selections; every choice the card offers is a pending selection the player
  answers.
- If the DSL or the engine cannot express a clause, do not route around it and do
  not hand-write a `raw_rust` effect. Report it under `gaps` (`dsl` for
  `qa/dsl-vocab-gaps.md`, `engine` for `docs/RUST_ENGINE_GAPS.md`; reuse a tracker
  id when one exists). A gap parks the card; that is the correct outcome.
- Run the tests you wrote and paste the verbatim `test result:` lines. Never
  paraphrase a result or report a run you did not make.
{directives}
## Result
Return only JSON matching the `implement` schema:
`{"files": [repo-relative paths], "tests": [test names], "test_result_lines": [verbatim lines], "gaps": [{"kind": "dsl" or "engine", "id": "...", "summary": "..."}], "notes": "..."}`

## Worktree hygiene

Keep scratch out of the worktree: write temporary files (notes, probes, helper
scripts) under the system temp directory, never inside the repository. Only
the files you deliver may be left behind; anything else is dropped at merge.
