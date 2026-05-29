use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::{AttackTargetRestriction, EffectContext};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

fn make_digimon(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        keywords: Vec::new(),
        ace_overflow: None,
        dual: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

#[test]
fn temporary_attack_window_prompts_for_fresh_played_attacker_and_can_decline() {
    let mut runner = DebugRunner::builder()
        .add_card(make_digimon("FRESH", 5000))
        .add_card(make_digimon("SEC", 2000))
        .security(1, &["SEC"])
        .start();
    runner.game.turn_count = 5;
    let attacker = runner.place_on_field(0, "FRESH", Some(5));
    let source = runner.top_card(attacker);

    {
        let mut ctx = EffectContext::new(&mut runner.game, source, Some(attacker), 0);
        ctx.may_attack_now_ignoring_summoning_sickness(
            attacker,
            AttackTargetRestriction::PlayerOnly,
            false,
            true,
            "This Digimon may attack",
        )
        .expect("install temporary attack window");
    }

    let attack_action = encode_attack(attacker.index as u16, SECURITY_TARGET);
    let pending = runner
        .pending_selection_view()
        .expect("fresh attacker should receive an optional attack prompt");
    assert_eq!(pending.kind, SelectionKind::Target);
    assert!(pending.is_optional);
    assert!(pending.valid_action_ids.contains(&attack_action));
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[PASS as usize], 1.0);
    assert_eq!(mask[attack_action as usize], 1.0);

    runner
        .execute_action(0, PASS)
        .expect("decline optional attack");
    assert!(runner.game.pending_attack.is_none());
    assert!(!runner.game.players[0].battle_area[attacker.index as usize].is_suspended);
    assert_eq!(runner.game.players[1].security.len(), 1);
}

#[test]
fn temporary_attack_window_routes_through_normal_attack_resolution() {
    let mut runner = DebugRunner::builder()
        .add_card(make_digimon("FRESH", 5000))
        .add_card(make_digimon("SEC", 2000))
        .security(1, &["SEC"])
        .start();
    runner.game.turn_count = 5;
    let attacker = runner.place_on_field(0, "FRESH", Some(5));
    let source = runner.top_card(attacker);

    {
        let mut ctx = EffectContext::new(&mut runner.game, source, Some(attacker), 0);
        ctx.may_attack_now_ignoring_summoning_sickness(
            attacker,
            AttackTargetRestriction::PlayerOnly,
            false,
            true,
            "This Digimon may attack",
        )
        .expect("install temporary attack window");
    }

    let attack_action = encode_attack(attacker.index as u16, SECURITY_TARGET);
    runner
        .execute_action(0, attack_action)
        .expect("resolve effect-created attack");

    assert!(runner.game.players[0].battle_area[attacker.index as usize].is_suspended);
    assert_eq!(
        runner.game.players[1].security.len(),
        0,
        "effect-created attack should use the normal security-check flow"
    );
}

#[test]
fn temporary_attack_window_can_follow_named_unsuspend_setup() {
    let mut runner = DebugRunner::builder()
        .add_card(make_digimon("SHOUTMON-EX6", 5000))
        .add_card(make_digimon("SEC", 2000))
        .security(1, &["SEC"])
        .start();
    runner.game.turn_count = 5;
    let attacker = runner.place_on_field(0, "SHOUTMON-EX6", Some(5));
    let source = runner.top_card(attacker);
    runner.game.players[0].battle_area[attacker.index as usize].is_suspended = true;

    {
        let mut ctx = EffectContext::new(&mut runner.game, source, Some(attacker), 0);
        let top_name = ctx.game.players[0].battle_area[attacker.index as usize]
            .top_card()
            .card_name(&ctx.game.card_data)
            .to_string();
        assert!(top_name.contains("SHOUTMON-EX6"));
        ctx.unsuspend(attacker);
        ctx.may_attack_now_ignoring_summoning_sickness(
            attacker,
            AttackTargetRestriction::PlayerOnly,
            false,
            true,
            "Named Digimon may attack",
        )
        .expect("install attack prompt after setup");
    }

    let attack_action = encode_attack(attacker.index as u16, SECURITY_TARGET);
    assert!(runner
        .pending_selection_view()
        .expect("unsuspended named body should receive attack prompt")
        .valid_action_ids
        .contains(&attack_action));
}

#[test]
fn temporary_attack_window_noops_without_legal_attacker_or_target() {
    let mut runner = DebugRunner::builder()
        .add_card(make_digimon("FRESH", 5000))
        .start();
    runner.game.turn_count = 5;
    let attacker = runner.place_on_field(0, "FRESH", Some(5));
    let source = runner.top_card(attacker);

    runner.game.players[0].battle_area[attacker.index as usize].is_suspended = true;
    {
        let mut ctx = EffectContext::new(&mut runner.game, source, Some(attacker), 0);
        ctx.may_attack_now_ignoring_summoning_sickness(
            attacker,
            AttackTargetRestriction::PlayerOnly,
            false,
            true,
            "No attack",
        )
        .expect("no legal attacker path should not error");
    }
    assert!(runner.pending_selection().is_none());

    runner.game.players[0].battle_area[attacker.index as usize].is_suspended = false;
    {
        let mut ctx = EffectContext::new(&mut runner.game, source, Some(attacker), 0);
        ctx.may_attack_now_ignoring_summoning_sickness(
            attacker,
            AttackTargetRestriction::DigimonOnly,
            false,
            true,
            "No target",
        )
        .expect("no legal target path should not error");
    }
    assert!(runner.pending_selection().is_none());
}
