"""card-loop: drive Claude / Codex CLI workers to implement Digimon cards, fix gaps,
and adjudicate every clause and gating interaction against the DCGO oracle.

Design and requirements: openspec/changes/add-card-authoring-loop/.

Module ownership (keep cross-module imports to `config` and `contracts`):
    workset.py, preflight.py     inputs -> work set; new-set preflight
    workers/                     claude / codex / fake adapters, worktree pool
    ledger.py, scorecard.py,
    router.py, audit.py          attempts, provenance, corrections, routing, audit
    interactions/                Q&A join, probe generator, denominator, outcomes
    driver.py, state.py, stages/ the orchestrator (after the readiness plans land)
"""
