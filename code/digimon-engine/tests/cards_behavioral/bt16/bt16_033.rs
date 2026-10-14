//! Production Harpymon behavioral matrix. Availability RED was captured before YAML adoption.
//! Official observation d8bb34e7...; image BT16-033; Q2629; rules13-1-7.
use digimon_engine::action::{
    mask::build_action_mask,
    space::{encode_attack, SECURITY_TARGET},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;

fn both_seats(mut scenario: impl FnMut(u8)) {
    let mut failed = Vec::new();
    for seat in 0..=1 {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| scenario(seat))).is_err() {
            failed.push(seat);
        }
    }
    assert!(failed.is_empty(), "failed attacker seats {failed:?}");
}

fn runner(seat: u8, own_security: usize, opponent_security: usize) -> DebugRunner {
    let mut inert = make_test_card("INERT", "Inert security/deck fixture");
    inert.level = Some(3);
    inert.dp = Some(1000);
    inert.play_cost = 3;
    let own = vec!["INERT"; own_security];
    let opponent = vec!["INERT"; opponent_security];
    let mut r = DebugRunner::builder()
        .dsl_card("BT16-033")
        .expect("production BT16-033 must exist in embedded pack")
        .add_card(inert)
        .add_card(make_test_card("BASE", "Armor base fixture"))
        .security(seat, &own)
        .security(1 - seat, &opponent)
        .deck(seat, &["INERT"; 10])
        .deck(1 - seat, &["INERT"; 10])
        .build();
    r.game.turn_player_idx = seat as usize;
    r.game.memory_pair = (seat, 1 - seat);
    r.game.memory = 3;
    r.game.turn_count = 2;
    r.game.current_phase = GamePhase::Main;
    r
}

fn attack(r: &mut DebugRunner, attacker: PermanentHandle) {
    let action = encode_attack(attacker.index as u16, SECURITY_TARGET);
    assert_eq!(
        r.game.current_phase,
        GamePhase::Main,
        "normal attack requires Main phase"
    );
    assert_eq!(
        r.turn_player(),
        attacker.player,
        "normal attack uses turn controller"
    );
    assert!(
        r.pending_selection().is_none(),
        "normal action cannot answer a parked selection"
    );
    assert_eq!(
        build_action_mask(&r.game, attacker.player)[action as usize],
        1.0
    );
    // An explicitly empty defender field cannot supply this attacker action.
    // Do not submit a wrong-seat action to decode_action: Main dispatch uses
    // current turn player and returns (), not an actor-rejection Result.
    if r.battle_area_size(1 - attacker.player) == 0 {
        assert_eq!(
            build_action_mask(&r.game, 1 - attacker.player)[action as usize],
            0.0
        );
    }
    r.game.decode_action(action, attacker.player);
}

#[test]
fn bt16_033_boundary_two_recovers_without_memory_both_seats() {
    both_seats(|seat| {
        let mut r = runner(seat, 2, 1);
        let atk = r.place_on_field(seat, "BT16-033", Some(0));
        attack(&mut r, atk);
        assert_eq!(
            r.memory(),
            3,
            "recovery2->3 cannot enable earlier memory branch"
        );
        assert_eq!(r.security_count(seat), 3);
        assert_eq!(r.deck_size(seat), 9);
        assert_eq!(r.security_count(1 - seat), 0);
        assert!(r.pending_selection().is_none());
    });
}

#[test]
fn bt16_033_boundary_three_gains_without_recovery_both_seats() {
    both_seats(|seat| {
        let mut r = runner(seat, 3, 1);
        let atk = r.place_on_field(seat, "BT16-033", Some(0));
        attack(&mut r, atk);
        assert_eq!(r.memory(), 4);
        assert_eq!(r.security_count(seat), 3);
        assert_eq!(r.deck_size(seat), 10);
    });
}

#[test]
fn bt16_033_nonattacking_copy_does_not_gain_both_seats() {
    both_seats(|seat| {
        let mut r = runner(seat, 5, 1);
        let atk = r.place_on_field(seat, "BT16-033", Some(0));
        r.place_on_field(seat, "BT16-033", Some(0));
        attack(&mut r, atk);
        assert_eq!(r.memory(), 4, "only original checking source gains once");
        assert_eq!(r.deck_size(seat), 10);
    });
}

#[test]
fn bt16_033_distinct_checks_recovery_then_memory_not_opt_both_seats() {
    both_seats(|seat| {
        let mut r = runner(seat, 2, 2);
        let atk = r.place_on_field(seat, "BT16-033", Some(0));
        // Explicit SYNTHETIC check-count fixture; production Harpymon is unchanged.
        let index = r
            .game
            .card_data
            .iter()
            .position(|d| d.card_id == "BT16-033")
            .unwrap();
        r.game.card_data[index]
            .keywords
            .push(Keyword::SecurityAttackPlus(1));
        attack(&mut r, atk);
        assert_eq!(r.memory(), 4);
        assert_eq!(r.security_count(seat), 3);
        assert_eq!(r.deck_size(seat), 9);
        assert_eq!(r.security_count(1 - seat), 0);
    });
}

