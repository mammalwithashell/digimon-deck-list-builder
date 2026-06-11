## 0. Preconditions

- [x] 0.1 Confirmed: the interim patch (`combat/mod.rs`, `game/queries.rs`, `wargreymon_security_attack.rs`) is uncommitted (working-tree modifications only; not in HEAD). On `main`.
- [x] 0.2 Baseline: `cards_behavioral` = **4521 passed, 0 failed, 66 ignored** (full run this session, interim patch present).

## 1. Leading keyword-line tokenizer

- [x] 1.1 Rewrote `parse_printed_keywords` (`card_data.rs`): the per-field scan is now a leading-keyword-line consumer (`strip_leading_keyword_headers` → loop consuming `＜kw＞` + optional `consume_balanced_parens` reminder → break at first non-`＜` content). Classification block (prefix table + parametric) unchanged — only WHERE it scans changed.
- [x] 1.2 Header set pinned in `strip_leading_keyword_headers`: `Inherited Effect`, `Security Effect`, `Rule Effect`. (Step 2 diff will surface any missed header.)
- [x] 1.3 8 new unit tests in `card_data.rs` (all green): innate-with-reminder→`[Blocker]`; granted-after-timing→`[]` (WarGreymon); target-filter→`[]` (SkullGreymon); conditional-grant→`[]` (Flarerizamon); reminder inner `[trait]` doesn't end line + second keyword still parsed; leading keyword then timed effect→only leading; inherited header skipped (with-kw vs timed-grant). Existing keyword tests still pass.

## 2. Pool-wide keyword diff + audit

- [x] 2.1 Built `code/tools/keyword_parse_diff.py` (OLD all-token vs NEW context-classifier over `cards.json`) → `keyword-diff.md`. `new_gained=0` (parser never invents a keyword).
- [x] 2.2 Partitioned by DSL `*.yaml` (NOT `*.json` — those are metadata; the partition bug initially inflated the count). Result: **167 implemented (yaml) losers**, 941 unimplemented (metadata-only, no DSL behavior). Built `code/tools/keyword_audit.py` (role + modeled classifier) → `keyword-audit.md`. Both artifacts committed for review.
- [x] 2.3 **Gate fired** (design D3): the raw count (1367/1094) triggered a pause; root-caused to the partition bug + the leading-line mechanism flaw. After both corrections the regression-risk surface is **20 cards**, and manual review found **all 20 false-positive** (Alliance-reminder Security A., Token stat-blocks, condition/action references, already-modeled grants).

## 3. Model the gaps (implemented regressors)

- [x] 3.1 No gaps to model: the audit (2.3) found **zero** implemented cards that genuinely relied on the phantom. All 20 risk candidates were false-positives (lost token was a reminder description / Token stat-block / condition reference / already-modeled grant). Verified by reading each card's YAML + effect text.
- [x] 3.2 N/A — no grant changed from phantom to conditional-effect (no real gaps).
- [x] 3.3 Unimplemented losers (941) are metadata-only cards with no DSL behavior, so the phantom keyword was already inert for them — nothing regresses. Recorded the verdict in design D3 rather than logging 941 inert entries.

## 4. Supersede the interim WarGreymon patch

- [x] 4.1 Removed both helpers from `game/queries.rs` and the `raw_security_strike` subtraction from `combat/mod.rs` (reverted to `let sa_keyword = self.security_attack_keyword_bonus(target)`). Compiles clean.
- [x] 4.2 WarGreymon real-data tests retained and PASS via the parser fix (27/27 in the wargreymon suite, incl. `..._checks_three_on_your_turn` →3 and `..._checks_one_off_turn` →1).
- [x] 4.3 Updated the `docs/RUST_ENGINE_GAPS.md` WarGreymon entry: now RESOLVED at the parser root cause via `fix-innate-keyword-overextraction`, superseding the combat-site patch.

## 5. Verification

- [x] 5.1 Final `cards_behavioral` (parser fix alone, interim patch removed): **4521 passed, 0 failed, 66 ignored** = baseline. (Also confirmed 4521/0 with patch+fix before removal.)
- [x] 5.2 `card_data` tokenizer unit tests (19, incl. 13 keyword) + WarGreymon real-data tests green.
- [x] 5.3 No signature changes → PyO3/desktop unaffected at the API level; the engine change improves the value the desktop reads (`effective_security_strike` → 3). Live desktop spot-check left optional (the engine real-data test proves it).

## 6. Docs

- [x] 6.1 Documented the innate-vs-granted contract in the `parse_printed_keywords` doc comment + the per-field loop comment (`card_data.rs`), and added a "Keyword innate-vs-granted parsing — durable contract" section to `docs/RUST_ENGINE_GAPS.md`.
- [x] 6.2 Recorded the deferred `spec.keywords`-authoritative follow-up in `docs/RUST_ENGINE_GAPS.md` (noted as not-needed-now: the context-classifier had 0 genuine regressions in the pool audit).
