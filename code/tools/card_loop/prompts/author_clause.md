version: 1
# Author an exam scenario for {clause_id}

You are one stateless worker in the card-authoring loop. Do exactly this task in
your working directory, then return the JSON result. Do not invoke slash-command
skills and do not spawn sub-agents.

## The clause
{card_name} ({card_id}), clause `{clause_id}` ({clause_label}):
> {clause_text}

## Task
Write ONE scripted exam scenario, `{scenario_path}` with `clause: {clause_id}`,
whose line makes both our engine and DCGO resolve this clause. A line that never
fires the clause covers nothing.
- Format, step vocabulary, prompt kinds, deck stacking and asserts: ask the exam
  MCP. `exam_authoring_guide` (topics `format`, `steps`, `prompts`, `decks`,
  `assert`), and `exam_keyword_brief` for each keyword the line meets: {keywords}.
- Imitate the scenarios and notes already in `{scenario_dir}` ({notes_files});
  reuse their deck books.
- Predict every prompt DCGO asks (each step's `expect:`), optional yes/no gates
  above all, from the printed text in `{bundle_path}` and DCGO's script
  `{dcgo_script}`.
- Put `meta: {{produced_by: {attempt_id}}}` in the scenario. Leave `assert:`
  empty: the oracle backfills it.
- Validate with `exam_validate`, then dry-run with `exam_probe` (sim-only;
  `inspect_step` shows our live prompt). Do not run the oracle.
- Rules: `docs/digimon-rules/` (`keyword-semantics.md` first).
{previous}
## Result
Return only JSON matching `author_clause`:
`{"scenario_paths": ["{scenario_path}"], "covers": ["{clause_id}"], "notes": "..."}`
If no legal line reaches the clause, return no paths and say why in `notes`.
