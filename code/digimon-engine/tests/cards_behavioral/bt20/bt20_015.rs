//! BT20-015 Hisyaryumon: actual card image and official bundle inspected.
//! Printed-text idioms: BT20-053 shares the optional breeding play / independent
//! attack tail. CR 4.3 8-1-2-1 and 3-4-7 govern evolution requirements/breeding;
//! 13-1-6/8 distinguish actual security checks. Only real masked choices resolve.
//! Tests precede production YAML; the parent owns Cargo and shared primitives.
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, MAX_FIELD_SLOTS, PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{
    make_test_card, make_test_card_with_level, DebugRunner, DebugRunnerBuilder,
};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, GamePhase, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{
    AttackState, AttackTarget, PendingAttack, SelectionKind, TriggerSource,
};

const ID: &str = "BT20-015";

const SECURITY_DIGIMON: &str = r#"
card: SECURITY-DIGIMON
name: Security Digimon witness
kind: digimon
color: [blue]
level: 3
cost: 3
dp: 1000
effects:
  - scope: security
    when: on_security
    process:
      - draw: { of: you, count: 2 }
"#;
const OFF_TURN_HOST: &str = r#"
card: OFF-TURN-HOST
name: Off-turn attack host
kind: digimon
color: [red]
level: 6
cost: 10
dp: 15000
effects:
  - when: start_of_opponents_main_phase
    process:
      - force_attack: { attacker: source, targets: any }
"#;

fn main_action(r: &mut DebugRunner, player: u8, action: u16) {
    assert!(r.pending_selection().is_none());
    assert_eq!(build_action_mask(&r.game, player)[action as usize], 1.0);
    r.game.decode_action(action, player);
}

