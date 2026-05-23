## 1. Establish Current BG Imperial Pool

- [x] 1.1 Derive the unique `BG Imperial` card IDs from `data/deck_library.json` and compare them with `qa/qa-reports/validated_cards_dsl.json`.
- [x] 1.2 Document any pool or archetype-owner mismatch, including `BT17-077` appearing in the deck-library pool while being canonically tracked under another ledger archetype.
- [x] 1.3 Confirm every pool card has a YAML file under `code/digimon-engine/cards/`.

## 2. Reconcile Stale Readiness Signals

- [x] 2.1 Audit BG Imperial YAML and behavioral-test files for stale `PARTIAL`, `BLOCKED`, approximation, ignored-test, and raw-rust comments.
- [x] 2.2 Rewrite stale comments so resolved gaps read as resolved history, not live blockers.
- [x] 2.3 Update BG Imperial QA trackers under `qa/archetype-qa/` to match current implementation state.
- [x] 2.4 Update `qa/dsl-vocab-gaps.md` and `qa/resolved-gaps.md` only where BG Imperial entries still describe resolved substrate as open.
- [x] 2.5 Update `qa/qa-reports/validated_cards_dsl.json` for BG Imperial cards whose focused tests support an `IMPLEMENTED` verdict.

## 3. Raw-Rust Audit

- [x] 3.1 Scan all BG Imperial pool YAML files for non-comment `raw_rust` clauses, steps, or formulas.
- [x] 3.2 Record the zero-live-raw-rust result in BG Imperial readiness notes.
- [x] 3.3 Document adjacent raw-rust functions outside the BG Imperial pool, such as `BT13-040`, as separate follow-up rather than BG Imperial blockers.

## 4. Verification

- [x] 4.1 Run focused `cards_behavioral` tests for every card whose ledger status or stale-blocker language changes.
- [x] 4.2 Include focused verification for `BT21-037` if its ledger entry changes from `PARTIAL` to `IMPLEMENTED`.
- [x] 4.3 Include focused verification for `BT17-077` when citing it as covered by the BG Imperial deck-library pool.
- [x] 4.4 Run `openspec status --change reconcile-bg-imperial-dsl-readiness` and confirm the change remains apply-ready.

## 5. Final Review

- [x] 5.1 Review `git diff` to ensure no unrelated source behavior, action-space, tensor, PyO3, frontend, or model metadata changes were introduced.
- [x] 5.2 Summarize remaining follow-ups, especially any non-BG Imperial raw-rust DSL gaps, without expanding this change's scope.
