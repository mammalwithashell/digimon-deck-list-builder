//! Focused follow-up to the frozen-13 audit; not whole-pool certification.
//!
//! Printed EX12-034 and ST23-04 images were checked against production data.
//! CR 4.3 15-16-15-1 (general_rule.pdf p.35) scopes [End of Attack] to an
//! attack performed using the card with that effect. DCGO ST23_04.cs calls
//! CanTriggerOnEndAttack -> CanTriggerOnAttack -> attacking stack contains card.
//! EX12_034.cs selects an own Digimon's leave and plays from hand or THIS
//! Erlangmon's sources for free; its effect does not prevent the original leave.

use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, PASS},
    },
    card_data::{CardData, EvoCost},
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, GamePhase},
    permanent::PermanentHandle,
    selection::SelectionKind,
};

fn digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = 7;
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn setup(card_id: &str, player: u8) -> DebugRunner {
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/cards.json"
    ))
    .unwrap();
    let mut data = CardData::load_from_str(&raw).unwrap();
    let mut tamer = make_test_card("TAMER", "Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.colors = vec![CardColor::Yellow];
    tamer.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".into();
    let mut chronicle = digimon("CHRON-L6", 6, 11000, &["Chronicle"]);
    chronicle.evo_costs = vec![EvoCost {
        card_color: CardColor::Yellow as u8,
        level: 5,
        memory_cost: 3,
    }];
    let mut runner = DebugRunner::builder()
        .dsl_card(card_id)
        .unwrap()
        .add_card(data.remove(card_id).unwrap())
        .add_card(digimon("SW-A", 5, 7000, &["SW"]))
        .add_card(digimon("SW-B", 4, 5000, &["SW"]))
        .add_card(digimon("SW-TOO-HIGH", 6, 11000, &["SW"]))
        .add_card(digimon("PLAIN", 5, 9000, &[]))
        .add_card(digimon("FILL", 3, 1000, &[]))
        .add_card(digimon("HOST", 6, 12000, &["Glowing Dawn"]))
        .add_card(digimon("TB-A", 4, 5000, &["TB"]))
        .add_card(chronicle)
        .add_card(tamer)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(8)
        .start();
    runner.set_first_player(player);
    runner.set_phase(GamePhase::Main);
    runner
}

fn candidate_board(card: &str, player: u8, turn_player: u8) -> (DebugRunner, PermanentHandle) {
    let mut r = setup(card, turn_player);
    let carrier = match card {
        "BT25-041" | "EX12-046" => r.place_stack(player, &[card, "HOST"]),
        "P-153" => r.place_stack(player, &["HOST", card]),
        _ => r.place_on_field(player, card, Some(0)),
    };
    match card {
        "BT25-041" => {
            let tamer = r.place_stack(player, &["FILL", "TAMER"]);
            r.game.players[player as usize].battle_area[tamer.index as usize].card_sources[0]
                .face_down = true;
        }
        "EX12-046" => {
            r.add_to_hand(player, "TB-A");
        }
        "AD1-015" => {
            r.add_to_hand(player, "TAMER");
        }
        "EX11-012" => {
            r.place_on_field(1 - player, "PLAIN", Some(0));
        }
        "EX13-057" => {
            r.add_to_hand(player, "CHRON-L6");
        }
        "P-153" => {}
        _ => panic!("unknown candidate {card}"),
    }
    (r, carrier)
}

fn candidate_payload_happened(r: &DebugRunner, card: &str, player: u8) -> bool {
    match card {
        "BT25-041" => !r.game.players[player as usize].battle_area[0].is_suspended,
        "EX12-046" => field_has(r, player, "TB-A"),
        "AD1-015" => field_has(r, player, "TAMER"),
        "EX11-012" => !field_has(r, 1 - player, "PLAIN"),
        "EX13-057" => field_has(r, player, "CHRON-L6"),
        "P-153" => !field_has(r, player, "P-153"),
        _ => unreachable!(),
    }
}

