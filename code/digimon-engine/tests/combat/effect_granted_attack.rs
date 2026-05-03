use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::{AttackTargetRestriction, EffectContext};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
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
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

#[test]
fn may_attack_now_installs_attack_prompt_with_player_only_target() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", 5000))
        .add_card(make_digimon("DEF", 3000))
        .start();
    let p0 = r.game.turn_player();
    let p1 = 1 - p0;
    let attacker = r.place_on_field(p0, "ATK", Some(0));
    let defender = r.place_on_field(p1, "DEF", Some(0));
    r.game.player_mut(p1).battle_area[defender.index as usize].is_suspended = true;
    let source_card = r.top_card(attacker);

    {
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(attacker), p0);
        ctx.may_attack_now(
            attacker,
            AttackTargetRestriction::PlayerOnly,
            false,
            "Attack with this Digimon?",
        )
        .expect("install attack prompt");
    }

    assert_eq!(r.game.current_phase, GamePhase::SelectTarget);
    let pending = r.game.pending_selection.as_ref().expect("attack prompt");
    assert_eq!(pending.kind, SelectionKind::Target);
    assert_eq!(pending.selecting_player, p0);
    assert_eq!(
        pending.valid_action_ids,
        vec![encode_attack(attacker.index as u16, SECURITY_TARGET)]
    );
    assert!(
        !pending
            .valid_action_ids
            .contains(&encode_attack(attacker.index as u16, defender.index as u16)),
        "digimon targets should be filtered out"
    );

    let mask = build_action_mask(&r.game, p0);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "player attack should be exposed through the action mask"
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize],
        0.0,
        "filtered digimon target should not be exposed through the action mask"
    );
}

#[test]
fn may_attack_now_without_suspending_resolves_real_attack_flow() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", 7000))
        .add_card(make_digimon("SEC", 2000))
        .security(1, &["SEC"])
        .start();
    let p0 = r.game.turn_player();
    let p1 = 1 - p0;
    let attacker = r.place_on_field(p0, "ATK", Some(0));
    r.game.player_mut(p0).battle_area[attacker.index as usize].is_suspended = true;
    let source_card = r.top_card(attacker);
    let security_before = r.game.player(p1).security.len();

    {
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(attacker), p0);
        ctx.may_attack_now(
            attacker,
            AttackTargetRestriction::PlayerOnly,
            true,
            "Attack without suspending?",
        )
        .expect("install attack prompt");
    }

    let action = encode_attack(attacker.index as u16, SECURITY_TARGET);
    r.game
        .resolve_selection(p0, action)
        .expect("resolve effect-granted attack");

    assert_eq!(
        r.game.player(p1).security.len(),
        security_before - 1,
        "effect-granted attack should run the normal security flow"
    );
    assert!(
        r.game.player(p0).battle_area[attacker.index as usize].is_suspended,
        "without_suspending should allow and preserve an already-suspended attacker"
    );
}
