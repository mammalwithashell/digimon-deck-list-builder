Worker-adapter fixtures for `test_card_loop_workers_*.py`.

Recorded (real CLI output, 2026-10-04, task 3.7 smoke):
- `claude_envelope_not_logged_in.json` — claude 2.1.195 `-p --output-format json`, CLI not logged in.
- `codex_events_ok.jsonl` — codex-cli 0.159.2 `exec --json`, trivial `{"ok": true}` task.
- `codex_events_triage.jsonl` — same, with `schemas/triage.json` as `--output-schema`.
- `codex_events_sandbox.jsonl` — same, two shell writes under `-s workspace-write`
  (outside the workspace denied, `--add-dir` allowed).

Synthesized (`*_synth.*`) — shapes taken from the CLI's own code (claude: result
message fields `structured_output`, `total_cost_usd`, `usage`, `modelUsage`,
`num_turns`; subtypes `success`, `error_max_budget_usd`, `error_max_turns`,
`error_during_execution`, `error_max_structured_output_retries`), values invented.
Replace with recordings once the success / budget / quota paths are observed.