use digimon_engine::action::space::{encode_digivolve, PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_source::CardHandle;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::{AttackTargetRestriction, EffectContext};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, ModifierType};
use digimon_engine::events::GameEvent;
use digimon_engine::modifiers::{ModifierEntry, ModifierPayload, PlayerModifierEntry};
use digimon_engine::selection::SelectionKind;
use std::sync::Arc;

fn assert_prompt_mask(r: &DebugRunner, actor: u8) {
    let s = r.pending_selection().expect("actual pending choice");
    assert_eq!(s.selecting_player, actor);
    let mut expected = s.valid_action_ids.clone();
    if s.is_optional && !expected.contains(&PASS) {
        expected.push(PASS);
    }
    expected.sort();
    for seat in 0..=1 {
        let actual: Vec<u16> = build_action_mask(&r.game, seat)
            .iter()
            .enumerate()
            .filter_map(|(id, &v)| (v > 0.5).then_some(id as u16))
            .collect();
        assert_eq!(
            actual,
            if seat == actor {
                expected.clone()
            } else {
                vec![]
            }
        );
    }
}

fn choose(r: &mut DebugRunner, actor: u8, id: u16) {
    assert_prompt_mask(r, actor);
    assert_eq!(build_action_mask(&r.game, actor)[id as usize], 1.0);
    assert!(r.execute_action(1 - actor, id).is_err());
    r.execute_action(actor, id).unwrap();
}

fn choose_source(r: &mut DebugRunner, source: CardHandle) {
    let s = r
        .pending_selection()
        .expect("source-labelled trigger order");
    assert_eq!(s.kind, SelectionKind::TriggerOrder);
    let actor = s.selecting_player;
    let id = s
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|e| e.source_card == Some(source))
        .expect("requested physical source offered")
        .action_id;
    choose(r, actor, id);
}

fn native_action(r: &mut DebugRunner, actor: u8, id: u16) {
    assert!(r.pending_selection().is_none());
    if r.game.current_phase != GamePhase::Mulligan {
        assert_eq!(r.turn_player(), actor);
    }
    assert_eq!(
        build_action_mask(&r.game, actor)[id as usize],
        1.0,
        "actor {actor} action {id} phase {:?}",
        r.game.current_phase
    );
    r.game.decode_action(id, actor);
}

fn reveal_positions(r: &DebugRunner) -> Vec<usize> {
    r.game
        .events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| matches!(e, GameEvent::SecurityReveal { .. }).then_some(i))
        .collect()
}
fn harp_memory_positions(r: &DebugRunner) -> Vec<usize> {
    r.game
        .events
        .iter()
        .enumerate()
        .filter_map(|(i, e)| match e {
            GameEvent::MemoryChange {
                source_card_id: Some(id),
                ..
            } if id == "BT16-033" => Some(i),
            _ => None,
        })
        .collect()
}
fn plus_one(r: &mut DebugRunner) {
    // Synthetic check-count modifier only; production effect remains unchanged.
    r.game
        .card_data
        .iter_mut()
        .find(|d| d.card_id == "BT16-033")
        .unwrap()
        .keywords
        .push(Keyword::SecurityAttackPlus(1));
}

