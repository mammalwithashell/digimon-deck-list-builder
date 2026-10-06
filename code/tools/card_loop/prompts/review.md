version: 1
# Review the implementation of {card_name} ({card_id})

You are one stateless reviewer in the card-authoring loop. Read only: do not edit
any file. Do not invoke slash-command skills and do not spawn sub-agents.

Another model implemented this card (attempt `{implementer_attempt}`). Its whole
change is the diff `{diff_path}` (manifest `{manifest_path}`). The spec is
`{spec_path}`; the tests are `{test_path}`.

Check it against the printed text in `{bundle_path}` and DCGO's behaviour in
`{dcgo_script}`:
- every printed clause is implemented, with its timing, its optionality ("you
  may", "up to"), its scope (this Digimon / 1 of your Digimon / all) and any
  once-per-turn limit;
- no approximation: no stub, no auto-selected choice, no clause silently dropped;
- the tests assert the card's behaviour (not just that it loads), include the
  decline path where the card says "you may", and would fail without the spec;
- the reported test lines are plausible for the tests in the diff.
Rules: `docs/digimon-rules/`. DSL test API: `docs/RUST_DSL_TEST_API.md`.

The implementer reported:
{implementer_report}

## Result
Return only JSON matching the `review` schema:
`{"verdict": "accept" or "reject", "directives": [{"path": "<repo path>" or null, "directive": "..."}], "summary": "..."}`
Reject only for a concrete defect, with one directive per required change.
