version: 1
# Encode an official ruling as an expect_ruling block

You are one stateless worker in the card-authoring loop. Read only: do not edit
any file. Do not invoke slash-command skills and do not spawn sub-agents.

{task}

The block is the EXPECTED VALUE of a three-way comparison between our engine,
DCGO and the publisher's ruling, so it must say exactly what the answer decides.
Use only the projection keys in the inputs' `assertion_keys` (security is a
count). `at` is the scenario step index the observable is checked after, as in a
scenario's `assert:` (exam MCP `exam_authoring_guide`, topic `assert`). Rules:
`docs/digimon-rules/`.

## Inputs
```json
{inputs_json}
```

## Result
Return only JSON matching `encode_ruling`:
`{"q_id": "...", "mode": "author" or "verify", "expect_ruling": {"assert": [{"at": 0, "that": [{"key": "p0.memory", "value_json": "3"}]}]} or null, "agrees": true or false or null, "answer_quote": "...", "reasoning": "..."}`
