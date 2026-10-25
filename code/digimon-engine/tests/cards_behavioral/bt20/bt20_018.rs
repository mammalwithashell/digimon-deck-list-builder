//! BT20-018 Ouryumon: actual card image and DCGO BT20_018.cs inspected.
//! CR 4.3 16-12-1/4: declare 1..2, stop at Lv.3; 3-4-7-4/6: ordinary
//! evolution effects do not trigger in breeding. Staged attack windows below
//! isolate the printed conditional tail; they are not oracle scenario claims.
use digimon_engine::action::{build_action_mask, space};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::count_choice::CountChoiceOperation;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{AttackState, AttackTarget, PendingAttack, TriggerSource};

const ID: &str = "BT20-018";

fn card(id: &str, level: u8, color: CardColor, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.colors = vec![color];
    c.dp = Some(dp);
    c
}

fn evolution(id: &str, level: u8, chronicle: bool, base: u8, color: CardColor) -> CardData {
    let mut c = card(id, level, CardColor::Black, 9000);
    if chronicle {
        c.traits = vec!["Chronicle".into()];
    }
    c.evo_costs = vec![EvoCost {
        card_color: color as u8,
        level: base,
        memory_cost: 7,
    }];
    c
}

fn builder(hand: &[&str]) -> DebugRunnerBuilder {
    let mut tamer = evolution("TAMER", 6, true, 5, CardColor::Black);
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    let mut exact = card("OURYUKEN", 7, CardColor::Black, 15000);
    exact.card_name = "Alphamon: Ouryuken".into();
    let mut near = exact.clone();
    near.card_id = "NEAR-OURYUKEN".into();
    near.card_name = "Alphamon: Ouryuken (X Antibody)".into();
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT20-018 production YAML must be implemented")
        .dsl_card("BT20-056")
        .expect("real evolution with When Digivolving Recovery")
        .add_card(card("FILL", 3, CardColor::Black, 1000))
        .add_card(card("MID", 4, CardColor::Black, 4000))
        .add_card(card("BASE-B", 5, CardColor::Black, 7000))
        .add_card(card("BASE-R", 5, CardColor::Red, 7000))
        .add_card(card("BASE-U", 5, CardColor::Blue, 7000))
        .add_card(card("LOW-A", 3, CardColor::Blue, 2000))
        .add_card(card("LOW-B", 3, CardColor::Blue, 2000))
        .add_card(card("HIGH", 6, CardColor::Blue, 12000))
        .add_card(evolution("EVO6", 6, true, 5, CardColor::Black))
        .add_card(evolution("EVO5", 5, true, 5, CardColor::Black))
        .add_card(evolution("EVO7", 7, true, 5, CardColor::Black))
        .add_card(evolution("NOT-CHRONICLE", 6, false, 5, CardColor::Black))
        .add_card(evolution("WRONG-COLOR", 6, true, 5, CardColor::Blue))
        .add_card(evolution("WRONG-LEVEL", 6, true, 4, CardColor::Black))
        .add_card(tamer)
        .add_card(exact)
        .add_card(near)
        .hand(0, hand)
        .deck(0, &["FILL"; 30])
        .deck(1, &["FILL"; 30])
        .security(0, &["FILL"; 5])
        .security(1, &["FILL"; 5])
        .memory(10)
}

fn ready(mut r: DebugRunner) -> DebugRunner {
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}

fn main_action(r: &mut DebugRunner, player: u8, action: u16) {
    assert!(r.pending_selection().is_none());
    assert_eq!(build_action_mask(&r.game, player)[action as usize], 1.0);
    r.game.decode_action(action, player);
}

fn choices(r: &DebugRunner, count: usize, optional: bool) -> Vec<u16> {
    let p = r.pending_selection().expect("player-visible choice");
    assert_eq!(p.selecting_player, 0);
    assert_eq!(p.is_optional, optional, "{}", p.prompt);
    let ids: Vec<_> = p
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != space::PASS)
        .collect();
    assert_eq!(ids.len(), count, "{}: {:?}", p.prompt, ids);
    for (a, value) in build_action_mask(&r.game, 0).iter().enumerate() {
        assert_eq!(
            *value,
            if ids.contains(&(a as u16)) || (optional && a == space::PASS as usize) {
                1.0
            } else {
                0.0
            },
            "mask {a}"
        );
    }
    assert!(build_action_mask(&r.game, 1).iter().all(|v| *v == 0.0));
    ids
}

