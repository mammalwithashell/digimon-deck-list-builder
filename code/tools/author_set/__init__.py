"""Release-set authoring workflow tooling.

Deterministic primitives behind the `/author-set <SET>` Workflow:

- `set_resolver`    — set prefix -> distinct card-ID list (Phase 1)
- `ingest_diff`     — pull + diff + merge against cards.json (Phase 1)
- `dcgo_manifest`   — DCGO keyword-surface extractor + Rust-enum gap diff (Phase 2)
- `lexicons`        — complete trait + card-name lexicons for denoising (Phase 2)
- `keyword_gate`    — bracket scan -> set-subtraction -> denoise -> DCGO triage (Phase 2)

See `openspec/changes/add-author-set-workflow/` for the spec + design.
"""