fn attack_security(r: &mut DebugRunner, attacker: PermanentHandle) -> bool {
    use digimon_engine::action::space::SECURITY_TARGET;
    let defender = 1 - attacker.player;
    // Security fixture is placed without game setup/shuffling, so every card
    // is a passive 1000-DP Digimon and the attacker survives its real check.
    let fill = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "FILL")
        .unwrap();
    for _ in 0..3 {
        let idx = r.game.next_card_index();
        r.game.players[defender as usize].security.push(
            digimon_engine::card_source::CardSource::new(fill, defender, idx),
        );
    }
    let action = encode_attack(attacker.index as u16, SECURITY_TARGET);
    assert_eq!(
        build_action_mask(&r.game, attacker.player)[action as usize],
        1.0
    );
    r.game.decode_action(action, attacker.player);
    let mut end_of_attack_prompt = false;
    for _ in 0..16 {
        if r.pending_selection().is_none() {
            break;
        }
        end_of_attack_prompt |= matches!(
            r.game.pending_attack.as_ref().map(|p| &p.state),
            Some(digimon_engine::selection::AttackState::EndOfAttackDrain(_))
        );
        first_nonpass(r);
    }
    assert!(r.pending_selection().is_none(), "unexpected prompt loop");
    assert!(r.game.pending_attack.is_none());
    end_of_attack_prompt
}

#[test]
fn audit_six_remaining_pool_end_of_attack_cards_only_trigger_for_their_carrier() {
    let mut wrongly_triggered = Vec::new();
    for card in [
        "BT25-041", "EX12-046", "AD1-015", "EX11-012", "EX13-057", "P-153",
    ] {
        for player in [0, 1] {
            for other_player_attacks in [false, true] {
                let attacker_player = if other_player_attacks {
                    1 - player
                } else {
                    player
                };
                let (mut r, carrier) = candidate_board(card, player, attacker_player);
                r.game.players[player as usize].battle_area[carrier.index as usize].is_suspended =
                    true;
                let attacker = r.place_on_field(attacker_player, "PLAIN", Some(0));
                let end_of_attack_prompt = attack_security(&mut r, attacker);
                if end_of_attack_prompt || candidate_payload_happened(&r, card, player) {
                    wrongly_triggered.push((card, player, other_player_attacks));
                }
            }
        }
    }
    assert!(
        wrongly_triggered.is_empty(),
        "unrelated attack triggered printed EndOfAttack: {wrongly_triggered:?}"
    );
}

#[test]
fn audit_six_remaining_pool_end_of_attack_cards_keep_own_attack_payoffs() {
    for card in [
        "BT25-041", "EX12-046", "AD1-015", "EX11-012", "EX13-057", "P-153",
    ] {
        for player in [0, 1] {
            let (mut r, carrier) = candidate_board(card, player, player);
            attack_security(&mut r, carrier);
            assert!(
                candidate_payload_happened(&r, card, player),
                "own attack payoff missing for {card}, player {player}"
            );
        }
    }
}

fn field_has(runner: &DebugRunner, player: u8, id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == id)
}

fn pick(runner: &mut DebugRunner, action: u16) {
    let view = runner
        .pending_selection_view()
        .expect("selection must be pending");
    assert!(
        view.valid_action_ids.contains(&action),
        "action {action} not in {:?}: {}",
        view.valid_action_ids,
        view.prompt
    );
    assert_eq!(
        build_action_mask(&runner.game, view.selecting_player)[action as usize],
        1.0
    );
    runner
        .execute_action(view.selecting_player, action)
        .unwrap();
}

fn first_nonpass(runner: &mut DebugRunner) {
    let view = runner
        .pending_selection_view()
        .expect("selection must be pending");
    let action = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("non-pass action");
    pick(runner, action);
}

fn finish(runner: &mut DebugRunner) {
    for _ in 0..16 {
        if runner.pending_selection().is_none() {
            return;
        }
        first_nonpass(runner);
    }
    panic!(
        "unexpected prompt loop: {:?}",
        runner.pending_selection_view()
    );
}

fn choose_sw_zone(runner: &mut DebugRunner, zone: &str) {
    let initial = runner
        .pending_selection_view()
        .expect("own Digimon leave must offer Erlangmon");
    assert_eq!(initial.kind, SelectionKind::Replacement);
    first_nonpass(runner);
    let view = runner.pending_selection_view().expect("hand/source choice");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    let action = view
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|c| c.label == zone)
        .expect("printed zone choice")
        .action_id;
    pick(runner, action);
}