fn card(id: &str, name: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card_with_level(id, name, level);
    c.colors = vec![color];
    c.dp = Some(if level >= 6 { 15000 } else { 1000 });
    c
}
fn builder(hand: &[&str]) -> DebugRunnerBuilder {
    let mut chronicle = card("CHRONICLE", "Other", 4, CardColor::Blue);
    chronicle.traits = vec!["Chronicle".into()];
    let mut fake = make_test_card("FAKE", "Dorumon");
    fake.level = None;
    fake.dp = None;
    fake.card_kind = CardKind::Tamer;
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("full Hisyaryumon DSL must be implemented; no stub")
        .dsl_card("BT7-056")
        .expect("real Dorumon with On Play search")
        .add_card(card("RYUDA", "Ryudamon", 3, CardColor::Black))
        .add_card(card("NEAR", "Dorumon (X Antibody)", 3, CardColor::Black))
        .add_card(fake)
        .add_card(card("BASE-B", "Plain", 4, CardColor::Black))
        .add_card(card("BASE-R", "Plain", 4, CardColor::Red))
        .add_card(card("BASE-U", "Plain", 4, CardColor::Blue))
        .add_card(card("GIN", "Ginryumon", 4, CardColor::Blue))
        .add_card(card("GIN-NEAR", "Ginryumon X", 4, CardColor::Blue))
        .add_card(chronicle)
        .dsl_card("EX1-072")
        .expect("real Option Security effect")
        .dsl_card("ST1-12")
        .expect("real Tamer Security effect")
        .from_dsl_yaml(SECURITY_DIGIMON)
        .expect("Digimon security witness")
        .from_dsl_yaml(OFF_TURN_HOST)
        .expect("effect-driven off-turn attack")
        .add_card(card("ALLY", "Ally", 6, CardColor::Black))
        .add_card(card("FOE", "Foe", 3, CardColor::Blue))
        .hand(0, hand)
        .deck(0, &["FOE"; 20])
        .deck(1, &["FOE"; 20])
        .security(0, &["FOE"; 5])
        .security(1, &["FOE"; 5])
        .memory(10)
}
fn choose(r: &mut DebugRunner, action: u16) {
    let player = r
        .pending_selection()
        .expect("choice prompt")
        .selecting_player;
    assert_eq!(
        build_action_mask(&r.game, player)[action as usize],
        1.0,
        "never bypass action mask"
    );
    r.execute_action(player, action).expect("legal selection");
}
fn hand_action(r: &DebugRunner, id: &str) -> u16 {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap() as u16
}
fn breeding_id(r: &DebugRunner) -> Option<String> {
    r.game.players[0]
        .breeding_area
        .as_ref()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
}
fn fire(r: &mut DebugRunner, h: PermanentHandle, evo: bool) {
    if evo {
        r.game
            .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
        r.game.drain_effect_queue();
    } else {
        r.fire_on_play(h.player, h.index as usize);
    }
}
fn prohibit(r: &mut DebugRunner, player: u8) {
    r.game.modifiers.add_player_modifier(
        player,
        PlayerModifierEntry::simple(
            ModifierType::CannotPlayDigimonByEffect,
            0,
            Expiry::Permanent,
            None,
            1,
        ),
    );
}
// Directly stages a currently resolving attack window, not an oracle/legal-opening proof.
// Triggered card choices below still use the real action mask and selection dispatcher.
fn attack_window(r: &mut DebugRunner, attacker: PermanentHandle) {
    r.set_first_player(attacker.player);
    r.game.pending_attack = Some(PendingAttack {
        attacker,
        original_target: AttackTarget::Player(1 - attacker.player),
        effective_target: AttackTarget::Player(1 - attacker.player),
        effective_target_removed: false,
        is_blocked: false,
        blocker: None,
        is_vortex: false,
        is_overclock: false,
        declaration_committed: true,
        cancelled: false,
        battle_occurred: false,
        battle_defender_deleted: false,
        effect_battle_piercing_pending: false,
        security_check_started: false,
        return_phase: GamePhase::Main,
        state: AttackState::Declared,
        counter_depth: 0,
    });
}
#[test]
fn breeding_prompt_is_optional_exact_name_digimon_only_and_preserves_battle_area() {
    for selected in ["BT7-056", "RYUDA"] {
        let mut r = builder(&[ID, "BT7-056", "RYUDA", "NEAR", "FAKE"]).start();
        r.skip_mulligan();
        r.game.enter_main_phase();
        let before = r.memory();
        main_action(&mut r, 0, 0);
        let p = r.pending_selection().expect("breeding hand choice");
        assert_eq!(p.kind, SelectionKind::Hand);
        assert!(p.is_optional);
        let mask = build_action_mask(&r.game, 0);
        for id in ["BT7-056", "RYUDA"] {
            assert_eq!(mask[hand_action(&r, id) as usize], 1.0);
        }
        for id in ["NEAR", "FAKE"] {
            assert_eq!(mask[hand_action(&r, id) as usize], 0.0);
        }
        assert_eq!(mask[PASS as usize], 1.0);
        let a = hand_action(&r, selected);
        choose(&mut r, a);
        assert_eq!(breeding_id(&r).as_deref(), Some(selected));
        assert_eq!(
            r.battle_area_size(0),
            1,
            "rookie goes into breeding, not battle"
        );
        assert_eq!(
            r.memory(),
            before - 7,
            "only Hisyaryumon's printed cost is paid"
        );
        assert!(
            r.pending_selection().is_none(),
            "breeding Dorumon must not fire On Play search"
        );
    }
}
#[test]
fn when_digivolving_offers_same_free_breeding_play() {
    let mut r = builder(&[ID, "RYUDA"]).start();
    let base = r.place_on_field(0, "BASE-B", Some(0));
    r.skip_mulligan();
    r.game.enter_main_phase();
    main_action(&mut r, 0, encode_digivolve(0, base.index as u16));
    let a = hand_action(&r, "RYUDA");
    choose(&mut r, a);
    assert_eq!(breeding_id(&r).as_deref(), Some("RYUDA"));
    assert_eq!(r.memory(), 6, "standard evolution4; breeding play is free");
}
#[test]
fn decline_is_exposed_and_does_not_play_or_charge_rookie() {
    for evo in [false, true] {
        let mut r = builder(&["RYUDA"]).start();
        let h = r.place_on_field(0, ID, Some(0));
        fire(&mut r, h, evo);
        choose(&mut r, PASS);
        assert_eq!(r.hand_size(0), 1);
        assert_eq!(breeding_id(&r), None);
        assert_eq!(r.memory(), 10);
        assert!(r.pending_selection().is_none());
    }
}
#[test]
fn occupied_or_prohibited_breeding_does_not_offer_impossible_hand_choice() {
    for evo in [false, true] {
        for occupied in [false, true] {
            let mut r = builder(&["RYUDA"]).start();
            let h = r.place_on_field(0, ID, Some(0));
            if occupied {
                r.place_in_breeding(0, "FOE");
            } else {
                prohibit(&mut r, 0);
            }
            fire(&mut r, h, evo);
            assert!(r.pending_selection().is_none());
            assert_eq!(r.hand_size(0), 1);
            assert_eq!(
                breeding_id(&r).as_deref(),
                if occupied { Some("FOE") } else { None }
            );
        }
    }
}
#[test]
fn opponent_prohibition_does_not_block_our_breeding_play() {
    let mut r = builder(&["RYUDA"]).start();
    let h = r.place_on_field(0, ID, Some(0));
    prohibit(&mut r, 1);
    fire(&mut r, h, false);
    choose(&mut r, 0);
    assert_eq!(breeding_id(&r).as_deref(), Some("RYUDA"));
}
#[test]
fn full_battle_area_does_not_block_empty_breeding_destination() {
    let mut r = builder(&["RYUDA"]).start();
    let h = r.place_on_field(0, ID, Some(0));
    for _ in 1..MAX_FIELD_SLOTS {
        r.place_on_field(0, "ALLY", Some(0));
    }
    assert_eq!(r.battle_area_size(0), MAX_FIELD_SLOTS as usize);
    fire(&mut r, h, false);
    choose(&mut r, 0);
    assert_eq!(breeding_id(&r).as_deref(), Some("RYUDA"));
    assert_eq!(r.battle_area_size(0), MAX_FIELD_SLOTS as usize);
}
#[test]
fn clone_resume_preserves_optional_breeding_choice_and_context() {
    let mut r = builder(&["RYUDA"]).start();
    let h = r.place_on_field(0, ID, Some(0));
    fire(&mut r, h, false);
    let mut clone = DebugRunner::wrap(r.game.clone());
    choose(&mut r, 0);
    choose(&mut clone, 0);
    assert_eq!(breeding_id(&r), breeding_id(&clone));
    assert_eq!(r.hand_size(0), clone.hand_size(0));
}
#[test]
fn attack_tail_still_runs_after_decline_on_either_players_attack() {
    for attacker_player in [0, 1] {
        let mut r = builder(&["RYUDA"]).start();
        let h = r.place_on_field(0, ID, Some(0));
        let ally = r.place_on_field(0, "ALLY", Some(0));
        let tamer = r.place_on_field(0, "FAKE", Some(0));
        let enemy = r.place_on_field(1, "FOE", Some(0));
        let attacker = if attacker_player == 0 {
            h
        } else {
            r.place_on_field(1, "FOE", Some(0))
        };
        attack_window(&mut r, attacker);
        fire(&mut r, h, true);
        choose(&mut r, PASS);
        let p = r
            .pending_selection()
            .expect("independent Then must still select a Digimon");
        assert!(!p.is_optional);
        assert_eq!(p.selecting_player, 0);
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
        let mask = build_action_mask(&r.game, 0);
        assert_eq!(mask[(100 + h.index as u16) as usize], 1.0);
        assert_eq!(mask[(100 + ally.index as u16) as usize], 1.0);
        assert_eq!(mask[(100 + tamer.index as u16) as usize], 0.0);
        assert_eq!(
            mask[(100 + MAX_FIELD_SLOTS + enemy.index as u16) as usize],
            0.0
        );
        let pick = 100 + ally.index as u16;
        choose(&mut r, pick);
        assert_eq!(r.effective_dp(ally), Some(20000));
        assert_eq!(r.effective_dp(h), Some(7000));
        assert_eq!(r.hand_size(0), 1);
        assert_eq!(breeding_id(&r), None);
        assert_eq!(r.game.effective_security_strike(ally), 2);
        assert_eq!(r.game.effective_security_strike(h), 1);
    }
}
#[test]
fn blocked_breeding_does_not_skip_independent_attack_buff() {
    for occupied in [false, true] {
        let mut r = builder(&["RYUDA"]).start();
        let h = r.place_on_field(0, ID, Some(0));
        if occupied {
            r.place_in_breeding(0, "FOE");
        } else {
            prohibit(&mut r, 0);
        }
        attack_window(&mut r, h);
        fire(&mut r, h, false);
        let p = r
            .pending_selection()
            .expect("mandatory attack buff despite impossible first part");
        assert!(!p.is_optional);
        assert_ne!(p.kind, SelectionKind::Hand);
        choose(&mut r, 100 + h.index as u16);
        assert_eq!(r.effective_dp(h), Some(12000));
        assert_eq!(r.game.effective_security_strike(h), 2);
        assert_eq!(r.hand_size(0), 1);
    }
}
#[test]
fn attack_buff_expires_at_opponents_turn_end_not_own_turn_end() {
    let mut r = builder(&[]).start();
    let h = r.place_on_field(0, ID, Some(0));
    attack_window(&mut r, h);
    fire(&mut r, h, false);
    choose(&mut r, 100 + h.index as u16);
    assert_eq!(r.effective_dp(h), Some(12000));
    assert_eq!(r.game.effective_security_strike(h), 2);
    r.game.modifiers.expire_end_of_turn(0);
    assert_eq!(r.effective_dp(h), Some(12000));
    assert_eq!(r.game.effective_security_strike(h), 2);
    r.game.modifiers.expire_end_of_turn(1);
    assert_eq!(r.effective_dp(h), Some(7000));
    assert_eq!(r.game.effective_security_strike(h), 1);
}
#[test]
fn no_attack_means_no_buff_prompt_even_if_rookie_is_missing() {
    let mut r = builder(&[]).start();
    let h = r.place_on_field(0, ID, Some(0));
    fire(&mut r, h, false);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.effective_dp(h), Some(7000));
    assert_eq!(r.game.effective_security_strike(h), 1);
}