#[test]
fn bt16_033_metadata_and_ordinary_named_routes_both_seats() {
    both_seats(|seat| {
        for (name, level, color, alias, costs) in [
            ("Other", 3, CardColor::Yellow, false, vec![3]),
            ("Other", 3, CardColor::Red, false, vec![3]),
            ("Other", 4, CardColor::Yellow, false, vec![]),
            ("Other", 3, CardColor::Blue, false, vec![]),
            ("Hawkmon", 3, CardColor::Yellow, false, vec![2, 3]),
            ("Hawkmon", 5, CardColor::Blue, false, vec![2]),
            ("Alias", 5, CardColor::Blue, true, vec![2]),
            ("Granted", 5, CardColor::Blue, false, vec![2]),
            ("GrantedSubstring", 5, CardColor::Blue, false, vec![]),
            ("Hawkmon X", 5, CardColor::Blue, false, vec![]),
            ("Other", 5, CardColor::Blue, false, vec![]),
        ] {
            for cost in if costs.is_empty() {
                vec![-1]
            } else {
                costs.clone()
            } {
                let mut base = make_test_card("ROUTE-BASE", name);
                base.level = Some(level);
                base.colors = vec![color];
                if alias {
                    base.also_treated_as.push("Hawkmon".into());
                }
                let mut r = DebugRunner::builder()
                    .dsl_card("BT16-033")
                    .unwrap()
                    .add_card(base)
                    .hand(seat, &["BT16-033"])
                    .deck(seat, &["ROUTE-BASE"; 10])
                    .build();
                r.game.turn_player_idx = seat as usize;
                r.game.memory_pair = (seat, 1 - seat);
                r.game.memory = 8;
                r.game.turn_count = 2;
                r.game.current_phase = GamePhase::Main;
                let data = r
                    .game
                    .card_data
                    .iter()
                    .find(|d| d.card_id == "BT16-033")
                    .unwrap();
                assert_eq!(data.card_name, "Harpymon");
                assert_eq!(data.card_kind, CardKind::Digimon);
                assert_eq!(
                    (data.level, data.dp, data.play_cost),
                    (Some(4), Some(5000), 5)
                );
                assert_eq!(data.colors, vec![CardColor::Yellow, CardColor::Red]);
                for t in ["Armor Form", "Free", "Mythical Beast"] {
                    assert!(data.traits.iter().any(|x| x == t));
                }
                assert!(data.inherited_text.is_empty() && data.security_text.is_empty());
                let target = r.place_on_field(seat, "ROUTE-BASE", Some(0));
                if name.starts_with("Granted") {
                    // Synthetic runtime name grant tests effective-name matching.
                    r.game.modifiers.add(
                        target,
                        ModifierEntry::simple(
                            ModifierType::ChangeBaseCardName,
                            0,
                            Expiry::Permanent,
                            seat,
                        )
                        .with_payload(ModifierPayload::Name {
                            value: if name == "Granted" {
                                "Hawkmon"
                            } else {
                                "Hawkmon X"
                            }
                            .into(),
                            base: true,
                        }),
                    );
                }
                let id = encode_digivolve(0, target.index as u16);
                assert_eq!(
                    build_action_mask(&r.game, seat)[id as usize],
                    if costs.is_empty() { 0.0 } else { 1.0 },
                    "{name}/{level}/{color:?}"
                );
                if costs.is_empty() {
                    continue;
                }
                native_action(&mut r, seat, id);
                if costs.len() > 1 {
                    let s = r.pending_selection().expect("both priced routes visible");
                    let entries = s.effect_choices.as_ref().unwrap();
                    assert_eq!(entries.len(), costs.len());
                    for c in &costs {
                        assert!(entries
                            .iter()
                            .any(|e| e.label == format!("Digivolve for cost {c}")));
                    }
                    let action = entries
                        .iter()
                        .find(|e| e.label == format!("Digivolve for cost {cost}"))
                        .unwrap()
                        .action_id;
                    choose(&mut r, seat, action);
                }
                assert_eq!(r.memory(), 8 - cost);
                assert_eq!(
                    r.game.players[seat as usize].battle_area[0]
                        .top_card()
                        .card_id(&r.game.card_data),
                    "BT16-033"
                );
                assert!(r.game.has_keyword(target, Keyword::ArmorPurge));
            }
        }
    });
}

#[test]
fn bt16_033_all_thresholds_and_face_up_down_once_both_seats() {
    both_seats(|seat| {
        for n in 0..=5 {
            for face_up in [false, true] {
                let mut r = runner(seat, n, 1);
                let atk = r.place_on_field(seat, "BT16-033", Some(0));
                if face_up {
                    let i = r.game.players[(1 - seat) as usize].security[0].card_index;
                    r.game.players[(1 - seat) as usize]
                        .face_up_security
                        .insert(i);
                }
                attack(&mut r, atk);
                assert_eq!(r.memory(), if n >= 3 { 4 } else { 3 });
                assert_eq!(r.security_count(seat), if n >= 3 { n } else { n + 1 });
                assert_eq!(r.deck_size(seat), if n >= 3 { 10 } else { 9 });
                assert_eq!(harp_memory_positions(&r).len(), usize::from(n >= 3));
                assert_eq!(reveal_positions(&r).len(), 1);
                assert!(r.pending_selection().is_none());
            }
        }
    });
}

#[test]
fn bt16_033_multicheck_timeline_and_second_attack_no_opt_both_seats() {
    both_seats(|seat| {
        let mut r = runner(seat, 2, 4);
        let atk = r.place_on_field(seat, "BT16-033", Some(0));
        plus_one(&mut r);
        attack(&mut r, atk);
        let reveals = reveal_positions(&r);
        let gains = harp_memory_positions(&r);
        assert_eq!((reveals.len(), gains.len()), (2, 1));
        assert!(gains[0] > reveals[1]);
        assert_eq!(
            (r.memory(), r.security_count(seat), r.deck_size(seat)),
            (4, 3, 9)
        );
        // Explicit unsuspend fixture models a separate legal attack in the same turn.
        r.game.players[seat as usize].battle_area[0].is_suspended = false;
        attack(&mut r, atk);
        assert_eq!(
            (r.memory(), r.security_count(seat), r.deck_size(seat)),
            (6, 3, 9)
        );
        assert_eq!(
            (reveal_positions(&r).len(), harp_memory_positions(&r).len()),
            (4, 3)
        );
    });
}