fn choose(r: &mut DebugRunner, action: u16) {
    let owner = r.pending_selection().unwrap().selecting_player;
    assert_eq!(build_action_mask(&r.game, owner)[action as usize], 1.0);
    r.execute_action(owner, action)
        .expect("legal player choice");
}

fn zone_action(r: &DebugRunner, id: &str, trash: bool) -> u16 {
    let zone = if trash {
        &r.game.players[0].trash
    } else {
        &r.game.players[0].hand
    };
    let index = zone
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("zone card");
    if r.game.pending_selection.as_ref().and_then(|p|p.card_choice.as_ref()).is_some() {
        return owned_union_fixture::action(&r.game, 0, id, trash).expect("printed eligible card has a legal physical row");
    }
    (if trash {
        space::TRASH_EFFECT_START
    } else {
        space::PLAY_HAND_START
    }) + index as u16
}

fn breeding_id(r: &DebugRunner) -> Option<String> {
    r.game.players[0]
        .breeding_area
        .as_ref()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
}

fn fire(r: &mut DebugRunner, host: PermanentHandle, evo: bool) {
    if evo {
        r.game.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(host),
        );
        r.game.drain_effect_queue();
    } else {
        r.fire_on_play(host.player, host.index as usize);
    }
}

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

fn remove_security(r: &mut DebugRunner, host: PermanentHandle, owner: u8) {
    let source = r.game.players[host.player as usize].battle_area[host.index as usize]
        .top_card()
        .handle();
    let mut ctx = EffectContext::new(&mut r.game, source, None, host.player);
    assert!(ctx.trash_top_security(owner));
}

