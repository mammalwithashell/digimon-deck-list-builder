# Archetype DSL Implementation: BT25 link-finish-appmon slice
Date: 2026-06-07
Total cards in slice: 4
Processed this run: 4
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 3  (BT25-004, BT25-045, BT25-036)
- PARTIAL: 1      (BT25-089 — carry-forward, residual gaps still open)
- BLOCKED: 0

This slice was a **re-adjudication run**: all four cards carried prior verdicts
(BT25-089 PARTIAL; BT25-004/045/036 BLOCKED) that were **stale** — the engine
primitives they were blocked on have since landed:
- facet #10 host-filtered `WhenWouldLink` cost reducer (Gap 5) → unblocks
  BT25-004, BT25-045.
- App Fusion alt-play (`AltPathKind::AppFusion` resolves end-to-end) → unblocks
  BT25-036.

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Tests | Notes |
|---------|------|------|---------|-------|-------|
| BT25-004 | Tapmon | IMPLEMENT | IMPLEMENTED | 5 | Inherited [Your Turn][OPT] `when_would_link_to_this` reducer (Social/Tool/Game → −1). Sole clause. |
| BT25-045 | Onmon | IMPLEMENT | IMPLEMENTED | 6 | alt-digivolve Lv.2 Appmon c0 + link_condition Appmon c1 + face-up reducer + [When Linking] suspend opp. |
| BT25-036 | Craftmon | IMPLEMENT | IMPLEMENTED | 9 | App Fusion + alt-digivolve Lv.2 [Stnd.] c2 + link_condition c2 + [Security] play-self + OP/WD add-security+Recovery+1 + [When Linking] trash-Appmon→Draw2. |
| BT25-089 | Kazuki & Itsuki | (carry) | PARTIAL | 0 | [Main] link + memory ramp + inherited [Security] shipped; effect-initiated App Fuse + Tamer-anchored link-from-own-Digimon-sources still gapped. |

## Engine-Gap Resolutions Confirmed (production-faithful)
- **facet #10 — predicated `WhenWouldLink` reducer (Gap 5)** — BT25-004 (inherited)
  and BT25-045 (face-up) are the first production users. DSL surface:
  `when: when_would_link_to_this` + `active_when: { would_link_card_trait_any_of }`
  + `reduce_link_cost: { amount }`, optional + once_per_turn → lowers to
  `EffectContext::reduce_pending_link_cost`.
- **App Fusion alt-play** — BT25-036 is the first production card on
  `alt_paths: [{ kind: app_fusion, materials, cost }]`. Resolves through the
  digivolve route funnel; stacks the App-Fusion card on top + drains the host's
  linked cards under it as sources.

## Still-Open Gaps (BT25-089)
- **Effect-initiated App Fuse** (a field Digimon fuses INTO a chosen hand card) —
  no DSL process-step / engine primitive (distinct from the alt-path App Fusion).
- **Tamer-anchored link-from-own-Digimon-sources** — `link_card_to_self` with
  `from: [digivolution_sources]` is anchored to the effect's own permanent; a
  Tamer has no under-sources, so scanning all of the controller's Digimon for
  digivolution-card link sources is unsupported.

## Test Evidence
`cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- bt25_004 bt25_045 bt25_036 app_fusion`
→ 24 passed; 0 failed.

Full `cards_behavioral` suite: 4124 passed / 7 failed — the 7 failures are the
documented pre-existing DP-bonus failures (`bt21_072`, `ex7_030`, `p_134`,
`p_197`), unrelated to this slice. No regressions introduced.