#[test]
fn bt16_033_scope_buried_breeding_opposing_and_nonattacker_both_seats() {
    both_seats(|seat| {
        let mut r = runner(seat, 5, 2);
        let atk = r.place_on_field(seat, "BT16-033", Some(0));
        r.place_on_field(seat, "BT16-033", Some(0));
        r.place_stack(seat, &["BT16-033", "INERT"]);
        r.place_in_breeding(seat, "BT16-033");
        r.place_on_field(1 - seat, "BT16-033", Some(0));
        attack(&mut r, atk);
        assert_eq!(r.memory(), 4);
        assert_eq!(harp_memory_positions(&r).len(), 1);
        let buried = PermanentHandle {
            player: seat,
            index: 2,
        };
        r.game.players[seat as usize].battle_area[2].turn_played = 0;
        attack(&mut r, buried);
        assert_eq!(r.memory(), 4);
        assert_eq!(harp_memory_positions(&r).len(), 1);
    });
}

#[test]
fn bt16_033_noncombat_trash_digimon_target_block_and_no_reveal_both_seats() {
    both_seats(|seat| {
        for mode in ["trash", "digimon", "block", "zero_checks"] {
            let mut r = runner(seat, 5, 2);
            let atk = r.place_on_field(seat, "BT16-033", Some(0));
            if mode == "trash" {
                let source = r.top_card(atk);
                let mut ctx = EffectContext::new(&mut r.game, source, Some(atk), seat);
                assert!(ctx.trash_top_security(seat));
                assert!(ctx.trash_top_security(1 - seat));
            } else if mode == "digimon" {
                let def = r.place_on_field(1 - seat, "INERT", Some(0));
                r.game.players[(1 - seat) as usize].battle_area[def.index as usize].is_suspended =
                    true;
                native_action(
                    &mut r,
                    seat,
                    encode_attack(atk.index as u16, def.index as u16),
                );
            } else if mode == "block" {
                r.game
                    .card_data
                    .iter_mut()
                    .find(|d| d.card_id == "INERT")
                    .unwrap()
                    .keywords
                    .push(Keyword::Blocker);
                r.place_on_field(1 - seat, "INERT", Some(0));
                attack(&mut r, atk);
                let s = r.pending_selection().expect("defender can block");
                assert_eq!(s.valid_action_ids.len(), 1, "exactly one eligible blocker");
                let id = s.valid_action_ids[0];
                choose(&mut r, 1 - seat, id);
            } else {
                r.game
                    .card_data
                    .iter_mut()
                    .find(|d| d.card_id == "BT16-033")
                    .unwrap()
                    .keywords
                    .push(Keyword::SecurityAttackMinus(1));
                attack(&mut r, atk);
            }
            assert_eq!(r.memory(), 3, "{mode}");
            assert!(harp_memory_positions(&r).is_empty());
            assert!(reveal_positions(&r).is_empty(), "{mode}");
        }
    });
}

#[test]
fn bt16_033_opponents_turn_effect_created_attack_does_not_trigger_both_seats() {
    both_seats(|seat| {
        let mut r = runner(1 - seat, 5, 2);
        let atk = r.place_on_field(seat, "BT16-033", Some(0));
        let src = r.top_card(atk);
        {
            let mut ctx = EffectContext::new(&mut r.game, src, Some(atk), seat);
            ctx.may_attack_now(
                atk,
                AttackTargetRestriction::PlayerOnly,
                false,
                "synthetic opponent-turn attack",
            )
            .unwrap();
        }
        choose(
            &mut r,
            seat,
            encode_attack(atk.index as u16, SECURITY_TARGET),
        );
        assert_eq!(reveal_positions(&r).len(), 1);
        assert!(harp_memory_positions(&r).is_empty());
        assert_eq!(r.memory(), 3);
    });
}

#[test]
fn bt16_033_memory_precedes_losing_security_battle_both_seats() {
    both_seats(|seat| {
        let mut r = runner(seat, 3, 1);
        r.force_base_dp("INERT", 6000);
        let atk = r.place_on_field(seat, "BT16-033", Some(0));
        attack(&mut r, atk);
        let gain = harp_memory_positions(&r);
        assert_eq!(gain.len(), 1);
        let deleted = r
            .game
            .events
            .iter()
            .position(|e| matches!(e,GameEvent::Trash{card_id,..} if card_id=="BT16-033"))
            .expect("attacker deleted");
        assert!(reveal_positions(&r)[0] < gain[0] && gain[0] < deleted);
        assert_eq!(r.memory(), 4);
        assert_eq!(r.battle_area_size(seat), 0);
    });
}

fn card_choice(r: &mut DebugRunner, actor: u8, id: &str) {
    // Shared replay resolver maps the observed card identity into the current
    // phase-specific mask. No hand/union/trigger ordinal is assumed.
    let row: digimon_engine::dcgo_recording::SelectionRow = serde_json::from_value(
        serde_json::json!({"step":0,"actor":actor,"prompt":"card identity", "card_ids":[id]}),
    )
    .unwrap();
    let action = digimon_engine::runners::selection_resolve::resolve_next(&r.game, &row, 0)
        .unwrap()
        .expect("identity resolves to an actual candidate");
    choose(r, actor, action);
}

