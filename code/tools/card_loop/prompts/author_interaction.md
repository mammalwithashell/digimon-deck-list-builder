version: 2
# Author an adversarial interaction exam: {interaction_id}

You are one stateless worker in the card-authoring loop. Do exactly this task in
your working directory, then return the JSON result. Do not invoke slash-command
skills and do not spawn sub-agents.

Your goal is to FIND A DIVERGENCE: a legal line on which our engine and DCGO (or
our engine and the official ruling) resolve this interaction differently. Write
the line most likely to expose a wrong scope, a missing decline, a missed trigger
or a wrong timing -- not the easiest line that agrees.

## The interaction
{target}
Cards: {cards}
Clauses:
{clauses}

## Task
Write the scenario at `{scenario_path}` with
`interaction: {{id: "{interaction_id}", source: {source}, kind: {kind}}}` and
`covers:` listing every clause the line exercises.
{kind_rule}
{ruling_rule}
- Extend an oracle-confirmed clause line from `{scenario_dir}` where you can
  ({notes_files}); suggested base: {base_scenario}
- Format and vocabulary: exam MCP `exam_authoring_guide` (topics `format`,
  `steps`, `prompts`, `decks`, `assert`) and `exam_keyword_brief` for {keywords}.
  Rules: `docs/digimon-rules/`.
- Put `meta: {{produced_by: {attempt_id}}}` in the scenario. Leave `assert:` for
  the oracle to backfill.
- Validate with `exam_validate` (it rejects an interaction id outside the
  denominator), then dry-run with `exam_probe` (sim-only, `inspect_step`). Do not
  run the oracle.
{previous}
## Result
Return only JSON matching `author_interaction`:
`{"scenario_paths": [repo-relative paths], "covers": [clause ids], "notes": "..."}`
If no legal line reaches the interaction, return no paths and say why in `notes`;
the loop measures it before anything is called unreachable.

## Worktree hygiene

Keep scratch out of the worktree: write temporary files (notes, probes, helper
scripts) under the system temp directory, never inside the repository. Only
the files you deliver may be left behind; anything else is dropped at merge.
