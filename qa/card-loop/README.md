# card-loop ledgers

Cross-run truth for the card-loop model scorecard (OpenSpec change
`add-card-authoring-loop`, design D11/D12). Per-run state lives in the
git-ignored `runs/card-loop/<run-id>/`; these files are committed.

All three are **append-only JSONL**: one compact JSON object per line, UTF-8,
LF endings. `.gitattributes` gives each `merge=union`, so runs on different
branches merge by concatenating rows instead of conflicting. Never rewrite or
reorder a row; a later judgement is a new row. Readers tolerate unknown keys
and skip (and report) damaged lines.

| File | Written by | One row per |
|---|---|---|
| `attempts.jsonl` | the driver, via `tools.card_loop.ledger` | worker call (Claude or Codex) |
| `corrections.jsonl` | the driver, via `tools.card_loop.corrections` | correction of an earlier attempt |
| `audits.jsonl` | a human, via `python -m tools.card_loop audit record` | human verdict on a sampled output |
| `escalations/` | the driver (group 6) | item the loop could not adjudicate |

## attempts.jsonl

`attempt_id, run_id, ts, stage, item, family, model, effort, prompt_version,
assignment, outcome, usage{input_tokens, output_tokens, cache_read_tokens,
cache_write_tokens, cost_usd, cost_derived, wall_seconds}`, plus optional
`parent_attempt`, `artifacts`, `notes`.

- `assignment`: `routed` (stage default or scorecard winner), `explore` (the
  seeded ~20% cross-assignment), `forced` (an interaction exam routed away from
  the card's implementer family).
- `outcome`: `accepted | rejected | gate_failed | schema_invalid | error |
  quota_exhausted | escalated` — the call's final *immediate* outcome.

## corrections.jsonl

`ts, corrected_attempt, by_attempt | by_human | by_gate, kind, latency, stage,
item?, detail`. `latency` is fixed by `kind`:

- immediate: `review_reject`, `gate_fail`, `schema_invalid`, `family_disagreement`
- late: `later_edit` (git blame through `Loop-Attempt:` trailers),
  `verdict_overturned`, `escalation_resolved_against`, `clause_not_exercised`,
  `fix_reverted`, `false_alarm`

## audits.jsonl

`ts, attempt_id, verdict (agree|disagree), sampled, stage, item, family, model,
prompt_version, auditor?, note?`. The sample is deterministic: an accepted
attempt is audited iff a seeded hash of its id falls below `audit_rate` (5%).
Work the queue with `python -m tools.card_loop audit next` and
`audit record <attempt_id> agree|disagree --note "..."`.

## Reading them

`python -m tools.card_loop report models [--since 30d] [--stage S] [--json]`
prints per stage x model x prompt-version attempts, first-pass acceptance,
immediate / late correction rates, cost per accepted-and-uncorrected unit,
Wilson intervals, audit precision, reviewer precision, and author acceptance
with and without each reviewer.
