version: 3
# Triage a divergence: {item_id}

You are one stateless worker in the card-authoring loop. Read only: do not edit
any file. Do not invoke slash-command skills and do not spawn sub-agents. Your
answer can end this item, so another model family answers the same question
independently: argue only from sources you can cite.

## The divergence
Our engine and DCGO ran the same scripted line and disagreed.
- Exam: {subject}
- Scenario(s): {scenario_paths}
- Oracle result:
```json
{oracle_json}
```
{prompt_evidence}
## Sources, in priority order
1. `general_rule.pdf`, through `docs/digimon-rules/` (`rules-index.json` names the
   pages). It outranks DCGO on rules.
2. DCGO C#: {dcgo_scripts} -- the authority for how a card resolves.
3. Printed text: {bundles} (official Bandai DB). Official rulings:
   `data/card_qa.json`.
Replay our side with the exam MCP `exam_probe` (sim-only, `inspect_step: N`) to
see our prompt and board at the divergent step.

Two shapes of finding carry no field divergence and are still divergences:
- A line DCGO STOPPED (its `reason` names a prompt, actor or candidate
  mismatch; the prompt evidence above spells out who asked what): the engines
  disagree about that prompt -- one asks it, the other does not, or they offer
  different candidates. Decide who is right about asking it there. Do not
  answer "nothing to classify": the stopped line is the finding.
- A `reason` of `ours contradicts ruling qa:<Q>`: our engine against the
  publisher's own answer, which outranks DCGO. `ours_wrong` unless the ruling
  is misread (then say exactly which words).

## Classify
- `ours_wrong`: our engine contradicts a correct rule, ruling or DCGO behaviour.
- `dcgo_quirk`: our engine follows the rules or ruling and DCGO does not, or the
  two differ only in a rules-neutral way.
- `unreachable`: no legal line reaches the clause the way the exam needs; state
  what you measured.
- `undetermined`: the sources do not decide it.
A `dcgo_quirk` or `unreachable` call without a citation is not accepted.

## Result
Return only JSON matching `triage`:
`{"classification": "...", "citation": {"kind": "rule" or "ruling" or "dcgo", "ref": "16-36" or "qa:Q1601" or "<path>.cs:<line>"} or null, "reasoning": "..."}`

## Worktree hygiene

Keep scratch out of the worktree: write temporary files (notes, probes, helper
scripts) under the system temp directory, never inside the repository. Only
the files you deliver may be left behind; anything else is dropped at merge.