#[test]
fn evolution_routes_include_both_standard_colors_and_exact_special_routes() {
    for (base, cost, legal) in [
        ("BASE-B", 4, true),
        ("BASE-R", 4, true),
        ("BASE-U", 0, false),
        ("GIN", 3, true),
        ("CHRONICLE", 3, true),
        ("GIN-NEAR", 0, false),
    ] {
        let mut r = builder(&[ID]).start();
        r.skip_mulligan();
        r.game.enter_main_phase();
        let h = r.place_on_field(0, base, Some(0));
        let action = encode_digivolve(0, h.index as u16);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize],
            if legal { 1.0 } else { 0.0 },
            "route {base}"
        );
        if legal {
            main_action(&mut r, 0, action);
            assert_eq!(r.memory(), 10 - cost, "route {base}");
            assert_eq!(
                r.game.players[0].battle_area[h.index as usize]
                    .top_card()
                    .card_id(&r.game.card_data),
                ID
            );
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn accepted_breeding_play_continues_to_mandatory_attack_buff() {
    for evo in [false, true] {
        let mut r = builder(&["RYUDA"]).start();
        let h = r.place_on_field(0, ID, Some(0));
        attack_window(&mut r, h);
        fire(&mut r, h, evo);
        choose(&mut r, 0);
        assert_eq!(breeding_id(&r).as_deref(), Some("RYUDA"));
        let p = r
            .pending_selection()
            .expect("Then selects an own battle Digimon");
        assert!(!p.is_optional);
        assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
        choose(&mut r, 100 + h.index as u16);
        assert_eq!(r.effective_dp(h), Some(12000));
        assert_eq!(r.game.effective_security_strike(h), 2);
    }
}

fn security_runner(owner: u8, security: &str) -> DebugRunner {
    let mut r = builder(&[])
        .security(0, &[security; 4])
        .security(1, &[security; 4])
        .start();
    r.skip_mulligan();
    r.set_first_player(owner);
    r.game.enter_main_phase();
    r
}
fn attack(r: &mut DebugRunner, h: PermanentHandle) {
    main_action(r, h.player, encode_attack(h.index as u16, SECURITY_TARGET));
    assert!(r.pending_selection().is_none());
    assert!(r.game.pending_security.is_none());
    assert!(r.game.pending_attack.is_none());
}
fn option_in_hand(r: &DebugRunner, owner: u8) -> bool {
    r.game.players[owner as usize]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "EX1-072")
}