fn sw_leave_case(player: u8, from_sources: bool, own_leave: bool, clone_at_pick: bool) {
    let mut runner = setup("EX12-034", player);
    // The other victim precedes Erlangmon, exercising handle rebinding after
    // the original leave compacts the field. Payload remains Erlangmon's own.
    let other = runner.place_on_field(player, "PLAIN", Some(0));
    let erlang = runner.place_stack(player, &["SW-A", "SW-TOO-HIGH", "EX12-034"]);
    if !from_sources {
        runner.add_to_hand(player, "SW-A");
        runner.add_to_hand(player, "SW-TOO-HIGH");
        runner.add_to_hand(player, "FILL");
    }
    let target = if own_leave { erlang } else { other };
    let target_id = if own_leave { "EX12-034" } else { "PLAIN" };
    let memory_before = runner.memory();
    runner.game.return_to_hand_from_effect(target, 1 - player);
    choose_sw_zone(&mut runner, if from_sources { "Sources" } else { "Hand" });
    let selection = runner
        .pending_selection_view()
        .expect("matching SW candidate");
    assert_eq!(
        selection
            .valid_action_ids
            .iter()
            .filter(|&&a| a != PASS)
            .count(),
        1,
        "only the level <=5 SW candidate is legal"
    );
    if clone_at_pick {
        runner = DebugRunner::wrap(runner.game.clone());
    }
    finish(&mut runner);
    assert!(
        field_has(&runner, player, "SW-A"),
        "free-play payload must enter"
    );
    assert!(
        !field_has(&runner, player, target_id),
        "original leave must still occur"
    );
    assert!(runner.game.players[player as usize]
        .hand
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == target_id));
    assert_eq!(runner.memory(), memory_before, "free play pays no memory");
    if !own_leave {
        let sources = &runner.game.players[player as usize]
            .battle_area
            .iter()
            .find(|p| p.top_card().card_id(&runner.game.card_data) == "EX12-034")
            .unwrap()
            .card_sources;
        assert_eq!(
            sources
                .iter()
                .filter(|c| c.card_id(&runner.game.card_data) == "SW-A")
                .count(),
            usize::from(!from_sources),
            "only a selected source leaves Erlangmon"
        );
    }
}

#[test]
fn audit_erlangmon_own_leave_free_plays_from_hand() {
    for p in [0, 1] {
        sw_leave_case(p, false, true, false);
    }
}

#[test]
fn audit_erlangmon_own_leave_free_plays_from_sources_after_clone() {
    for p in [0, 1] {
        sw_leave_case(p, true, true, true);
    }
}

#[test]
fn audit_erlangmon_other_own_leave_free_plays_from_hand() {
    for p in [0, 1] {
        sw_leave_case(p, false, false, false);
    }
}

#[test]
fn audit_erlangmon_other_own_leave_plays_own_source_after_clone_and_compaction() {
    for p in [0, 1] {
        sw_leave_case(p, true, false, true);
    }
}

#[test]
fn audit_erlangmon_does_not_trigger_on_opponent_digimon_or_own_tamer_leave() {
    for player in [0, 1] {
        for own_tamer in [false, true] {
            let mut runner = setup("EX12-034", player);
            runner.place_stack(player, &["SW-A", "EX12-034"]);
            let target = runner.place_on_field(
                if own_tamer { player } else { 1 - player },
                if own_tamer { "TAMER" } else { "PLAIN" },
                Some(0),
            );
            runner.game.return_to_hand_from_effect(target, 1 - player);
            assert!(
                runner.pending_selection().is_none(),
                "unrelated leave must not offer Erlangmon: {:?}",
                runner.pending_selection_view()
            );
            assert!(!field_has(&runner, player, "SW-A"));
        }
    }
}

#[test]
fn audit_erlangmon_successful_free_play_spends_once_per_turn() {
    for player in [0, 1] {
        let mut runner = setup("EX12-034", player);
        runner.place_stack(player, &["SW-A", "SW-B", "EX12-034"]);
        let first = runner.place_on_field(player, "PLAIN", Some(0));
        runner.game.return_to_hand_from_effect(first, 1 - player);
        choose_sw_zone(&mut runner, "Sources");
        finish(&mut runner);
        assert!(field_has(&runner, player, "SW-A"));
        let second = runner.place_on_field(player, "FILL", Some(0));
        runner.game.return_to_hand_from_effect(second, 1 - player);
        assert!(
            runner.pending_selection().is_none(),
            "second own leave cannot re-use spent OPT"
        );
        assert!(!field_has(&runner, player, "SW-B"));
    }
}

fn gd_host_and_tamer(runner: &mut DebugRunner, player: u8) -> (PermanentHandle, PermanentHandle) {
    let host = runner.place_stack(player, &["ST23-04", "HOST"]);
    let tamer = runner.place_stack(player, &["FILL", "TAMER"]);
    runner.game.players[player as usize].battle_area[tamer.index as usize].card_sources[0]
        .face_down = true;
    (host, tamer)
}

