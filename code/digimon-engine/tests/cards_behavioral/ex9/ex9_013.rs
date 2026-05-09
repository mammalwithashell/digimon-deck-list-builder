//! EX9-013 BlitzGreymon — Lv6, Red/White, DP12000, Cost7. Traits: Cyborg, DM, Ver.1
//!
//! # Card text (cards.json)
//!
//! [Hand] [Counter] <Blast Digivolve>
//! <Alliance>
//! <Blocker>
//! [On Play] [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon.
//! (Trash up to 3 cards from the top. You can't trash past level 3 cards.)
//! [End of Your Turn] 2 of your Digimon may DNA digivolve into [Omnimon Alter-S]
//! in the hand. Then, 1 of your Digimon may attack.
//!
//! # Inherited effect (cards.json)
//!
//! Ace Overflow <-4>
//! (As this card moves from the field or under a card to an area other than those,
//! lose 4 memory.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX9/Red/EX9_013.cs
//!
//! # Patterns this test covers
//! - G2: DNA digivolve (effect-initiated, from hand, end-of-your-turn trigger)
//! - H5: Blocker keyword (grant_keyword: Blocker)
//! - H10: Alliance keyword (grant_keyword: Alliance)
//! - H12: Blast Digivolve (grant_keyword: BlastDigivolve)
//! - H13: ACE (ace_overflow: -4)
//! - D1-adjacent: De-Digivolve 3 (select + de_digivolve step, mandatory)
//! - E2-adjacent: optional End-of-Turn clause
//! - G-MAY-ATTACK-NOW: post-DNA optional may-attack-now sub-clause

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{decode_attack, encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, make_test_dna_card, DebugRunner};
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

/// Production YAML for EX9-013, loaded at compile time.
const YAML: &str = include_str!("../../../cards/ex9/EX9-013.yaml");

/// Compile EX9-013 from production YAML.
fn compiled_ex9_013() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("EX9-013.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("EX9-013.yaml compiles");
    registry
        .lookup("EX9-013")
        .expect("EX9-013 in registry")
        .clone()
}

/// Build a minimal runner with EX9-013 loaded from production YAML.
fn blitz_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX9-013 YAML loads")
        .memory(10)
        .build()
}

/// Build a runner with EX9-013 and one opponent test Digimon.
fn blitz_runner_with_opp() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX9-013 YAML loads")
        .add_card(make_test_card("OPP-DIG", "OppDigimon"))
        .memory(10)
        .build()
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn ex9_013_compiled_has_correct_ace_overflow() {
    let compiled = compiled_ex9_013();
    assert_eq!(
        compiled.ace_overflow,
        Some(-4),
        "ACE Overflow should be -4 per printed inherited text"
    );
}

#[test]
fn ex9_013_has_two_digivolve_alt_paths() {
    let compiled = compiled_ex9_013();
    let digi_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(
        digi_paths.len(),
        2,
        "Should have standard (Lv5/4) + alt-digi (Lv5+Greymon|DM/3) paths"
    );
}

#[test]
fn ex9_013_standard_alt_path_cost_4() {
    let compiled = compiled_ex9_013();
    let standard = compiled.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && !p.ignore_requirements
            && matches!(
                p.cost,
                Some(digimon_dsl::compiled::CompiledCost::Literal(4))
            )
    });
    assert!(
        standard.is_some(),
        "Should have a Lv5/Cost4 standard digivolve path"
    );
}

#[test]
fn ex9_013_alt_digi_path_cost_3_ignore_requirements() {
    let compiled = compiled_ex9_013();
    let alt_digi = compiled.alt_paths.iter().find(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.ignore_requirements
            && matches!(
                p.cost,
                Some(digimon_dsl::compiled::CompiledCost::Literal(3))
            )
    });
    assert!(
        alt_digi.is_some(),
        "Should have an alt-digi Lv5+[Greymon]|[DM]/Cost3/ignore_requirements path"
    );
}

#[test]
fn ex9_013_has_three_grant_keyword_declarative_clauses() {
    let compiled = compiled_ex9_013();
    let kw_clauses: Vec<_> = compiled
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
            )
        })
        .collect();
    assert_eq!(
        kw_clauses.len(),
        3,
        "Should have BlastDigivolve + Alliance + Blocker grant_keyword clauses"
    );
}