fn accept_outer_if_present(r: &mut DebugRunner, actor: u8, id: &str) {
    if r.pending_kind() == Some(SelectionKind::Replacement) {
        let s = r.pending_selection().unwrap();
        assert_eq!(r.game.card(s.source_card).card_id, id);
        assert!(
            s.prompt.starts_with("You may activate"),
            "only an observed outer activation gate"
        );
        assert!(s.is_optional);
        choose(r, actor, REPLACEMENT_ACCEPT);
    }
}

fn q6813_setup(seat: u8) -> DebugRunner {
    use std::collections::HashMap;
    let decks = [Q6813_DECK_0, Q6813_DECK_1];
    let stacks = [Q6813_STACK_0, Q6813_STACK_1];
    let mut b = DebugRunner::builder();
    let mut ids = decks
        .iter()
        .flat_map(|d| d.iter().copied())
        .collect::<Vec<_>>();
    ids.push("EX12-004");
    ids.sort();
    ids.dedup();
    for id in ids {
        b = b
            .dsl_card(id)
            .unwrap_or_else(|e| panic!("Q6813 production {id}: {e}"));
    }
    let template = b.build();
    let data: HashMap<_, _> = template
        .game
        .card_data
        .iter()
        .cloned()
        .map(|c| (c.card_id.clone(), c))
        .collect();
    let mut ordered = vec![Vec::<String>::new(); 2];
    for logical in 0..2 {
        let mut rest = decks[logical].to_vec();
        for id in stacks[logical] {
            let pos = rest.iter().position(|c| c == id).unwrap();
            rest.remove(pos);
        }
        let draw_order: Vec<String> = stacks[logical]
            .iter()
            .chain(rest.iter())
            .map(|s| s.to_string())
            .collect();
        assert_eq!(draw_order.len(), 50);
        let physical = if logical == 0 { seat } else { 1 - seat };
        ordered[physical as usize] = draw_order.into_iter().rev().collect();
        ordered[physical as usize].extend(std::iter::repeat_n("EX12-004".to_string(), 4));
    }
    let game = digimon_engine::game::Game::new_with_ordered_decks(
        &ordered,
        &data,
        digimon_engine::rules::Rules::standard(),
        Some(424242),
        seat,
    )
    .unwrap();
    let mut r = DebugRunner::wrap(game);
    for logical in 0..2 {
        let physical = if logical == 0 { seat } else { 1 - seat };
        let hand: Vec<_> = r.game.players[physical as usize]
            .hand
            .iter()
            .map(|c| c.card_id(&r.game.card_data))
            .collect();
        assert_eq!(
            hand,
            &stacks[logical][..5],
            "actual dealt hand, seat {physical}"
        );
    }
    while let Some(actor) = r.mulligan_current() {
        native_action(&mut r, actor, 0);
    }
    for logical in 0..2 {
        let physical = if logical == 0 { seat } else { 1 - seat };
        let security: Vec<_> = r.game.players[physical as usize]
            .security
            .iter()
            .map(|c| c.card_id(&r.game.card_data))
            .collect();
        assert_eq!(
            security,
            &stacks[logical][5..10],
            "native security deal guard"
        );
    }
    println!("Q6813 seat={seat}: actual native initial hands/security match R3 guard");
    let pass_phase = |r: &mut DebugRunner, actor, phase| {
        assert_eq!(r.game.current_phase, phase);
        native_action(r, actor, PASS);
    };
    let play = |r: &mut DebugRunner, actor: u8, id: &str| {
        assert_eq!(r.game.current_phase, GamePhase::Main);
        let slot = r.game.players[actor as usize]
            .hand
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == id)
            .unwrap();
        native_action(r, actor, slot as u16);
    };
    pass_phase(&mut r, seat, GamePhase::Breeding);
    pass_phase(&mut r, seat, GamePhase::Main);
    pass_phase(&mut r, 1 - seat, GamePhase::Breeding);
    play(&mut r, 1 - seat, "BT15-037");
    pass_phase(&mut r, seat, GamePhase::Breeding);
    play(&mut r, seat, "EX12-046");
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    // OppField candidates use the field-target encoding, not the hand/union
    // card-pick replay payload. Resolve the public target identity first.
    let gatomon_index = r.game.players[(1 - seat) as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "BT15-037")
        .unwrap();
    choose(&mut r, seat, encode_attack(0, gatomon_index as u16));
    pass_phase(&mut r, 1 - seat, GamePhase::Breeding);
    pass_phase(&mut r, 1 - seat, GamePhase::Main);
    pass_phase(&mut r, seat, GamePhase::Breeding);
    play(&mut r, seat, "BT16-033");
    pass_phase(&mut r, 1 - seat, GamePhase::Breeding);
    pass_phase(&mut r, 1 - seat, GamePhase::Main);
    pass_phase(&mut r, seat, GamePhase::Breeding);
    assert_eq!(r.memory(), 3);
    assert_eq!(r.security_count(seat), 5);
    assert_eq!(r.security_count(1 - seat), 5);
    assert_eq!(
        r.effective_dp(PermanentHandle {
            player: 1 - seat,
            index: 0
        }),
        Some(4000),
        "temporary Shishimamon debuff expired"
    );
    assert_eq!(field_ids(&r, seat), ["EX12-046", "BT16-033"]);
    assert_eq!(field_ids(&r, 1 - seat), ["BT15-037"]);
    r
}

