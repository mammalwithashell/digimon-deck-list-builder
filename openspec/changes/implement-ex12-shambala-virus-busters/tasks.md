# Tasks: Implement EX12 Shambala + Virus Busters slices

## 1. Gap assessment (54 cards)

- [x] 1.1 Manual fallback audit completed because workflow run `wf_6f7700f2-6c5` returned no usable verdicts; collected per-card SUPPORTED/PARTIAL/BLOCKED verdicts + consolidated gap entries
- [x] 1.2 Merge consolidated gap entries into `docs/RUST_ENGINE_GAPS.md` / `qa/dsl-vocab-gaps.md` (dedup vs existing entries; keep G-KEYWORD-GUARD / G-KEYWORD-ENGAGE ids); write the audit-index table into `qa/archetype-qa/` scoping doc
- [x] 1.3 Resolve the Engage open questions from official rules/card-list sources (target legality; played-this-turn) and the [Kotenken] token stats (EX12-034); record answers in the scoping doc
- [x] 1.4 Extend the `dsl-card-scripting-vocabulary` delta spec with any assessment-surfaced vocabulary (per spec requirement) before closure begins

## 2. Keyword substrate: Guard + Engage (TDD)

- [x] 2.1 `Keyword::Guard` + printed parse (`＜Guard＞`/`<Guard>`) with parse tests; validator allowlist entry (`KNOWN_KEYWORD_KEYS`) per the native-printed-keyword pattern
- [x] 2.2 Guard behavior: auto-emitted protect-others leave replacement (delete_self cost, prevent outcome, opponent-effect cause scope), clone-safe; engine tests per the keyword-guard spec scenarios (accept/decline/own-effect negative/carrier-not-protected/clone-safety)
- [x] 2.3 `Keyword::Engage` + printed parse + validator entry; end-of-your-turn optional attack window per confirmed rulings (or literal reminder text with the open point pinned); engine tests per the keyword-engage spec scenarios
- [x] 2.4 Aura-granted Guard/Engage parity test (grant via aura behaves like printed — EX12-072 shape)
- [x] 2.5 Token registry: [Paishu] (Yellow/6000/Blocker+Guard) + [Kotenken] (per 1.3) with token-carried keyword tests
- [ ] 2.6 Schema regen + vocab-doc drift gate; mark G-KEYWORD-GUARD / G-KEYWORD-ENGAGE RESOLVED in trackers; commit the keyword round

## 3. Gap-closure rounds (assessment findings)

- [ ] 3.1 Group confirmed gaps by subsystem and dispatch TDD closure agents (clone-safe, DSL-level tests, tracker RESOLVED marks) — one commit per round, scoped suites green
- [ ] 3.2 Full `--test dsl` + affected-binary sweep after the final round; adjudicate any NOT-A-GAP findings in the trackers

## 4. Implementation waves — Shambala (33 cards)

Local non-commit status (2026-07-08): all 33 Shambala YAML specs and
DebugRunner tests are present locally; focused EX12-047/EX12-074 tests, the
`returned_card_color_count` DSL regression, and the full `ex12_0` behavioral
filter are green. Wave boxes remain unchecked because their stated gate includes
review/merge/verdict/commit work not performed in this local-only pass.

- [ ] 4.1 Wave S1: eggs + Lv3s (EX12-002, -004, -006, -009, -020, -022, -039, -061) — implement→review→merge→verdicts→commit
- [ ] 4.2 Wave S2: Lv4–5 SW engine (EX12-012, -015, -025, -029, -043, -045, -056) — same gate
- [ ] 4.3 Wave S3: Lv4–5 TB engine (EX12-011, -026, -031, -046, -062, -063) — same gate
- [ ] 4.4 Wave S4: Tentei Hachibushu Lv6s + Lv7 + Options (EX12-019, -034, -036, -047, -048, -057, -065, -070, -071, -074, -075, -076) — same gate; Guard/Engage consumers verified against the keyword machinery
- [ ] 4.5 Fix round for any review rejections; scoped suites green; verdicts recorded for all 33

## 5. Implementation waves — Virus Busters (21 cards)

Local non-commit status (2026-07-08): EX12-001, EX12-005, EX12-007,
EX12-010, EX12-013, EX12-014, EX12-016, EX12-017, EX12-018, EX12-021,
EX12-024, EX12-032, EX12-035, EX12-037, EX12-040, EX12-042, EX12-044,
EX12-066, EX12-069, EX12-073, and EX12-077 YAML specs and DebugRunner tests
are present locally. The latest full `ex12_0` behavioral filter is green
(`130 passed; 0 failed`), and the full `cargo test -p digimon-engine --test
dsl -- --test-threads=1` seal is green after local Windows stack-guard fixes
plus the EX12-073 printed-type reconciliation (`NSp/DS/NSo/WG/ME/VB`) into
production card data. The V2/V3 follow-up added EX12-024, EX12-032,
EX12-035, EX12-037, EX12-040, EX12-042, EX12-044, EX12-066, EX12-069, and
EX12-073 coverage, including EX12-032's same-level source-pair gate,
EX12-035's printed Assembly route, EX12-037's formula-count repeated modal
choices, EX12-018's no-security-effect correction, and EX12-073's trait-parity
fix. Wave boxes remain unchecked because the stated gate includes formal
review, verdict recording, and commit work not performed in this local-only
pass.

- [ ] 5.1 Wave V1: Gammamon line + DUAL (EX12-001, -005, -007, -013, -014, -018, -077) — Siriusmon on the dual-YAML shape with top-or-bottom placement + rider per spec
- [ ] 5.2 Wave V2: Agu/Gabu/ME lines (EX12-010, -016, -017, -021, -024, -032, -035, -037) — same gate
- [ ] 5.3 Wave V3: Salamon line + support (EX12-040, -042, -044, -066, -069, -073) — same gate
- [ ] 5.4 Fix round for review rejections; scoped suites green; verdicts recorded for all 21

## 6. Seal + capstone

- [ ] 6.1 Full `cards_behavioral` + `dsl` seal suites over the completed slices; full-pool dsl-lint exit 0
- [ ] 6.2 Shambala interaction capstone: archetype model doc + interaction tests + four static archetype tests; verdict in `qa/qa-reports/archetype_interactions.json`
- [ ] 6.3 Virus Busters interaction capstone: same shape (partner-line combos incl. Gammamon→Siriusmon and Omnimon assembly)
- [ ] 6.4 Reconcile any scan-vs-JSON text divergences found during review into `data/card_overrides.json`; final commit; tag a DCGO re-audit follow-up for when the community implementation ships EX12
