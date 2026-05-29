## 1. Acceptance Tests and Baseline

- [x] 1.1 Record the resolved Xros Heart and XrosHeart pool baseline, including current Rust YAML coverage and the high-frequency missing cards this change targets.
- [x] 1.2 Add failing Rust behavioral tests for `BT19-008` and `BT19-057` effect digivolving from cards under Tamers.
- [x] 1.3 Add failing Rust behavioral tests for `BT19-014`, `AD1-006`, `AD1-013`, `BT19-026`, and `BT21-030` stack-derived selectors, formulas, and DP comparisons.
- [x] 1.4 Add failing Rust behavioral tests for `BT19-038`, `BT19-051`, `BT19-035`, and `BT20-037` temporary lockout and source-stack interaction shapes.
- [x] 1.5 Add DSL compile/lowering tests for source-zone digivolve, stack-derived metrics, temporary lockouts, and explicit unsupported-field errors.

## 2. Source-Zone Effect Digivolve

- [x] 2.1 Add or reuse source-zone selection bindings that preserve the selected card and origin Tamer across pending selection resolution.
- [x] 2.2 Wire selected under-Tamer cards into effect-initiated digivolve legality checks without moving the card before commitment.
- [x] 2.3 Commit source-zone card removal and source attachment only after the digivolution succeeds.
- [x] 2.4 Verify normal when-digivolving timing and inherited source order after source-zone effect digivolve.
- [x] 2.5 Add no-target, declined optional, illegal-path, and multiple-Tamer regression tests.

## 3. Stack-Derived Effect Metrics

- [x] 3.1 Add no-source and fewest-source selector predicates that preserve tied legal candidates.
- [x] 3.2 Add source-color counting formulas for live source stacks.
- [x] 3.3 Add current-DP comparison predicates that compare selected targets against the acting Digimon after active modifiers.
- [x] 3.4 Wire stack-derived metrics into target selection, DP modification, deletion, return-to-deck, and bottom-deck payoff effects used by the fixtures.
- [x] 3.5 Add regression tests for empty stacks, ties, changed DP before resolution, and multi-color source cards.

## 4. Temporary Effect Lockouts

- [x] 4.1 Add expiring modifier support for suppressing specific timing-effect families on affected Digimon or Tamers.
- [x] 4.2 Gate When Digivolving and On Play trigger collection/execution through active lockout modifiers.
- [x] 4.3 Add expiring cannot-unsuspend modifiers for Digimon and Tamers.
- [x] 4.4 Verify "until end of opponent's turn" expiry across the locked turn and the following turn.
- [x] 4.5 Add negative tests proving unrelated timing families and unaffected permanents still work.

## 5. DSL Schema and Lowering

- [x] 5.1 Add DSL vocabulary for selecting a digivolution card from cards under own Tamers and binding it for effect-initiated digivolve.
- [x] 5.2 Add DSL predicates/formulas for no-source targets, fewest-source targets, source-color counts, and current-DP comparisons.
- [x] 5.3 Add DSL vocabulary for temporary timing-effect lockouts and cannot-unsuspend modifiers with explicit expiry.
- [x] 5.4 Reject unsupported source zones, stack metrics, timing families, and expiry forms with explicit compile errors.
- [x] 5.5 Update DSL examples or schema docs where the new vocabulary needs authoring guidance.

## 6. Production Xros Heart Fixtures

- [x] 6.1 Author or promote production YAML for `BT19-008` and `BT19-057` using source-zone effect digivolve.
- [x] 6.2 Author or promote production YAML for `BT19-014`, `AD1-006`, `AD1-013`, `BT19-026`, and `BT21-030` using stack-derived metrics.
- [x] 6.3 Author or promote production YAML for `BT19-038`, `BT19-051`, `BT19-035`, and `BT20-037` using temporary lockouts and existing under-Tamer/source-flow primitives.
- [x] 6.4 Author or promote production YAML for `BT19-079` if existing Tamer material-routing primitives cover the card without new substrate.
- [x] 6.5 Re-run the resolver-backed Xros Heart coverage check and list any remaining missing cards as card-authoring work rather than substrate gaps.

## 7. Verification and Documentation

- [x] 7.1 Run focused Rust behavioral tests for all acceptance fixtures in this change.
- [x] 7.2 Run DSL parser/lowering tests for the new vocabulary and compile-error paths.
- [x] 7.3 Run targeted engine suites covering effect digivolve, source selectors, modifiers, and trigger collection.
- [x] 7.4 Run the broader relevant Rust suites or document unrelated pre-existing failures.
- [x] 7.5 Update `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/dsl-vocab-gaps.md` for closed and remaining reusable gaps.
- [x] 7.6 Update Xros Heart QA readiness notes to state whether the archetype is ready for full card authoring after this substrate lands.

Verification note, 2026-05-24:

- Focused acceptance filters passed for `bt19_008`, `bt19_057`, `bt19_014`,
  `ad1_006`, `ad1_013`, `bt19_026`, `bt21_030`, `bt19_038`, `bt19_051`,
  `bt19_035`, `bt19_079`, and `bt20_037`.
- Focused DSL filters passed for `source_stack_count`,
  `validate_source_stack_count_unknown_target_fails`, and
  `validate_unknown_timing_lockout_modifier_fails`.
- Targeted engine suites passed: `effect_context` (126/126),
  `timing_dispatch` (51/51), and `modifier_disable_effect` (2/2).
- Broader `cargo test --manifest-path code\digimon-engine\Cargo.toml --test dsl
  -- --nocapture` ran 716 tests with 713 passing and 3 unrelated failures in
  `group6_auras` granted-triggered-effect tests:
  `granted_body_installing_selection_parks_via_pending_selection`,
  `granted_triggered_effect_on_deletion_fires_when_carrier_deleted`, and
  `granted_triggered_effect_carrier_attribution_distinguishes_carrier_from_source`.
