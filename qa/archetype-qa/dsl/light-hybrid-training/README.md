# Local Light Hybrid training admission

The exact local DCGO list passes the simulator training gate: **20/20 unique IDs, 54/54 copies (4 eggs and 50 main-deck cards)**. Mechanics readiness, compiled registration plus production data, and the public card-admissibility validator all pass on the final wheel.

## Exact input

- Local file: `C:/Users/james/Documents/DCGO_Application/Assets/Decks/LightHybrid_ZWd06BaFMXQ.txt`.
- Byte-identical frozen list: [dcgo-deck.txt](dcgo-deck.txt).
- SHA256: `92ec23d1c154591f50c0ced36ffbede106f9bb84d7ef01d810e5485b22994091`.
- Branch: `feat/light-hybrid-training`, based on fetched main `ec360e352`.
- Final public gate result: [final-exact-deck-admission.json](verification/final-exact-deck-admission.json).

## Implementation and evidence

Nineteen card YAMLs were authored; BT16-082 already existed. Production data, overrides and all 20 split records agree. P-249 was published through the supported registry/data workflow.

The four former blockers BT18-037, BT18-042, BT18-063 and BT18-076 are implemented and promoted after clean aggregate qualification. Their canonical modules pass 17, 19, 20 and 20 tests. The complete card pool has 206 canonical tests, with additional shared controls. Nine active capacity witnesses cover source positions 11/12/39/300 and security positions 9/10/39.

The coordinated source/security chooser preserves legal masks, physical identity through compaction, privacy, clone/resume behavior and stale-token rejection. Typed receipts ensure cost rewards require actual completed payment. All affected selector families and consumers were migrated.

Final verification:

- All **93 required suites pass**, recorded in [frozen-final-tier2-clean-run.log](verification/logs/frozen-final-tier2-clean-run.log).
- Both full-card invocations pass **12729 tests, zero failures, 35 existing ignores** each.
- Frozen production source passes all **507 engine unit tests**.
- Final release wheel passes **123 normal-stack Python/native/environment/export checks**.
- Four final-wheel exact-deck seeded games terminate without errors or truncation.
- All **10 active replay goldens / 635 actions** verify. Three affected recordings were regenerated with supported tooling after explicit diagnostics; byte-identical originals, reasons and hashes are retained under `qa/replay-goldens/retired/2026-10-25-owned-choice-p249`.
- Final post-promotion freshness checks: [promoted-final-tier0-full-run.log](verification/logs/promoted-final-tier0-full-run.log).

Historical RED logs and earlier 16/20 snapshots are retained as provenance. They do not override the final admission report. The original two unrelated P-092 exam edits are preserved.

## Training environment and contracts

Use the isolated environment `C:/Users/james/.codex/build-targets/light-hybrid-training-venv` with `PYTHONPATH=code` to select this checkout. Its installed final wheel SHA256 is `5ddd98b9124c86d74d86cd1eb2c716d25b02330784eadff9887e65d775c53fa9`; the global Python wheel was not replaced.

The training CLI reads this deck format with `--deck1 qa/archetype-qa/dsl/light-hybrid-training/dcgo-deck.txt`. Its explicit-deck validator checks registration; the independent public mechanics/card-admissibility checks establish the admission reported here.

Action space remains 2192 with semantic schema 2. Pending-choice rows are 128 floats. Lite/deck/full profiles are 9434/9874/44506 floats, resolved through exported layout metadata. Pack format is 0.32.0. Old model metadata must not be relabeled to authorize old weights. Fresh compatible untrained recurrent actor/critic exports pass masking and episode-reset checks.

This establishes simulator training eligibility. No new live DCGO oracle certification, production training run or trained-policy quality promotion is claimed. Evidence and migration tasks are tracked in `openspec/changes/support-complete-digivolution-source-choices/tasks.md`.
