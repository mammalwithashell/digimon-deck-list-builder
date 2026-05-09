use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::{AttackCostUpgrade, AttackInitiator, AttackOpen, TargetConstraint};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::{AttackTargetRestriction, EffectContext};
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::selection::{AttackTarget, SelectionKind};

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
fn may_attack_now_exposes_decline_pass_without_attack_side_effects() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", 5000))
        .add_card(make_digimon("SEC", 2000))
        .security(1, &["SEC"])
        .start();
    let p0 = r.game.turn_player();
    let p1 = 1 - p0;
    let attacker = r.place_on_field(p0, "ATK", Some(0));
    let source_card = r.top_card(attacker);
    let security_before = r.game.player(p1).security.len();

    {
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(attacker), p0);
        ctx.may_attack_now(
            attacker,
            AttackTargetRestriction::PlayerOnly,
            false,
            "This Digimon may attack.",
        )
        .expect("install optional effect-created attack prompt");
    }

    let pending = r.game.pending_selection.as_ref().expect("attack prompt");
    assert!(
        pending.is_optional,
        "effect-created 'may attack' prompts must expose decline/PASS"
    );

    let mask = build_action_mask(&r.game, p0);
    assert_eq!(
        mask[PASS as usize], 1.0,
        "decline/PASS must be legal while the optional attack prompt is open"
    );

    r.game
        .resolve_selection(p0, PASS)
        .expect("decline optional effect-created attack");

    assert!(r.game.pending_attack.is_none());
    assert_eq!(r.game.player(p1).security.len(), security_before);
    assert!(
        !r.game.player(p0).battle_area[attacker.index as usize].is_suspended,
        "declining the optional attack must not pay the attack suspend cost"
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

#[test]
fn attack_open_entry_supports_effect_attacks_without_suspending() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", 7000))
        .add_card(make_digimon("SEC", 2000))
        .security(1, &["SEC"])
        .start();
    let p0 = r.game.turn_player();
    let p1 = 1 - p0;
    let attacker = r.place_on_field(p0, "ATK", Some(0));
    let source_card = r.top_card(attacker);
    let security_before = r.game.player(p1).security.len();

    let result = r.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: Some(source_card),
            optional: false,
        },
        suspend_attacker: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(p1)),
        allow_cancel: false,
        cost_upgrade: None,
    });

    assert_ne!(result, digimon_engine::combat::AttackResult::Invalid);
    assert_eq!(r.game.player(p1).security.len(), security_before - 1);
    assert!(
        !r.game.player(p0).battle_area[attacker.index as usize].is_suspended,
        "central effect attack entry must honor suspend_attacker=false"
    );
}

#[test]
fn attack_open_cost_upgrade_applies_for_attack_only() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", 7000))
        .add_card(make_digimon("SEC", 2000))
        .security(1, &["SEC"])
        .start();
    let p0 = r.game.turn_player();
    let p1 = 1 - p0;
    let attacker = r.place_on_field(p0, "ATK", Some(0));
    let source_card = r.top_card(attacker);

    let result = r.game.begin_attack_open(AttackOpen {
        attacker,
        initiator: AttackInitiator::Effect {
            source: Some(source_card),
            optional: false,
        },
        suspend_attacker: false,
        target_constraint: TargetConstraint::Forced(AttackTarget::Player(p1)),
        allow_cancel: false,
        cost_upgrade: Some(AttackCostUpgrade {
            dp: 3000,
            security_attack: 1,
        }),
    });

    assert_ne!(result, digimon_engine::combat::AttackResult::Invalid);
    assert_eq!(
        r.game.modifiers.sum(attacker, ModifierType::ChangeDp),
        0,
        "cost-upgrade DP must expire at EndOfAttack"
    );
    assert_eq!(
        r.game
            .modifiers
            .sum(attacker, ModifierType::SecurityAttackChange),
        0,
        "cost-upgrade security attack must expire at EndOfAttack"
    );
}

#[test]
fn cannot_attack_player_blocks_mask_and_shared_attack_entry() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", 7000))
        .add_card(make_digimon("SEC", 2000))
        .security(1, &["SEC"])
        .start();
    let p0 = r.game.turn_player();
    let p1 = 1 - p0;
    let attacker = r.place_on_field(p0, "ATK", Some(0));
    let action = encode_attack(attacker.index as u16, SECURITY_TARGET);

    assert_eq!(
        build_action_mask(&r.game, p0)[action as usize],
        1.0,
        "baseline direct-player attack should be legal"
    );

    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::CannotAttackPlayer, 1, Expiry::EndOfTurn, p0),
    );

    assert_eq!(
        build_action_mask(&r.game, p0)[action as usize],
        0.0,
        "CannotAttackPlayer must hide direct-player attacks from the mask"
    );
    assert_eq!(
        r.game.attack_player(attacker, p1, false),
        digimon_engine::combat::AttackResult::Invalid,
        "the shared combat entry must also reject direct-player attacks"
    );
    assert!(r.game.pending_attack.is_none());
    assert!(
        !r.game.player(p0).battle_area[attacker.index as usize].is_suspended,
        "rejected attacks must not pay the suspend cost"
    );
}

#[test]
fn force_opponent_attack_prompts_attacker_controller() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("SRC", 5000))
        .add_card(make_digimon("OPP", 6000))
        .add_card(make_digimon("SEC", 2000))
        .security(0, &["SEC"])
        .start();
    let p0 = r.game.turn_player();
    let p1 = 1 - p0;
    let source = r.place_on_field(p0, "SRC", Some(0));
    let forced_attacker = r.place_on_field(p1, "OPP", Some(0));
    let source_card = r.top_card(source);
    let security_before = r.game.player(p0).security.len();

    {
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), p0);
        ctx.force_opponent_attack(
            forced_attacker,
            AttackTargetRestriction::PlayerOnly,
            false,
            "Your Digimon must attack.",
        )
        .expect("install forced opponent attack prompt");
    }

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("forced attack prompt");
    assert_eq!(pending.selecting_player, p1);
    assert!(
        !pending.is_optional,
        "forced attacks must not expose decline/PASS"
    );

    let action = encode_attack(forced_attacker.index as u16, SECURITY_TARGET);
    let p1_mask = build_action_mask(&r.game, p1);
    assert_eq!(p1_mask[action as usize], 1.0);
    assert_eq!(p1_mask[PASS as usize], 0.0);

    r.game
        .resolve_selection(p1, action)
        .expect("opponent resolves forced attack");

    assert_eq!(r.game.player(p0).security.len(), security_before - 1);
    assert!(
        r.game.player(p1).battle_area[forced_attacker.index as usize].is_suspended,
        "forced natural-style attack should pay the suspend cost"
    );
}