fn field_ids(r: &DebugRunner, seat: u8) -> Vec<String> {
    r.game.players[seat as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn assert_q6813_order_then_choose_harp(r: &mut DebugRunner, seat: u8) {
    let s = r
        .pending_selection()
        .expect("turn-player simultaneous order");
    assert_eq!(s.kind, SelectionKind::TriggerOrder);
    assert!(!s.is_optional);
    let entries = s.effect_choices.as_ref().unwrap();
    assert_eq!(entries.len(), 2);
    let actual: Vec<_> = entries
        .iter()
        .map(|e| {
            r.game
                .card(e.source_card.expect("physical source"))
                .card_id
                .clone()
        })
        .collect();
    assert!(actual.iter().any(|id| id == "BT16-033") && actual.iter().any(|id| id == "EX12-046"));
    assert!(!actual.iter().any(|id| id == "BT15-037"));
    assert_prompt_mask(r, seat);
    assert!(
        r.execute_action(seat, PASS).is_err(),
        "mandatory Harpymon prohibits decline-all"
    );
    println!(
        "Q6813 seat={seat}: mixed TriggerOrder sources={actual:?}, memory={}",
        r.memory()
    );
    let src = r.top_card(PermanentHandle {
        player: seat,
        index: 1,
    });
    choose_source(r, src);
    accept_outer_if_present(r, seat, "EX12-046");
    let s = r
        .pending_selection()
        .expect("actual Shishimamon hand choice");
    assert_eq!(s.kind, SelectionKind::Hand);
    assert_eq!(r.game.card(s.source_card).card_id, "EX12-046");
    assert!(s.is_optional);
    assert_prompt_mask(r, seat);
    assert_eq!(r.memory(), 4);
    assert_eq!(r.security_count(seat), 5);
    assert_eq!(r.security_count(1 - seat), 4);
    assert_eq!(field_ids(r, seat), ["EX12-046", "BT16-033"]);
    assert!(!r.game.players[seat as usize].battle_area[0].is_suspended);
    assert!(r.game.players[seat as usize].battle_area[1].is_suspended);
}

#[test]
fn bt16_033_production_q6813_actual_deal_actor_order_optional_and_clone_both_seats() {
    both_seats(|seat| {
        let mut r = q6813_setup(seat);
        let attacker = PermanentHandle {
            player: seat,
            index: 1,
        };
        attack(&mut r, attacker);
        assert_eq!(reveal_positions(&r).len(), 1);
        let reveals: Vec<_> = r
            .game
            .events
            .iter()
            .filter_map(|e| match e {
                GameEvent::SecurityReveal {
                    defender, card_id, ..
                } => Some((*defender, card_id.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(reveals, vec![(1 - seat, "EX12-074")]);
        assert_prompt_mask(&r, 1 - seat);
        assert_eq!(r.memory(), 3);
        assert_eq!(field_ids(&r, 1 - seat), ["BT15-037"]);
        assert_eq!(r.security_count(1 - seat), 4);
        assert_eq!(
            r.game
                .card(r.pending_selection().unwrap().source_card)
                .card_id,
            "EX12-074"
        );
        println!(
            "Q6813 seat={seat}: Security first kind={:?}, actor={}, memory3, Gasamon absent",
            r.pending_kind(),
            1 - seat
        );
        let mut security_clone = DebugRunner::wrap(r.game.clone());
        let mut security_decline = DebugRunner::wrap(r.game.clone());
        choose(&mut security_decline, 1 - seat, PASS);
        assert_q6813_order_then_choose_harp(&mut security_decline, seat);
        choose(&mut security_decline, seat, PASS);
        assert_eq!(security_decline.memory(), 3);
        assert_eq!(field_ids(&security_decline, 1 - seat), ["BT15-037"]);
        for g in [&mut r, &mut security_clone] {
            accept_outer_if_present(g, 1 - seat, "EX12-074");
            assert!(matches!(
                g.pending_kind(),
                Some(SelectionKind::UnionZone { .. })
            ));
            assert_eq!(g.memory(), 3);
            card_choice(g, 1 - seat, "EX12-020");
            assert_eq!(field_ids(g, 1 - seat), ["BT15-037", "EX12-020"]);
            assert_q6813_order_then_choose_harp(g, seat);
            println!("Q6813 seat={seat}: Shishimamon hand prompt memory4, suspended Harpymon, Gasamon present");
            let mut hand_clone = DebugRunner::wrap(g.game.clone());
            let mut decline = DebugRunner::wrap(g.game.clone());
            choose(&mut decline, seat, PASS);
            assert_eq!(decline.memory(), 3);
            assert_eq!(field_ids(&decline, seat), ["EX12-046", "BT16-033"]);
            assert!(decline.pending_selection().is_none());
            for h in [g, &mut hand_clone] {
                assert!(
                    h.game.players[seat as usize].trash.is_empty(),
                    "fixed Q6813 fixture must have empty actor trash before Kaguyamon"
                );
                card_choice(h, seat, "EX12-065");
                assert!(
                    h.pending_selection().is_none(),
                    "empty-trash Q6813 fixture must not create a derived Kaguyamon prompt"
                );
                println!("Q6813 seat={seat}: no derived Kaguyamon prompt with empty trash");
                assert_eq!(field_ids(h, seat), ["EX12-065", "BT16-033"]);
                assert_eq!(
                    h.game.players[seat as usize].battle_area[0].card_sources[0]
                        .card_id(&h.game.card_data),
                    "EX12-046"
                );
                assert_eq!(h.memory(), 2);
                assert!(h.pending_selection().is_none());
                let source_events: Vec<_> = h
                    .game
                    .events
                    .iter()
                    .filter_map(|e| match e {
                        GameEvent::MemoryChange {
                            source_card_id: Some(id),
                            ..
                        } if id == "BT16-033" || id == "BT15-037" => Some(id.as_str()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(source_events, vec!["BT16-033", "BT15-037"]);
            }
        }
    });
}

struct RemoveAttacker;
impl CardEffect for RemoveAttacker {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .process(|ctx| ctx.delete_permanent(ctx.attacker().unwrap()))
            .build()]
    }
}

#[test]
fn bt16_033_q2629_removal_and_armor_purge_clone_accept_decline_both_seats() {
    both_seats(|seat| {
        for own_security in [2, 3] {
            for stacked in [false, true] {
                for accept in [false, true] {
                    let mut r = runner(seat, own_security, 2);
                    r.register_effect("INERT", Arc::new(RemoveAttacker));
                    let atk = if stacked {
                        r.place_stack(seat, &["BASE", "BT16-033"])
                    } else {
                        r.place_on_field(seat, "BT16-033", Some(0))
                    };
                    r.game.players[seat as usize].battle_area[0].turn_played = 0;
                    let sibling = r.place_on_field(seat, "BT16-033", Some(0));
                    let sibling_card = r.top_card(sibling);
                    plus_one(&mut r);
                    attack(&mut r, atk);
                    if stacked {
                        assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
                        assert!(r.pending_is_optional());
                        assert_prompt_mask(&r, seat);
                        let mut clone = DebugRunner::wrap(r.game.clone());
                        for g in [&mut r, &mut clone] {
                            choose(g, seat, if accept { REPLACEMENT_ACCEPT } else { PASS });
                            assert_eq!(g.memory(), 3);
                            assert_eq!(g.security_count(seat), own_security);
                            assert_eq!(g.deck_size(seat), 10);
                            assert_eq!(g.security_count(1 - seat), 1);
                            assert!(harp_memory_positions(g).is_empty());
                            assert_eq!(reveal_positions(g).len(), 1);
                            assert!(g.pending_selection().is_none());
                            assert!(g.game.players[seat as usize]
                                .battle_area
                                .iter()
                                .any(|p| p.top_card().handle() == sibling_card));
                            // The promoted 2000 DP base survives the inert 1000 security battle.
                            assert_eq!(g.battle_area_size(seat), if accept { 2 } else { 1 });
                            if accept {
                                assert_eq!(
                                    g.game.players[seat as usize].battle_area[0]
                                        .top_card()
                                        .card_id(&g.game.card_data),
                                    "BASE"
                                );
                            }
                        }
                    } else {
                        assert_eq!(r.memory(), 3);
                        assert_eq!(r.security_count(seat), own_security);
                        assert_eq!(r.deck_size(seat), 10);
                        assert_eq!(r.security_count(1 - seat), 1);
                        assert_eq!(
                            r.top_card(PermanentHandle {
                                player: seat,
                                index: 0
                            }),
                            sibling_card
                        );
                        assert!(harp_memory_positions(&r).is_empty());
                        assert_eq!(reveal_positions(&r).len(), 1);
                    }
                }
            }
        }
    });
}

struct ShiftSecurity {
    timing: EffectTiming,
    recover: bool,
    player: u8,
}
impl CardEffect for ShiftSecurity {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let (recover, player) = (self.recover, self.player);
        let builder = if self.timing == EffectTiming::SecuritySkill {
            Effect::security(card)
        } else {
            Effect::on_security_check(card)
        };
        vec![builder
            .process(move |ctx| {
                if recover {
                    ctx.recover_from_deck(player, 1);
                } else {
                    ctx.trash_top_security(player);
                }
            })
            .build()]
    }
}

#[test]
fn bt16_033_live_security_and_sibling_processing_thresholds_both_seats() {
    both_seats(|seat| {
        for security_effect in [true, false] {
            for recover in [true, false] {
                let mut r = runner(seat, if recover { 2 } else { 3 }, 1);
                r.register_effect(
                    "INERT",
                    Arc::new(ShiftSecurity {
                        timing: if security_effect {
                            EffectTiming::SecuritySkill
                        } else {
                            EffectTiming::OnSecurityCheck
                        },
                        recover,
                        player: seat,
                    }),
                );
                let atk = r.place_on_field(seat, "BT16-033", Some(0));
                let src = r.top_card(atk);
                if !security_effect {
                    let helper = r.place_on_field(seat, "INERT", Some(0));
                    let helper_source = r.top_card(helper);
                    attack(&mut r, atk);
                    assert_eq!(r.security_count(seat), if recover { 2 } else { 3 });
                    choose_source(&mut r, helper_source);
                    if r.pending_selection().is_some() {
                        choose_source(&mut r, src);
                    }
                } else {
                    attack(&mut r, atk);
                }
                assert_eq!(r.memory(), if recover { 4 } else { 3 });
                assert_eq!(r.security_count(seat), 3);
                assert_eq!(r.deck_size(seat), 9);
                assert_eq!(harp_memory_positions(&r).len(), usize::from(recover));
            }
        }
    });
}

#[test]
fn bt16_033_empty_deck_memory_and_security_prohibitions_both_seats() {
    both_seats(|seat| {
        for mode in ["empty", "memory", "memory_except_tamer", "security"] {
            let mut r = runner(seat, if mode.starts_with("memory") { 3 } else { 2 }, 1);
            let atk = r.place_on_field(seat, "BT16-033", Some(0));
            if mode == "empty" {
                r.game.players[seat as usize].deck.clear();
            } else {
                r.game.modifiers.add_player_modifier(
                    seat,
                    PlayerModifierEntry::simple(
                        match mode {
                            "memory" => ModifierType::CannotGainMemoryByEffect,
                            "memory_except_tamer" => ModifierType::CannotGainMemoryExceptFromTamers,
                            _ => ModifierType::CannotAddSecurityByEffect,
                        },
                        0,
                        Expiry::Permanent,
                        None,
                        1 - seat,
                    ),
                );
            }
            attack(&mut r, atk);
            assert_eq!(r.memory(), 3);
            assert_eq!(
                r.security_count(seat),
                if mode.starts_with("memory") { 3 } else { 2 }
            );
            assert_eq!(r.deck_size(seat), if mode == "empty" { 0 } else { 10 });
            assert!(harp_memory_positions(&r).is_empty());
            assert!(r.pending_selection().is_none());
        }
    });
}

const Q6813_DECK_0: &[&str] = &[
    "EX12-009", "EX12-009", "EX12-009", "EX12-009", "EX12-011", "EX12-011", "EX12-011", "EX12-011",
    "EX12-020", "EX12-020", "EX12-020", "EX12-020", "EX12-026", "EX12-026", "EX12-026", "EX12-031",
    "EX12-031", "EX12-031", "EX12-031", "EX12-036", "EX12-036", "EX12-046", "EX12-046", "EX12-046",
    "EX12-046", "EX12-047", "EX12-047", "EX12-061", "EX12-061", "EX12-061", "EX12-062", "EX12-062",
    "EX12-063", "EX12-063", "EX12-065", "EX12-065", "EX12-070", "EX12-070", "EX12-070", "EX12-070",
    "EX12-074", "EX12-074", "EX12-074", "EX12-074", "EX12-075", "EX12-075", "EX12-075", "EX12-076",
    "EX12-076", "BT16-033",
];

const Q6813_STACK_0: &[&str] = &[
    "EX12-046", "EX12-065", "BT16-033", "EX12-070", "EX12-075", "EX12-009", "EX12-011", "EX12-062",
    "EX12-063", "EX12-036", "EX12-074", "EX12-075", "EX12-070", "EX12-074",
];

const Q6813_DECK_1: &[&str] = &[
    "EX12-009", "EX12-009", "EX12-009", "EX12-009", "EX12-011", "EX12-011", "EX12-011", "EX12-011",
    "EX12-020", "EX12-020", "EX12-020", "EX12-020", "EX12-026", "EX12-026", "EX12-026", "EX12-031",
    "EX12-031", "EX12-031", "EX12-031", "EX12-036", "EX12-036", "EX12-046", "EX12-046", "EX12-046",
    "EX12-046", "EX12-047", "EX12-047", "EX12-061", "EX12-061", "EX12-061", "EX12-062", "EX12-062",
    "EX12-063", "EX12-063", "EX12-065", "EX12-065", "EX12-070", "EX12-070", "EX12-070", "EX12-070",
    "EX12-074", "EX12-074", "EX12-074", "EX12-074", "EX12-075", "EX12-075", "EX12-075", "EX12-076",
    "EX12-076", "BT15-037",
];

const Q6813_STACK_1: &[&str] = &[
    "EX12-020", "EX12-036", "EX12-031", "BT15-037", "EX12-046", "EX12-011", "EX12-009", "EX12-020",
    "EX12-026", "EX12-074", "EX12-031", "EX12-036", "EX12-063", "EX12-031",
];
