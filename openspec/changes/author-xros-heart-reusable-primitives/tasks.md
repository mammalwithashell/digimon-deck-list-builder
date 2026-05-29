## 1. Acceptance Tests and Baseline

- [x] 1.1 Add failing Rust behavioral tests for `BT21-083` placing cards under itself and granting a just-played or just-digivolved matching Digimon an optional attack window.
- [x] 1.2 Add failing Rust behavioral tests for `BT11-095` placing cards under itself and granting under-Tamer cards as DigiXros materials for one pending play.
- [x] 1.3 Add failing Rust behavioral tests for `P-224` placing cards from hand or trash under itself and playing a level 5 or higher Xros Heart Digimon from under Tamers with play-cost reduction.
- [x] 1.4 Add failing Rust behavioral tests for `BT19-090` playing a low-DP Xros Heart Digimon from under a Tamer and resolving the unsuspend-then-attack option mode.
- [x] 1.5 Add failing Rust behavioral tests for `BT21-092` moving sources under a Tamer, counting moved cards, and playing an Xros Heart Digimon from hand with cost reduced by the moved count.
- [x] 1.6 Add failing Rust behavioral tests for `BT10-111` turn-scoped DigiXros wildcard requirement substitution.
- [x] 1.7 Add failing Rust behavioral tests for `BT21-027` leave-battle source rescue using trait filters instead of DigiXros recipe filters.
- [x] 1.8 Add failing Rust behavioral tests for `BT19-061` hand-or-trash stash placement on deletion and treated-as behavior for DigiXros.
- [x] 1.9 Record the current card YAML/example/gap state that these tests are expected to close or narrow.

## 2. Under-Tamer Card Flow

- [x] 2.1 Add reusable selectors for cards under one own Tamer and under any own Tamer, preserving origin Tamer and source identity.
- [x] 2.2 Add helpers for placing selected cards from hand under a chosen or source Tamer.
- [x] 2.3 Add helpers for placing selected cards from trash under a chosen or source Tamer.
- [x] 2.4 Add union-zone selection support for hand-or-trash into Tamer stash effects.
- [x] 2.5 Add play-from-under-Tamer support for free play while preserving normal play timing and on-play triggers.
- [x] 2.6 Add cost-reduced play-from-under-Tamer support with rollback or delayed consumption on payment failure.
- [x] 2.7 Add tests for no-target, declined optional, multiple-Tamer, and opponent-Tamer exclusion paths.

## 3. Source-Stack Payoffs and Leave-Battle Rescue

- [x] 3.1 Generalize snapshot-backed source rescue so authored filters can select from pre-removal sources beyond Material Save recipes.
- [x] 3.2 Add move-all-source-cards-under-Tamer support for effects that empty a chosen permanent's source stack.
- [x] 3.3 Add up-to-N filtered source movement under a Tamer with count-capped pending selection.
- [x] 3.4 Expose the moved-source count to later effect steps for cost formulas.
- [x] 3.5 Add opponent stack-trashing support for top-N stacked cards on a selected opponent Digimon.
- [x] 3.6 Add no-source target filtering for return-to-deck and similar payoff effects.
- [x] 3.7 Verify source movement ordering and handle/index stability when source cards move from a permanent that leaves battle.

## 4. DigiXros Transaction Follow-Ups

- [x] 4.1 Add a scoped DigiXros wildcard substitution model that can replace one unfilled recipe requirement without changing global card identity.
- [x] 4.2 Wire turn-scoped wildcard modifiers into later DigiXros transactions and expire them at the printed duration.
- [x] 4.3 Wire current-transaction modifiers into the pending DigiXros transaction without leaking to later plays.
- [x] 4.4 Ensure wildcard material choices appear in material-selection masks and are masked after the substitution is consumed.
- [x] 4.5 Add regression tests proving wildcard substitutions do not satisfy non-DigiXros name or trait predicates.

## 5. Event-Driven Attack Windows

- [x] 5.1 Add a temporary attack-window primitive for a just-played or just-digivolved permanent.
- [x] 5.2 Add optional may-attack handling that uses PASS or equivalent pending-selection decline behavior.
- [x] 5.3 Add effect-driven attack setup support for selecting and unsuspending named permanents before the attack prompt.
- [x] 5.4 Route effect-driven attacks through normal attack declaration, blocker, collision, and attack-resolution hooks.
- [x] 5.5 Add no-legal-attacker and no-legal-target regression tests.

## 6. DSL Schema and Lowering

- [x] 6.1 Add DSL steps for selecting cards under Tamers and binding their origin for later movement or play.
- [x] 6.2 Add DSL steps for placing selected cards from hand, trash, or hand-or-trash under Tamers.
- [x] 6.3 Add DSL steps for playing selected cards from under Tamers for free or with play-cost reductions.
- [x] 6.4 Add DSL steps for moving all or filtered source cards under Tamers and binding moved counts.
- [x] 6.5 Add DSL formulas that can consume moved-source counts for later play-cost reductions.
- [x] 6.6 Add DSL steps for top-N opponent stack trashing and no-source target filters.
- [x] 6.7 Add DSL vocabulary for scoped DigiXros wildcard requirement substitution.
- [x] 6.8 Add DSL vocabulary for optional immediate attack windows and effect-driven attack prompts.
- [x] 6.9 Reject unsupported under-Tamer, source-stack, wildcard, and effect-attack fields with explicit compile errors.

## 7. Production Xros Heart Authoring

- [x] 7.1 Promote or author production YAML for `BT21-083` with under-Tamer stash and optional attack-window behavior.
- [x] 7.2 Promote or author production YAML for `BT11-095` with under-Tamer stash and DigiXros material-access behavior.
- [x] 7.3 Promote or author production YAML for `P-224` with hand/trash stash and cost-reduced play-from-under-Tamer behavior.
- [x] 7.4 Promote or author production YAML for `BT19-090` with both option modes and security play behavior where supported.
- [x] 7.5 Promote or author production YAML for `BT21-092` with source movement, moved-count cost reduction, and security play behavior where supported.
- [x] 7.6 Promote or author production YAML for `BT10-111` without `raw_rust` wildcard placeholders.
- [x] 7.7 Promote or author production YAML for `BT21-027` with trait-filtered source rescue.
- [x] 7.8 Promote or author production YAML for `BT19-061` with treated-as DigiXros identity, search/trash split, and deletion stash behavior.
- [x] 7.9 Remove or update example comments that no longer describe true gaps after the primitive fixtures land.

## 8. Verification and Documentation

- [x] 8.1 Run focused Rust behavioral tests for all acceptance cards in this change.
- [x] 8.2 Run DSL parser and lowering tests for the new vocabulary.
- [x] 8.3 Run targeted engine tests for under-Tamer selectors, source rescue, DigiXros wildcard substitution, and effect-driven attacks.
- [x] 8.4 Run the broader relevant Rust suites or document unrelated pre-existing failures. (`effect_context` passed; full `dsl` currently has unrelated `group6_auras` failures.)
- [x] 8.5 Update `docs/RUST_ENGINE_GAPS.md` for closed under-Tamer, source rescue, wildcard, and effect-driven attack primitives.
- [x] 8.6 Update `qa/archetype-qa/engine-gaps.md` and `qa/dsl-vocab-gaps.md` with closure notes and remaining blockers.
- [x] 8.7 Update the Xros Heart archetype readiness report with the new primitive-first authoring verdict.