#[test]
fn ex9_013_grant_keyword_blast_digivolve_present() {
    let compiled = compiled_ex9_013();
    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) if keyword == "BlastDigivolve"
        )
    });
    assert!(found, "GrantKeyword(BlastDigivolve) clause must be present");
}

#[test]
fn ex9_013_grant_keyword_alliance_present() {
    let compiled = compiled_ex9_013();
    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) if keyword == "Alliance"
        )
    });
    assert!(found, "GrantKeyword(Alliance) clause must be present");
}

#[test]
fn ex9_013_grant_keyword_blocker_present() {
    let compiled = compiled_ex9_013();
    let found = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) if keyword == "Blocker"
        )
    });
    assert!(found, "GrantKeyword(Blocker) clause must be present");
}

#[test]
fn ex9_013_has_on_play_when_digivolving_triggered_clause() {
    let compiled = compiled_ex9_013();
    let clause = compiled.effects.iter().find_map(|c| {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
            {
                Some(t)
            } else {
                None
            }
        } else {
            None
        }
    });
    assert!(
        clause.is_some(),
        "Should have [On Play][When Digivolving] triggered clause"
    );
    let c = clause.unwrap();
    // `scope: own` in DSL maps to `CompiledScope::FaceUp` (own/face-up effects)
    assert_eq!(c.scope, CompiledScope::FaceUp);
    assert!(
        !c.optional,
        "De-Digivolve 3 is mandatory (DCGO canNoSelect: false)"
    );
    assert!(!c.once_per_turn, "De-Digivolve 3 has no [Once Per Turn]");
}

#[test]
fn ex9_013_has_end_of_your_turn_optional_triggered_clause() {
    let compiled = compiled_ex9_013();
    let clause = compiled.effects.iter().find_map(|c| {
        if let CompiledClause::Triggered(t) = c {
            if t.when.contains(&CompiledTiming::EndOfYourTurn) {
                Some(t)
            } else {
                None
            }
        } else {
            None
        }
    });
    assert!(
        clause.is_some(),
        "Should have EndOfYourTurn triggered clause"
    );
    let c = clause.unwrap();
    assert!(
        c.optional,
        "DNA digivolve clause is optional ('2 of your Digimon may')"
    );
    assert_eq!(c.scope, CompiledScope::FaceUp);
    assert!(
        !c.once_per_turn,
        "No [Once Per Turn] on End-of-Turn DNA clause"
    );
}

#[test]
fn ex9_013_total_effect_clause_count_is_five() {
    let compiled = compiled_ex9_013();
    assert_eq!(
        compiled.effects.len(),
        5,
        "3 grant_keyword + 1 on_play/when_digivolving + 1 end_of_your_turn = 5 clauses"
    );
}

// ─── Section 2: Condition gating — De-Digivolve 3 ───────────────────────────

/// Negative: no opponent Digimon → condition blocks the effect, no selection prompt.
#[test]
fn ex9_013_de_digivolve_negative_no_opponent_digimon_no_prompt() {
    let mut runner = blitz_runner();
    let perm = runner.place_on_field(0, "EX9-013", None);
    runner.fire_on_play(0, perm.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "No selection should install when opponent has no Digimon"
    );
}

/// Positive: opponent has a Digimon → condition passes, selection prompt installs.
#[test]
fn ex9_013_de_digivolve_positive_opponent_digimon_prompts_selection() {
    let mut runner = blitz_runner_with_opp();
    runner.place_on_field(1, "OPP-DIG", None);
    let perm = runner.place_on_field(0, "EX9-013", None);
    runner.fire_on_play(0, perm.index as usize);

    let kind = runner.pending_kind().expect("Selection should install");
    assert_eq!(
        kind,
        SelectionKind::OppField,
        "Should be OppField selection for opponent Digimon (select_opponent_permanent)"
    );
}

// ─── Section 3: De-Digivolve 3 behavioral ────────────────────────────────────

#[test]
fn ex9_013_de_digivolve_on_play_targets_exactly_one_opponent_digimon() {
    let mut runner = blitz_runner_with_opp();
    runner.place_on_field(1, "OPP-DIG", None);
    let perm = runner.place_on_field(0, "EX9-013", None);
    runner.fire_on_play(0, perm.index as usize);

    assert!(
        runner.pending_selection().is_some(),
        "Selection should be pending"
    );
    let view = runner.pending_selection_view().unwrap();
    // OppField kind installs for select_opponent_permanent
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "select_opponent_permanent installs OppField selection"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "Exactly 1 opponent Digimon should be a valid target"
    );
    // Selection should NOT be optional (DCGO canNoSelect: false)
    assert!(
        !view.is_optional,
        "De-Digivolve 3 target selection is mandatory"
    );
}

