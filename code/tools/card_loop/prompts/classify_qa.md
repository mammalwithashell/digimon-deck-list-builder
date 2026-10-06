version: 1
# Classify an official ruling before it is examined

You are one stateless worker in the card-authoring loop. Read only: do not edit
any file. Do not invoke slash-command skills and do not spawn sub-agents. Another
model family answers this same packet independently; neither sees the other.

{task}

Read the cards' official bundles (listed under References) and
`docs/digimon-rules/` as needed. Cite only what you read.

## Inputs
```json
{inputs_json}
```

## Result
Return only JSON matching `classify_qa`:
`{"q_id": "...", "classification": "behavioral" or "textual" or "not_examinable", "reasoning": "...", "citation": "qa:<Q-number> ...", "examined_clauses": [clause ids from the inputs]}`
