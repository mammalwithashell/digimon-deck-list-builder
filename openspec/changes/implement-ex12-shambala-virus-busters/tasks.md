# Tasks: Implement EX12 Shambala + Virus Busters slices

## 1. Gap assessment (54 cards)

- [ ] 1.1 Re-run the 8-batch assessment workflow (`Workflow({scriptPath: workflows/scripts/ex12-shambala-vb-assess-wf_6f7700f2-6c5.js, resumeFromRunId: "wf_6f7700f2-6c5"})`) once usage credits allow; collect per-card SUPPORTED/PARTIAL/BLOCKED verdicts + consolidated gap entries
- [ ] 1.2 Merge consolidated gap entries into `docs/RUST_ENGINE_GAPS.md` / `qa/dsl-vocab-gaps.md` (dedup vs existing entries; keep G-KEYWORD-GUARD / G-KEYWORD-ENGAGE ids); write the audit-index table into `qa/archetype-qa/` scoping doc
- [ ] 1.3 Resolve the Engage open questions from scans + wiki rulings (target legality; played-this-turn) and the [Kotenken] token stats (EX12-034 scan); record answers in the scoping doc
- [ ] 1.4 Extend the `dsl-card-scripting-vocabulary` delta spec with any assessment-surfaced vocabulary (per spec requirement) before closure begins

## 2. Keyword substrate: Guard + Engage (TDD)

- [ ] 2.1 `Keyword::Guard` + printed parse (`＜Guard＞`/`<Guard>`) with parse tests; validator allowlist entry (`KNOWN_KEYWORD_KEYS`) per the native-printed-keyword pattern
- [ ] 2.2 Guard behavior: auto-emitted protect-others leave replacement (delete_self cost, prevent outcome, opponent-effect cause scope), clone-safe; engine tests per the keyword-guard spec scenarios (accept/decline/own-effect negative/carrier-not-protected/clone-safety)
- [ ] 2.3 `Keyword::Engage` + printed parse + validator entry; end-of-your-turn optional attack window per confirmed rulings (or literal reminder text with the open point pinned); engine tests per the keyword-engage spec scenarios
- [ ] 2.4 Aura-granted Guard/Engage parity test (grant via aura behaves like printed — EX12-072 shape)
- [ ] 2.5 Token registry: [Paishu] (Yellow/6000/Blocker+Guard) + [Kotenken] (per 1.3) with token-carried keyword tests
- [ ] 2.6 Schema regen + vocab-doc drift gate; mark G-KEYWORD-GUARD / G-KEYWORD-ENGAGE RESOLVED in trackers; commit the keyword round

## 3. Gap-closure rounds (assessment findings)

- [ ] 3.1 Group confirmed gaps by subsystem and dispatch TDD closure agents (clone-safe, DSL-level tests, tracker RESOLVED marks) — one commit per round, scoped suites green
- [ ] 3.2 Full `--test dsl` + affected-binary sweep after the final round; adjudicate any NOT-A-GAP findings in the trackers

## 4. Implementation waves — Shambala (33 cards)

- [ ] 4.1 Wave S1: eggs + Lv3s (EX12-002, -004, -006, -009, -020, -022, -039, -061) — implement→review→merge→verdicts→commit
- [ ] 4.2 Wave S2: Lv4–5 SW engine (EX12-012, -015, -025, -029, -043, -045, -056) — same gate
- [ ] 4.3 Wave S3: Lv4–5 TB engine (EX12-011, -026, -031, -046, -062, -063) — same gate
- [ ] 4.4 Wave S4: Tentei Hachibushu Lv6s + Lv7 + Options (EX12-019, -034, -036, -047, -048, -057, -065, -070, -071, -074, -075, -076) — same gate; Guard/Engage consumers verified against the keyword machinery
- [ ] 4.5 Fix round for any review rejections; scoped suites green; verdicts recorded for all 33

## 5. Implementation waves — Virus Busters (21 cards)

- [ ] 5.1 Wave V1: Gammamon line + DUAL (EX12-001, -005, -007, -013, -014, -018, -077) — Siriusmon on the dual-YAML shape with top-or-bottom placement + rider per spec
- [ ] 5.2 Wave V2: Agu/Gabu/ME lines (EX12-010, -016, -017, -021, -024, -032, -035, -037) — same gate
- [ ] 5.3 Wave V3: Salamon line + support (EX12-040, -042, -044, -066, -069, -073) — same gate
- [ ] 5.4 Fix round for review rejections; scoped suites green; verdicts recorded for all 21

## 6. Seal + capstone

- [ ] 6.1 Full `cards_behavioral` + `dsl` seal suites over the completed slices; full-pool dsl-lint exit 0
- [ ] 6.2 Shambala interaction capstone: archetype model doc + interaction tests + four static archetype tests; verdict in `qa/qa-reports/archetype_interactions.json`
- [ ] 6.3 Virus Busters interaction capstone: same shape (partner-line combos incl. Gammamon→Siriusmon and Omnimon assembly)
- [ ] 6.4 Reconcile any scan-vs-JSON text divergences found during review into `data/card_overrides.json`; final commit; tag a DCGO re-audit follow-up for when the community implementation ships EX12
