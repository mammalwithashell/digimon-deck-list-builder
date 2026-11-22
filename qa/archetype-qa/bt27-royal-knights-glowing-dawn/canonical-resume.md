# Canonical admission resume — 2026-11-22

Started from verified local `3e733fe779f54e215807374a3d7d87a096fd67e7` (clean), still based on `349816e5c45a120629ba668ffdd6810a67554642`. No Library restoration was necessary.

Read-only upstream probe fetched main `dfde14aff94a1018780671ea8631cf05c402ddf3`. No changes under `code/tools/card_data` or to `data/card_registry.json`; no tracked canonical/revision/projection/scope manifests. Upstream changes 33 existing cards' colors/evolution costs, adding no IDs. Those unrelated metadata changes were not adopted. No rebase or remote snapshot overwrite.

`scope_merge.py` can retain observations and recompute affected cards, but production `cli.py::_require_candidate_semantic_readiness` requires a ready complete-full-pool candidate. `complete_candidate.py::_scope_key` requires both full-pool sources. The assembler normalizes all source rows and refreshes official conditions; source-admission transition retention does not preserve old compatibility condition selections. Neither the pinned base nor current main provides the marker-owned canonical baseline needed to justify a set-local bypass. No gate or allocator changed.

Acquisition command (review-only):
```
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=code python code/tools/bt27_gate_acquire.py --output-dir ../bt27-canonical-acquisition
```
The helper uses official/secondary repository adapters and bounded retry. Exact successful response bodies are SHA-256-addressed outside the repo; a URL index explicitly pins their original acquisition times. Re-running reuses those verified bytes, never labels them as freshly fetched. Secondary requests preserve the repository's rate-limit spacing. Official concurrency is bounded at three. Parser/completeness errors remain failures. Raw caches, temporary review trees and synthetic admission artifacts are excluded from publishable commits.

Acquisition is in progress at this first preservation checkpoint. Production admission remains unchanged and rejected; no candidate apply occurred.


## Completed current-source cross-check

Corrected the initial helper cache API typo (`read` -> `get`) before commit 038fbb86. The initial acquisition retained all successful response bodies; its official parse failed without publishing. The corrected rerun exited 0, acquired only missing responses, and built both full-pool review manifests: official 8,124 observations / 4,614 cards; secondary 4,660 observations / cards.

Repository reporter command (exit 2, expected blocked):
```
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=code python -m tools.card_data.condition_review_report --review-tree ../bt27-canonical-acquisition/review-tree --output ../bt27-canonical-acquisition/condition-review.json
```
Current vs committed October 29 blocker: identical 84-card membership; identical literal candidate multisets including blanks; 277 observations, 17 blanks on 13 cards in each. All 84 observation-ID sets differ. Aggregate source revision is in `canonical-comparison-summary.json`. No selections were created. Historical selections cannot be reused without validating current manifest/candidate bindings.

Rendered evidence: BT8-012 base face says “Digivolve: 2 from [Veemon]”; P3 face says “Digivolve [Veemon]: Cost 2”. Both name Flamedramon and print BT8-012. BT11-012_P1 image instead prints **BT12-012 Agunimon**, with BurningGreymon condition. Its HTML record key `522902:BT11-012:BT11-012_P1` itself contains Agunimon and BurningGreymon (not Shoutmon X3); this is upstream identity contamination, not adjacent-row parser contamination. Normal `522013:BT11-012:BT11-012` is Shoutmon X3. Evidence hashes are recorded; raw source/image evidence remains in the private cache checkpoint.

Next smallest justified repair: use the repository's reviewed correction/adjudication contracts to exclude or correct the mistagged promotional observation's gameplay identity, preserving immutable raw evidence. Bind any BT8-012 wording selection to this current manifest and exact candidate map. Do not treat either conclusion as approval for the other 82 cards or publish conflicting old semantics.

Native gate command from `code/` (exit 0, 2 tests passed):
```
RUST_MIN_STACK=67108864 sh ../../bt27-lane-a-runtime/cargo-run test -p digimon-engine --test bt27_production_gate -- --test-threads=1
```
This confirms genuine population validation still rejects BT27-044, BT27-056 and BT27-099 for missing canonical registration. It does **not** prove admission. No new behavior changes, runtime smoke, lowering regeneration, full engine suite or training were run in this cross-check. Initial shell invocation used an incorrect relative log path and exited 1 before Cargo; the corrected command above ran successfully. `git diff --quiet 349816e5 -- data/card_registry.json data/cards.json` exited 0: all 4,483 old IDs and card data remain unchanged. Historical pools and production artifacts were not edited.

Complete candidate reproduction (exit 1):
```
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=code python -m tools.card_data.complete_candidate --repo-root . --review-tree ../bt27-canonical-acquisition/review-tree --rebuild-source-plan --output-dir ../bt27-canonical-acquisition/candidate
```
Fails in `complete_candidate.py:896` -> `compatibility_conditions.py:118` with `SourceSchemaError: official condition projection preflight failed`. This independently confirms the 84-conflict reporter is an actual canonical assembly blocker. No production registry was supplied; after these conflicts are legitimately resolved, downstream production artifact validation still remains required. No apply was attempted.