#[test]
fn bt20_018_standard_evolution_requires_red_or_black_level_five_and_costs_three() {
    for (base, legal) in [
        ("BASE-B", true),
        ("BASE-R", true),
        ("BASE-U", false),
        ("MID", false),
    ] {
        let mut r = ready(builder(&[ID]).start());
        let host = r.place_on_field(0, base, Some(0));
        let action = space::encode_digivolve(0, host.index as u16);
        assert_eq!(
            build_action_mask(&r.game, 0)[action as usize] > 0.0,
            legal,
            "{base}"
        );
        if legal {
            main_action(&mut r, 0, action);
            assert_eq!(r.memory(), 7);
            assert!(r.game.has_keyword(host, Keyword::Piercing));
            assert_eq!(r.effective_dp(host), Some(11000));
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn bt20_018_piercing_checks_security_after_surviving_battle() {
    let mut r = ready(builder(&[]).start());
    let host = r.place_on_field(0, ID, Some(0));
    let foe = r.place_on_field(1, "LOW-A", Some(0));
    r.game.players[1].battle_area[foe.index as usize].is_suspended = true;
    main_action(
        &mut r,
        0,
        space::encode_attack(host.index as u16, foe.index as u16),
    );
    r.auto_resolve().expect("counter and battle resolve");
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.security_count(1), 4, "Piercing performs one check");
}

#[test]
fn bt20_018_de_digivolve_target_and_one_or_two_card_declaration_are_mandatory() {
    for evo in [false, true] {
        for count in [1, 2] {
            let mut r = ready(builder(&[]).start());
            let host = r.place_on_field(0, ID, Some(0));
            let target = r.place_stack(1, &["FILL", "MID", "BASE-B"]);
            r.place_on_field(1, "TAMER", Some(0));
            fire(&mut r, host, evo);
            let pick = choices(&r, 1, false)[0];
            choose(&mut r, pick);
            choices(&r, 2, false);
            let action = r
                .pending_selection()
                .unwrap()
                .effect_choices
                .as_ref()
                .unwrap()
                .iter()
                .find_map(|entry| match entry.count_choice.map(|v| v.operation) {
                    Some(CountChoiceOperation::Commit(v)) if v.declared == count => {
                        Some(entry.action_id)
                    }
                    _ => None,
                })
                .expect("declared De-Digivolve count");
            choose(&mut r, action);
            assert_eq!(
                r.game.players[1].battle_area[target.index as usize]
                    .card_sources
                    .len(),
                3 - count as usize
            );
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn bt20_018_de_digivolve_stops_at_level_three() {
    let mut r = ready(builder(&[]).start());
    let host = r.place_on_field(0, ID, Some(0));
    let target = r.place_stack(1, &["FILL", "FILL", "MID"]);
    fire(&mut r, host, false);
    let pick = choices(&r, 1, false)[0];
    choose(&mut r, pick);
    assert!(r.pending_selection().is_none());
    assert_eq!(
        r.game.players[1].battle_area[target.index as usize]
            .card_sources
            .len(),
        2
    );
}

#[test]
fn bt20_018_attack_tail_can_use_hand_or_trash_without_opponent_target() {
    for evo in [false, true] {
        for from_trash in [false, true] {
            let mut r = ready(builder(&["BT20-056"]).start());
            if from_trash {
                let c = r.game.players[0].hand.remove(0);
                r.game.players[0].trash.push(c);
            }
            let host = r.place_on_field(0, ID, Some(0));
            r.place_in_breeding(0, "BASE-B");
            attack_window(&mut r, host);
            let hand = r.hand_size(0);
            let deck = r.game.players[0].deck.len();
            fire(&mut r, host, evo);
            let pick = choices(&r, 1, true)[0];
            choose(&mut r, pick);
            let action = zone_action(&r, "BT20-056", from_trash);
            assert!(choices(&r, 1, true).contains(&action));
            choose(&mut r, action);
            assert_eq!(breeding_id(&r).as_deref(), Some("BT20-056"));
            assert_eq!(r.battle_area_size(0), 1, "breeding stack stays in breeding");
            assert_eq!(r.memory(), 10, "free effect evolution");
            assert_eq!(
                r.game.players[0].deck.len(),
                deck - 1,
                "exactly one evolution bonus draw"
            );
            assert_eq!(r.hand_size(0), hand + usize::from(from_trash));
            assert_eq!(
                r.security_count(0),
                5,
                "breeding Alphamon does not trigger Recovery"
            );
            assert!(r.pending_selection().is_none());
        }
    }
}

#[test]
fn bt20_018_union_filters_require_chronicle_level_at_most_six_and_legal_evolution() {
    let mut r = ready(
        builder(&[
            "BT20-056",
            "EVO6",
            "EVO5",
            "EVO7",
            "NOT-CHRONICLE",
            "WRONG-COLOR",
            "WRONG-LEVEL",
            "TAMER",
        ])
        .start(),
    );
    let c = r.game.players[0].hand.remove(1);
    r.game.players[0].trash.push(c);
    let host = r.place_on_field(0, ID, Some(0));
    r.place_in_breeding(0, "BASE-B");
    attack_window(&mut r, host);
    fire(&mut r, host, false);
    let pick = choices(&r, 1, true)[0];
    choose(&mut r, pick);
    let actions = choices(&r, 3, true);
    for (id, trash) in [("BT20-056", false), ("EVO5", false), ("EVO6", true)] {
        assert!(actions.contains(&zone_action(&r, id, trash)), "{id}");
    }
    for id in [
        "EVO7",
        "NOT-CHRONICLE",
        "WRONG-COLOR",
        "WRONG-LEVEL",
        "TAMER",
    ] {
        assert!(owned_union_fixture::action(&r.game, 0, id, false).is_none(), "{id} must have no legal physical row");
    }
    let action = zone_action(&r, "EVO5", false);
    choose(&mut r, action);
    assert_eq!(breeding_id(&r).as_deref(), Some("EVO5"));
}

#[test]
fn bt20_018_attack_tail_can_be_declined_at_breeder_or_card_choice() {
    for decline_card in [false, true] {
        let mut r = ready(builder(&["EVO6"]).start());
        let host = r.place_on_field(0, ID, Some(0));
        r.place_in_breeding(0, "BASE-B");
        attack_window(&mut r, host);
        fire(&mut r, host, false);
        let pick = choices(&r, 1, true)[0];
        if decline_card {
            choose(&mut r, pick);
            choices(&r, 1, true);
        }
        choose(&mut r, space::PASS);
        assert_eq!(breeding_id(&r).as_deref(), Some("BASE-B"));
        assert_eq!(r.hand_size(0), 1);
        assert_eq!(r.game.players[0].deck.len(), 30);
        assert_eq!(r.memory(), 10);
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn bt20_018_attack_tail_requires_attack_breeder_and_legal_candidate() {
    for reason in [
        "no attack",
        "no breeder",
        "no candidate",
        "wrong requirement",
    ] {
        let mut r = ready(
            builder(&[if reason == "no candidate" {
                "NOT-CHRONICLE"
            } else {
                "EVO6"
            }])
            .start(),
        );
        let host = r.place_on_field(0, ID, Some(0));
        if reason != "no breeder" {
            r.place_in_breeding(
                0,
                if reason == "wrong requirement" {
                    "BASE-U"
                } else {
                    "BASE-B"
                },
            );
        }
        if reason != "no attack" {
            attack_window(&mut r, host);
        }
        fire(&mut r, host, false);
        assert!(r.pending_selection().is_none(), "{reason}");
        assert_eq!(r.hand_size(0), 1);
    }
}

#[test]
fn bt20_018_attack_tail_runs_after_de_digivolve_during_opponents_attack() {
    let mut r = ready(builder(&["EVO6"]).start());
    let host = r.place_on_field(0, ID, Some(0));
    let foe = r.place_stack(1, &["FILL", "MID", "BASE-B"]);
    r.place_in_breeding(0, "BASE-B");
    attack_window(&mut r, foe);
    fire(&mut r, host, true);
    let pick = choices(&r, 1, false)[0];
    choose(&mut r, pick);
    choices(&r, 2, false);
    r.declare_max_count().unwrap();
    let pick = choices(&r, 1, true)[0];
    choose(&mut r, pick);
    let action = zone_action(&r, "EVO6", false);
    choices(&r, 1, true);
    choose(&mut r, action);
    assert_eq!(breeding_id(&r).as_deref(), Some("EVO6"));
    assert_eq!(
        r.game.players[1].battle_area[foe.index as usize]
            .card_sources
            .len(),
        1
    );
}

#[test]
fn bt20_018_security_removal_from_either_stack_selects_one_lowest_dp_digimon() {
    for owner in [0, 1] {
        let mut r = ready(builder(&[]).start());
        let host = r.place_on_field(0, ID, Some(0));
        let other_low = r.place_on_field(1, "LOW-A", Some(0));
        let picked = r.place_on_field(1, "LOW-B", Some(0));
        r.place_on_field(1, "HIGH", Some(0));
        r.place_on_field(1, "TAMER", Some(0));
        remove_security(&mut r, host, owner);
        let actions = choices(&r, 2, false);
        // OppField selections encode the slot within zone_owner's field;
        // only mixed-field selections add an opponent-field offset.
        assert_eq!(r.pending_selection().unwrap().zone_owner, Some(1));
        let chosen = space::encode_attack(0, picked.index as u16);
        assert_eq!(
            actions,
            vec![space::encode_attack(0, other_low.index as u16), chosen]
        );
        choose(&mut r, chosen);
        let remain: Vec<_> = r.game.players[1]
            .battle_area
            .iter()
            .map(|p| p.top_card().card_id(&r.game.card_data))
            .collect();
        assert_eq!(remain, vec!["LOW-A", "HIGH", "TAMER"]);
        remove_security(&mut r, host, 1 - owner);
        assert!(
            r.pending_selection().is_none(),
            "both security timings share one OPT"
        );
        assert_eq!(r.battle_area_size(1), 3);
        main_action(&mut r, 0, space::PASS);
        assert_eq!(r.game.turn_player(), 1);
        remove_security(&mut r, host, owner);
        let pick = choices(&r, 1, false)[0];
        choose(&mut r, pick);
        assert_eq!(r.battle_area_size(1), 2, "OPT resets on next turn");
    }
}

#[test]
fn bt20_018_security_removal_without_digimon_has_no_target_prompt() {
    let mut r = ready(builder(&[]).start());
    let host = r.place_on_field(0, ID, Some(0));
    r.place_on_field(1, "TAMER", Some(0));
    remove_security(&mut r, host, 1);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(1), 1);
}

#[test]
fn bt20_018_inherited_attack_trashes_only_for_exact_ouryuken_once_each_turn() {
    for (top, expected) in [("OURYUKEN", 4), ("NEAR-OURYUKEN", 5), ("BT20-056", 5)] {
        let mut r = ready(builder(&[]).start());
        let host = r.place_stack(0, &[ID, top]);
        for _ in 0..2 {
            r.game
                .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
            r.game.drain_effect_queue();
            assert_eq!(r.security_count(1), expected, "{top}");
            assert!(r.pending_selection().is_none());
        }
        main_action(&mut r, 0, space::PASS);
        r.game
            .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
        r.game.drain_effect_queue();
        assert_eq!(r.security_count(1), if top == "OURYUKEN" { 3 } else { 5 });
    }
}

#[path = "../../support/owned_union_fixture.rs"]
mod owned_union_fixture;