#[test]
fn inherited_effect_suppresses_real_option_security_for_either_owner() {
    for owner in [0, 1] {
        let defender = 1 - owner;
        let mut r = security_runner(owner, "EX1-072");
        let h = r.place_stack(owner, &[ID, "ALLY"]);
        let security = r.security_count(defender);
        attack(&mut r, h);
        assert_eq!(r.security_count(defender), security - 1);
        assert!(
            !option_in_hand(&r, defender),
            "suppressed Security does not add itself to hand"
        );
        assert!(
            !r.game.option_use_blocked(owner),
            "suppressed Security does not apply its restriction"
        );
        assert!(r.game.players[defender as usize]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EX1-072"));
    }
}

#[test]
fn inherited_effect_does_not_suppress_digimon_or_tamer_security() {
    for security in ["SECURITY-DIGIMON", "ST1-12"] {
        let mut r = security_runner(0, security);
        let h = r.place_stack(0, &[ID, "ALLY"]);
        let before = r.hand_size(1);
        attack(&mut r, h);
        assert_eq!(r.security_count(1), 3);
        if security == "SECURITY-DIGIMON" {
            assert_eq!(
                r.hand_size(1),
                before + 2,
                "Digimon Security draw remains active"
            );
        } else {
            assert_eq!(
                r.battle_area_size(1),
                1,
                "Tamer Security play remains active"
            );
            assert_eq!(
                r.game.players[1].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                "ST1-12"
            );
        }
    }
}

#[test]
fn top_card_face_down_source_and_removed_source_do_not_supply_inherited_effect() {
    for mode in ["top", "face-down", "removed"] {
        let mut r = security_runner(0, "EX1-072");
        let h = if mode == "top" {
            r.place_on_field(0, ID, Some(0))
        } else {
            r.place_stack(0, &[ID, "ALLY"])
        };
        if mode == "face-down" {
            r.game.players[0].battle_area[h.index as usize].card_sources[0].face_down = true;
        }
        if mode == "removed" {
            r.game.tick_declarative_effects();
            r.trash_one_source(h);
        }
        attack(&mut r, h);
        assert!(
            option_in_hand(&r, 1),
            "Option Security must activate for {mode}"
        );
        assert!(r.game.option_use_blocked(0));
    }
}

#[test]
fn another_attacker_cannot_borrow_hisyaryumons_inherited_effect() {
    let mut r = security_runner(0, "EX1-072");
    r.place_stack(0, &[ID, "ALLY"]);
    let other = r.place_on_field(0, "ALLY", Some(0));
    attack(&mut r, other);
    assert!(option_in_hand(&r, 1));
    assert!(r.game.option_use_blocked(0));
}

#[test]
fn inherited_your_turn_gate_allows_option_security_on_real_off_turn_attack() {
    let mut r = security_runner(0, "EX1-072");
    r.place_stack(0, &[ID, "OFF-TURN-HOST"]);
    r.game.tick_declarative_effects();
    main_action(&mut r, 0, PASS);
    assert_eq!(r.turn_player(), 1);
    // This fixture has no Digi-Eggs or breeding permanent, so the turn machine
    // automatically enters Main and parks on the host's mandatory effect attack.
    assert_eq!(r.current_phase(), GamePhase::SelectTarget);
    let p = r
        .pending_selection()
        .expect("off-turn host chooses its attack target");
    assert_eq!(p.selecting_player, 0);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    choose(&mut r, encode_attack(0, SECURITY_TARGET));
    assert!(r.pending_selection().is_none());
    assert!(option_in_hand(&r, 1));
    assert!(r.game.option_use_blocked(0));
    assert_eq!(r.security_count(1), 3);
}
