//! Full-card Grademon acceptance tests for the Q6238 supporting challenger.
//! Official observation 9b8945653a1aef65ee588df05e400d4da26013a0b9c9d4ff73effba87bc2eebc;
//! actual BT20-053 image; Q4721; base DCGO BT20/Black/BT20_053.cs.
//! Initial TDD RED was confirmed by the parent before production card creation.
//! Parent owns Cargo execution and shared DSL implementation.
//! Synthetic metadata cards isolate names/colors/traits, not replacement card effects.
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{CONCEDE_GAME, MAX_FIELD_SLOTS, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{
    CardColor, CardKind, EffectSourceKind, EffectTiming, Expiry, GamePhase, ModifierType,
    PlaySource,
};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{
    AttackState, AttackTarget, PendingAttack, SelectionKind, TriggerSource,
};

const ID: &str = "BT20-053";
fn card(id: &str, name: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card_with_level(id, name, level);
    c.colors = vec![color];
    c.dp = Some(if level >= 6 { 15000 } else { 1000 });
    c
}
fn builder(hand: &[&str]) -> DebugRunnerBuilder {
    let mut chronicle = card("CHRONICLE", "Other", 4, CardColor::Blue);
    chronicle.traits = vec!["Chronicle".into()];
    let mut fake = card("FAKE", "Dorumon", 3, CardColor::Black);
    fake.card_kind = CardKind::Tamer;
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("full Grademon DSL must be implemented; no stub")
        .dsl_card("BT7-056")
        .expect("real Dorumon with On Play search")
        .add_card(card("RYUDA", "Ryudamon", 3, CardColor::Black))
        .add_card(card("NEAR", "Dorumon (X Antibody)", 3, CardColor::Black))
        .add_card(fake)
        .add_card(card("BASE-B", "Plain", 4, CardColor::Black))
        .add_card(card("BASE-Y", "Plain", 4, CardColor::Yellow))
        .add_card(card("BASE-U", "Plain", 4, CardColor::Blue))
        .add_card(card("RAPTOR", "Raptordramon", 4, CardColor::Blue))
        .add_card(chronicle)
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
        let before = r.memory();
        r.play(0, 0).expect("Grademon normal play");
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
            "only Grademon's printed cost is paid"
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
    assert!(r
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve));
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
        let pick = 100 + ally.index as u16;
        choose(&mut r, pick);
        assert_eq!(r.effective_dp(ally), Some(20000));
        assert_eq!(r.effective_dp(h), Some(7000));
        assert_eq!(r.hand_size(0), 1);
        assert_eq!(breeding_id(&r), None);
        assert!(r
            .game
            .permanent_is_unaffected_by_effect(ally, 1, EffectSourceKind::Digimon));
        assert!(!r
            .game
            .permanent_is_unaffected_by_effect(ally, 1, EffectSourceKind::Option));
        assert!(!r
            .game
            .permanent_is_unaffected_by_effect(ally, 1, EffectSourceKind::Tamer));
        assert!(!r
            .game
            .permanent_is_unaffected_by_effect(ally, 0, EffectSourceKind::Digimon));
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
    r.game.modifiers.expire_end_of_turn(0);
    assert_eq!(r.effective_dp(h), Some(12000));
    r.game.modifiers.expire_end_of_turn(1);
    assert_eq!(r.effective_dp(h), Some(7000));
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(h, 1, EffectSourceKind::Digimon));
}
#[test]
fn no_attack_means_no_buff_prompt_even_if_rookie_is_missing() {
    let mut r = builder(&[]).start();
    let h = r.place_on_field(0, ID, Some(0));
    fire(&mut r, h, false);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.effective_dp(h), Some(7000));
    assert!(!r
        .game
        .permanent_is_unaffected_by_effect(h, 1, EffectSourceKind::Digimon));
}
fn redirect_fixture() -> (DebugRunner, PermanentHandle) {
    let mut r = builder(&[]).start();
    let host = r.place_stack(0, &[ID, "ALLY"]);
    r.set_first_player(1);
    r.set_turn(2);
    (r, host)
}
fn accept_optional(r: &mut DebugRunner) {
    let p = r.pending_selection().expect("optional redirect");
    assert!(p.is_optional);
    assert_eq!(p.selecting_player, 0);
    let a = *p
        .valid_action_ids
        .iter()
        .find(|a| **a != PASS && **a != CONCEDE_GAME)
        .expect("accept action");
    choose(r, a);
}
#[test]
fn inherited_redirect_accepts_once_then_second_real_attack_cannot_redirect() {
    let (mut r, host) = redirect_fixture();
    let first = r.place_on_field(1, "FOE", Some(0));
    let before = r.security_count(0);
    r.attack_player(first, 0, false);
    accept_optional(&mut r);
    assert_eq!(
        r.security_count(0),
        before,
        "redirected attack battles carrier, not security"
    );
    assert_eq!(r.effective_dp(host), Some(15000));
    let second = r.place_on_field(1, "FOE", Some(0));
    r.attack_player(second, 0, false);
    assert!(
        r.pending_selection().is_none(),
        "OPT actually attempted a second time"
    );
    assert_eq!(r.security_count(0), before - 1);
}
#[test]
fn inherited_redirect_decline_keeps_opt_available_for_second_attack() {
    let (mut r, _) = redirect_fixture();
    let first = r.place_on_field(1, "FOE", Some(0));
    r.attack_player(first, 0, false);
    choose(&mut r, PASS);
    let second = r.place_on_field(1, "FOE", Some(0));
    r.attack_player(second, 0, false);
    accept_optional(&mut r);
    assert_eq!(r.security_count(0), 4);
}
#[test]
fn face_up_grademon_does_not_gain_its_inherited_redirect() {
    let mut r = builder(&[]).start();
    r.place_on_field(0, ID, Some(0));
    r.set_first_player(1);
    r.set_turn(2);
    let attacker = r.place_on_field(1, "FOE", Some(0));
    r.attack_player(attacker, 0, false);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.security_count(0), 4);
}
#[test]
fn evolution_routes_include_both_standard_colors_and_exact_special_routes() {
    for (base, cost, legal) in [
        ("BASE-B", 4, true),
        ("BASE-Y", 4, true),
        ("BASE-U", 0, false),
        ("RAPTOR", 3, true),
        ("CHRONICLE", 3, true),
    ] {
        let mut r = builder(&[ID]).start();
        let h = r.place_on_field(0, base, Some(0));
        let success = r
            .game
            .digivolve_from_hand(0, 0, h.index as usize, PlaySource::ByDigivolve);
        assert_eq!(success, legal, "route {base}");
        if legal {
            assert_eq!(r.memory(), 10 - cost, "route {base}");
        }
    }
}

#[test]
fn accepted_breeding_play_continues_to_mandatory_attack_buff() {
    let mut r = builder(&["RYUDA"]).start();
    let h = r.place_on_field(0, ID, Some(0));
    attack_window(&mut r, h);
    fire(&mut r, h, false);
    choose(&mut r, 0);
    assert_eq!(breeding_id(&r).as_deref(), Some("RYUDA"));
    let p = r
        .pending_selection()
        .expect("Then selects an own battle Digimon");
    assert!(!p.is_optional);
    choose(&mut r, 100 + h.index as u16);
    assert_eq!(r.effective_dp(h), Some(12000));
}
#[test]
fn inherited_redirect_does_not_activate_during_owners_turn() {
    let (mut r, _) = redirect_fixture();
    r.set_first_player(0);
    let attacker = r.place_on_field(1, "FOE", Some(0));
    let card = r.game.players[1].battle_area[attacker.index as usize]
        .top_card()
        .handle();
    // A real opponent attack event can occur during our turn through an effect;
    // stage that event explicitly so this tests the printed turn scope.
    r.game.enqueue_triggered(
        EffectTiming::OnOpponentAttack,
        TriggerSource::PlayerBattleAreaAttack {
            player: 0,
            attacker,
            card,
        },
    );
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
}
