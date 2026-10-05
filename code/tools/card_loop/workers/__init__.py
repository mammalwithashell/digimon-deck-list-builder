"""card-loop worker adapters (design D10, spec `agent-cli-workers`).

    base.py    Worker protocol, schema validation, run_with_retry, capture_artifacts
    fake.py    FakeWorker — canned results for driver tests
    claude.py  ClaudeWorker  — `claude -p --output-format json --json-schema …`
    codex.py   CodexWorker   — `codex exec … --output-schema … --json`
    pool.py    WorktreePool  — pinned, reset-before-use worktrees, per-worktree CARGO_TARGET_DIR
    health.py  VendorHealth  — quota exhaustion disables a family for the run

Submodules are imported explicitly (`from tools.card_loop.workers.claude import
resolve_claude_exe`); this package init stays import-light.
"""