fn real_attack(runner: &mut DebugRunner, attacker: PermanentHandle, defender: PermanentHandle) {
    runner.game.players[defender.player as usize].battle_area[defender.index as usize]
        .is_suspended = true;
    let action = encode_attack(attacker.index as u16, defender.index as u16);
    assert_eq!(
        build_action_mask(&runner.game, attacker.player)[action as usize],
        1.0,
        "fixture must offer a legal main-phase attack"
    );
    runner.game.decode_action(action, attacker.player);
    finish(runner);
    assert!(
        runner.game.pending_attack.is_none(),
        "full attack must finish"
    );
}

#[test]
fn audit_murasamemon_own_attack_pays_and_unsuspends_host() {
    for player in [0, 1] {
        let mut runner = setup("ST23-04", player);
        let (host, tamer) = gd_host_and_tamer(&mut runner, player);
        let defender = runner.place_on_field(1 - player, "FILL", Some(0));
        real_attack(&mut runner, host, defender);
        assert!(
            !runner.game.players[player as usize].battle_area[host.index as usize].is_suspended
        );
        assert_eq!(
            runner.game.players[player as usize].battle_area[tamer.index as usize]
                .card_sources
                .len(),
            1
        );
        assert_eq!(runner.trash_size(player), 1);
    }
}

#[test]
fn audit_murasamemon_another_ally_attack_does_not_unsuspend_host_or_spend_resource() {
    let mut observed = Vec::new();
    for player in [0, 1] {
        let mut runner = setup("ST23-04", player);
        let (host, tamer) = gd_host_and_tamer(&mut runner, player);
        runner.game.players[player as usize].battle_area[host.index as usize].is_suspended = true;
        let attacker = runner.place_on_field(player, "PLAIN", Some(0));
        let defender = runner.place_on_field(1 - player, "FILL", Some(0));
        real_attack(&mut runner, attacker, defender);
        observed.push((
            player,
            runner.game.players[player as usize].battle_area[host.index as usize].is_suspended,
            runner.game.players[player as usize].battle_area[tamer.index as usize]
                .card_sources
                .len(),
            runner.trash_size(player),
        ));
    }
    assert_eq!(observed, vec![(0, true, 2, 0), (1, true, 2, 0)],
        "CR 15-16-15-1: another ally's attack must not trigger this inherited [End of Attack]; rows are (owner, suspended, Tamer stack size, own trash)");
}

#[test]
fn audit_murasamemon_opponent_attack_does_not_unsuspend_host_or_spend_resource() {
    let mut observed = Vec::new();
    for player in [0, 1] {
        let mut runner = setup("ST23-04", 1 - player);
        let (host, tamer) = gd_host_and_tamer(&mut runner, player);
        runner.game.players[player as usize].battle_area[host.index as usize].is_suspended = true;
        let defender = runner.place_on_field(player, "FILL", Some(0));
        let attacker = runner.place_on_field(1 - player, "PLAIN", Some(0));
        real_attack(&mut runner, attacker, defender);
        observed.push((
            player,
            runner.game.players[player as usize].battle_area[host.index as usize].is_suspended,
            runner.game.players[player as usize].battle_area[tamer.index as usize]
                .card_sources
                .len(),
            runner.trash_size(player),
        ));
    }
    assert_eq!(observed, vec![(0, true, 2, 1), (1, true, 2, 1)],
        "CR 15-16-15-1: opponent's attack must not trigger this inherited [End of Attack]; rows are (owner, suspended, Tamer stack size, own trash)");
}

/// A top-scope control for the same timing defect, present in the frozen
/// Three Musketeers lists. Printed EX4-074 [End of Attack] is carrier-scoped;
/// another Digimon attacking cannot force Ruin Mode to delete itself.
#[test]
fn audit_ruin_mode_another_ally_attack_does_not_self_delete_or_recover() {
    let mut observed = Vec::new();
    for player in [0, 1] {
        let mut runner = setup("EX4-074", player);
        runner.place_on_field(player, "EX4-074", Some(0));
        let attacker = runner.place_on_field(player, "PLAIN", Some(0));
        let defender = runner.place_on_field(1 - player, "FILL", Some(0));
        real_attack(&mut runner, attacker, defender);
        observed.push((
            player,
            field_has(&runner, player, "EX4-074"),
            runner.security_count(player),
        ));
    }
    assert_eq!(observed, vec![(0, true, 0), (1, true, 0)],
        "CR 15-16-15-1: another ally's attack cannot trigger Ruin Mode's self-deletion or Recovery; rows are (owner, Ruin Mode remains, security)");
}
