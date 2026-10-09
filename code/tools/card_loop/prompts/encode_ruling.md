version: 4
# Encode an official ruling as an expect_ruling block

You are one stateless worker in the card-authoring loop. Read only: do not edit
any file. Do not invoke slash-command skills and do not spawn sub-agents.

{task}

The block is the EXPECTED VALUE of a three-way comparison between our engine,
DCGO and the publisher's ruling, so it must say exactly what the answer decides.
Use only the projection keys in the inputs' `assertion_keys` (security is a
count). `at` is the scenario step index the observable is checked after, as in a
scenario's `assert:` (exam MCP `exam_authoring_guide`, topic `assert`). The
scenario in the inputs is the line written to exercise THIS ruling: find the
step at which the question's situation resolves and anchor there. If no step
reaches it, say so with `line_exercises_ruling: false` -- the line goes back to
its author with your reasoning -- rather than asserting setup state. An
`earlier_rejection` in the inputs is a verifier's objection to a previous block
on this same line: do not repeat it. Rules: `docs/digimon-rules/`.

## Inputs
```json
{inputs_json}
```

## Result
Return only JSON matching `encode_ruling`:
`{"q_id": "...", "mode": "author" or "verify", "expect_ruling": {"assert": [{"at": 0, "that": [{"key": "p0.memory", "value_json": "3"}]}]} or null, "agrees": true or false or null, "line_exercises_ruling": true or false or null, "answer_quote": "...", "reasoning": "..."}`

## What makes an encoding valid

The block must DISCRIMINATE: pick the observable that comes out differently
under the publisher's answer and under the opposite reading of the question,
at the step right after the ruling applies (a count, a zone's contents, a DP,
a prompt offered or not offered). A block that only restates the setup -- the
memory the line starts at, the cards stacked in hand -- is satisfied under
every reading and encodes nothing; the verifier rejects it. In `verify` mode,
agree only if the block would FAIL under the opposite reading; explain which
reading it fails to separate otherwise.

## Worktree hygiene

Keep scratch out of the worktree: write temporary files (notes, probes, helper
scripts) under the system temp directory, never inside the repository. Only
the files you deliver may be left behind; anything else is dropped at merge.