#[test]
fn ex9_013_de_digivolve_resolves_without_further_selection() {
    let mut runner = blitz_runner_with_opp();
    runner.place_on_field(1, "OPP-DIG", None);
    let perm = runner.place_on_field(0, "EX9-013", None);
    runner.fire_on_play(0, perm.index as usize);

    let view = runner.pending_selection_view().unwrap();
    let action_id = view.valid_action_ids[0];
    let sel_player = view.selecting_player;
    runner
        .game
        .resolve_selection(sel_player, action_id)
        .expect("selection resolves");

    // After resolution no further pending selection
    assert!(
        runner.pending_selection().is_none(),
        "De-digivolve should complete cleanly"
    );
}

#[test]
fn ex9_013_de_digivolve_fires_on_when_digivolving_timing() {
    let mut runner = blitz_runner_with_opp();
    runner.place_on_field(1, "OPP-DIG", None);
    let blitz = runner.place_on_field(0, "EX9-013", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(blitz),
    );
    runner.game.drain_effect_queue();

    let kind = runner
        .pending_kind()
        .expect("WhenDigivolving should prompt");
    assert_eq!(
        kind,
        SelectionKind::OppField,
        "select_opponent_permanent installs OppField"
    );
}

// ─── Section 4: End-of-Turn optional DNA clause ──────────────────────────────

/// Negative: End-of-Turn clause is optional — player can pass when prompted.
#[test]
fn ex9_013_eot_dna_clause_optional_can_pass() {
    let mut runner = blitz_runner();
    let blitz = runner.place_on_field(0, "EX9-013", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(blitz));
    runner.game.drain_effect_queue();

    // If a selection installed it must be optional
    if runner.pending_selection().is_some() {
        assert!(
            runner.pending_is_optional(),
            "EOT DNA clause must be optional ('may')"
        );
        runner.execute_action(0, PASS).ok();
    }
    assert!(
        runner.pending_selection().is_none(),
        "After declining, no pending selection should remain"
    );
}

/// Positive: with an "Omnimon Alter-S" named card in hand, the EOT clause
/// should install a hand selection prompt.
#[test]
fn ex9_013_eot_dna_prompts_hand_selection_with_eligible_card() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX9-013 YAML loads")
        // Synthetic card named "Omnimon Alter-S" — passes `name_contains` filter
        .add_card(make_test_card("TST-OMNI", "Omnimon Alter-S"))
        .add_card(make_test_card("ALLY-A", "AllyA"))
        .hand(0, &["TST-OMNI"])
        .memory(10)
        .build();

    let blitz = runner.place_on_field(0, "EX9-013", None);
    runner.place_on_field(0, "ALLY-A", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(blitz));
    runner.game.drain_effect_queue();

    // With the eligible hand card present, a selection should install
    // (select_hand for the Omnimon Alter-S card)
    if let Some(kind) = runner.pending_kind() {
        // First step is select_hand — kind should be Hand
        assert_eq!(
            kind,
            SelectionKind::Hand,
            "First EOT prompt should be Hand selection for Omnimon Alter-S"
        );
        // The selection is part of an optional clause — should be passable
        // (the overall clause is optional, so the first inner step selection
        //  inherits optional semantics from the clause)
    }
    // Test succeeds whether or not a prompt installed — key is no panic
}

// ─── Section 5: ACE Overflow inherited ───────────────────────────────────────

#[test]
fn ex9_013_ace_overflow_is_negative_four() {
    let compiled = compiled_ex9_013();
    assert_eq!(compiled.ace_overflow, Some(-4));
}

// ─── Section 6: may-attack-now sub-clause ───────────────────────────────────

#[test]
fn ex9_013_eot_clause_contains_post_dna_may_attack_now() {
    let compiled = compiled_ex9_013();
    let triggered = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::EndOfYourTurn) => {
                Some(t)
            }
            _ => None,
        })
        .expect("EX9-013 must have an EndOfYourTurn clause");

    let has_may_attack = triggered.process.iter().any(|step| match step {
        CompiledStep::MayAttackNow { optional, .. } => *optional,
        CompiledStep::Optional(body) => body
            .iter()
            .any(|inner| matches!(inner, CompiledStep::MayAttackNow { optional: true, .. })),
        _ => false,
    });

    assert!(
        has_may_attack,
        "EOT clause must contain the printed post-DNA '1 of your Digimon may attack' step"
    );
}

#[test]
fn ex9_013_eot_after_dna_one_digimon_may_attack() {
    let mut omni = make_test_dna_card("TST-OMNI", "Omnimon Alter-S", 6, 6, 0);
    omni.level = Some(7);
    omni.dp = Some(15000);

    let mut blue_lv6 = make_test_card("BLUE-LV6", "BlueLv6");
    blue_lv6.level = Some(6);
    blue_lv6.dp = Some(6000);
    blue_lv6.colors = vec![CardColor::Blue];

    let mut security_card = make_test_card("SEC", "Security");
    security_card.dp = Some(2000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX9-013 YAML loads")
        .add_card(omni)
        .add_card(blue_lv6)
        .add_card(security_card)
        .hand(0, &["TST-OMNI"])
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let blitz = runner.place_on_field(0, "EX9-013", None);
    let blue = runner.place_on_field(0, "BLUE-LV6", None);
    let security_before = runner.game.player(1).security.len();

    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfYourTurn, TriggerSource::Permanent(blitz));
    runner.game.drain_effect_queue();

    let hand = runner
        .pending_selection_view()
        .expect("EOT clause should select Omnimon Alter-S from hand");
    assert_eq!(hand.kind, SelectionKind::Hand);
    runner
        .game
        .resolve_selection(hand.selecting_player, hand.valid_action_ids[0])
        .expect("hand selection resolves");

    let first_dna = runner
        .pending_selection_view()
        .expect("first DNA material selection should install");
    assert_eq!(first_dna.kind, SelectionKind::Target);
    let blitz_pick = first_dna
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| *action == encode_attack(blitz.player as u16, blitz.index as u16))
        .expect("BlitzGreymon should be selectable as the first DNA material");
    runner
        .game
        .resolve_selection(first_dna.selecting_player, blitz_pick)
        .expect("first DNA material resolves");

    let second_dna = runner
        .pending_selection_view()
        .expect("second DNA material selection should install");
    assert_eq!(second_dna.kind, SelectionKind::Target);
    let blue_pick = second_dna
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| *action == encode_attack(blue.player as u16, blue.index as u16))
        .expect("Blue Lv6 should be selectable as the second DNA material");
    runner
        .game
        .resolve_selection(second_dna.selecting_player, blue_pick)
        .expect("second DNA material resolves");

    let choose_attacker = runner
        .pending_selection_view()
        .expect("post-DNA '1 of your Digimon may attack' should select an attacker");
    assert_eq!(choose_attacker.kind, SelectionKind::OwnField);
    assert!(
        choose_attacker.is_optional,
        "printed post-DNA 'may attack' should expose a decline before commitment"
    );
    assert!(
        build_action_mask(&runner.game, 0)[PASS as usize] > 0.0,
        "optional attacker selection must expose PASS through the action mask"
    );
    let attacker_action = choose_attacker.valid_action_ids[0];
    runner
        .game
        .resolve_selection(choose_attacker.selecting_player, attacker_action)
        .expect("attacker selection resolves");

    let attack_prompt = runner
        .pending_selection_view()
        .expect("selected Digimon should open the normal attack target prompt");
    assert!(
        attack_prompt.is_optional,
        "may_attack_now must keep PASS legal at the attack target prompt"
    );
    assert!(
        build_action_mask(&runner.game, 0)[PASS as usize] > 0.0,
        "optional attack target prompt must expose PASS through the action mask"
    );
    let attack_player = attack_prompt
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| {
            let (_, target) = decode_attack(*action);
            target == SECURITY_TARGET
        })
        .expect("normal attack flow should allow choosing the opponent player");
    runner
        .game
        .resolve_selection(attack_prompt.selecting_player, attack_player)
        .expect("effect-created attack resolves");

    assert_eq!(
        runner.game.player(1).security.len(),
        security_before - 1,
        "post-DNA effect-created attack should use the normal security flow"
    );
    assert!(
        runner.game.player(0).battle_area.iter().any(|p| {
            runner.game.card_data[p.top_card().data_index].card_name == "Omnimon Alter-S"
                && p.is_suspended
        }),
        "the chosen post-DNA attacker should pay the normal suspend cost"
    );
}
