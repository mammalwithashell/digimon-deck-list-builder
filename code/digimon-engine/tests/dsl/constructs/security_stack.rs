//! Security constructs: CR 4.3 Â§Â§3-1-3-8, 3-2-3, 3-7-2/3/4, 4-7-1/6,
//! 13-1-6, 15-7-2/4, 15-8-3-2 and 15-15-3-2. Index zero is bottom.
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    CardKind, EffectTiming, Expiry, ModifierType, TriggerSource,
};

const ADD: &str = r#"
card: PROBE
name: Probe
kind: digimon
color: [yellow]
level: 4
cost: 3
dp: 4000
effects:
  - when: when_digivolving
    process:
      - add_top_security_to_hand: { of: you }
      - gain_memory: 1
"#;
const RETURN: &str = r#"
card: PROBE
name: Probe
kind: digimon
color: [yellow]
level: 4
cost: 3
dp: 4000
effects:
  - when: when_digivolving
    process:
      - return_top_security_to_deck: { of: opponent, position: bottom }
      - if:
          condition: { effect_returned_any_card: true }
          then:
            - gain_memory: 1
"#;
const PLACE: &str = r#"
card: PROBE
name: Probe
kind: digimon
color: [yellow]
level: 4
cost: 3
dp: 4000
effects:
  - when: when_digivolving
    process:
      - security_place_top_card: { carrier: source, of: you, position: bottom, face: down }
"#;
const TRASH: &str = r#"
card: PROBE
name: Probe
kind: digimon
color: [yellow]
level: 4
cost: 3
dp: 4000
effects:
  - when: when_digivolving
    process:
      - select_security:
          of: opponent
          bind_as: pick
          optional: true
          filter: {}
          prompt: Choose security
      - trash_selected_security: { of: opponent, card: pick }
"#;
const PLAY: &str = r#"
card: PROBE
name: Probe
kind: digimon
color: [yellow]
level: 4
cost: 3
dp: 4000
effects:
  - when: when_digivolving
    process:
      - select_security:
          of: you
          bind_as: pick
          optional: true
          filter: { kind: tamer }
          prompt: Choose security
      - play_security_card: { of: you, card: pick }
"#;
const LOSE: &str = r#"
card: OBS
name: Observer
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_lose_security
    active_when: { all_turns: true }
    process:
      - gain_memory: 1
      - if:
          condition: { event_is_effect_initiated: true }
          then:
            - gain_memory: 2
"#;
const REMOVE: &str = r#"
card: PROBE
name: Probe
kind: digimon
color: [yellow]
level: 4
cost: 3
dp: 4000
effects:
  - when: when_digivolving
    process:
      - trash_top_security: { of: you }
"#;
const OWNER: &str = r#"
card: PROBE
name: Probe
kind: digimon
color: [yellow]
level: 4
cost: 3
dp: 4000
effects:
  - when: when_digivolving
    process:
      - if:
          condition:
            any_permanent: { of: opponent, zone: [battle_area], kind: digimon, owner_can_add_security: true }
          then:
            - gain_memory: 1
      - select_any_permanent:
          bind_as: body
          optional: true
          filter: { kind: digimon, owner_can_add_security: true }
          prompt: Choose payable body
"#;
const DISCARD: &str = r#"
card: SB
name: Discard witness
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: on_discard_security
    process:
      - gain_memory: 1
"#;
fn runner(y: &str, extra: &[&str], own: &[&str], opp: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder().from_dsl_yaml(y).unwrap();
    for e in extra {
        b = b.from_dsl_yaml(e).unwrap();
    }
    for id in [
        "BOT", "TOP", "SA", "SB", "SC", "UNDER", "D1", "D2", "DIG", "X0", "X1",
    ] {
        if id == "SB" && extra.iter().any(|e| e.contains("card: SB")) {
            continue;
        }
        b = b.add_card(make_test_card(id, id));
    }
    let mut t = make_test_card("TAM", "Tamer");
    t.card_kind = CardKind::Tamer;
    t.play_cost = 5;
    t.level = None;
    t.dp = None;
    if !extra.iter().any(|e| e.contains("card: TAM")) {
        b = b.add_card(t);
    }
    let mut r = b
        .security(0, own)
        .security(1, opp)
        .deck(0, &["D1", "D2"])
        .deck(1, &["D1", "D2"])
        .memory(0)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn fire(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn probe(r: &mut DebugRunner) {
    let h = r.place_on_field(0, "PROBE", Some(0));
    fire(r, h);
}
fn ids(r: &DebugRunner, p: usize, zone: &str) -> Vec<String> {
    let p = &r.game.players[p];
    let v = match zone {
        "security" => &p.security,
        "hand" => &p.hand,
        "trash" => &p.trash,
        "deck" => &p.deck,
        _ => panic!(),
    };
    v.iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}
fn no_prompt(r: &DebugRunner) {
    assert!(r.game.pending_selection.is_none());
}
fn pick(r: &mut DebugRunner, expected: &[u16], action: u16) {
    let p = r.game.pending_selection.as_ref().expect("real choice");
    assert!(p.is_optional);
    assert_eq!(p.selecting_player, 0);
    assert_eq!(p.valid_action_ids, expected);
    let mask = build_action_mask(&r.game, 0);
    for (id, value) in mask.iter().enumerate() {
        assert_eq!(
            *value,
            if expected.contains(&(id as u16)) || id == PASS as usize {
                1.0
            } else {
                0.0
            },
            "mask action {id}"
        );
    }
    r.execute_action(0, action).unwrap();
    no_prompt(r);
}
fn restrict(r: &mut DebugRunner, p: u8) {
    r.game.modifiers.add_player_modifier(
        p,
        PlayerModifierEntry::simple(
            ModifierType::CannotAddSecurityByEffect,
            0,
            Expiry::Permanent,
            None,
            0,
        ),
    );
}

#[test]
fn add_top_security_to_hand_moves_the_top_card_to_its_owners_hand_without_a_prompt() {
    let mut r = runner(ADD, &[], &["BOT", "TOP"], &[]);
    probe(&mut r);
    assert_eq!(ids(&r, 0, "security"), ["BOT"]);
    assert_eq!(ids(&r, 0, "hand"), ["TOP"]);
    assert_eq!(r.memory(), 1);
    no_prompt(&r);
}
#[test]
fn add_top_security_to_hand_of_opponent_moves_their_top_card() {
    let mut r = runner(
        &ADD.replace("of: you", "of: opponent"),
        &[],
        &[],
        &["BOT", "TOP"],
    );
    probe(&mut r);
    assert_eq!(ids(&r, 1, "security"), ["BOT"]);
    assert_eq!(ids(&r, 1, "hand"), ["TOP"]);
    assert!(ids(&r, 0, "hand").is_empty());
    no_prompt(&r);
}
#[test]
fn add_top_security_to_hand_routes_to_the_card_owner_not_the_stack_owner() {
    let mut r = runner(ADD, &[], &["BOT", "TOP"], &[]);
    r.game.players[0].security.last_mut().unwrap().owner = 1;
    probe(&mut r);
    assert_eq!(ids(&r, 1, "hand"), ["TOP"]);
    assert!(ids(&r, 0, "hand").is_empty());
}
#[test]
fn add_top_security_to_hand_clears_the_face_up_record() {
    let mut r = runner(ADD, &[], &["BOT", "TOP"], &[]);
    let key = r.game.players[0].security[1].card_index;
    r.game.players[0].face_up_security.insert(key);
    probe(&mut r);
    assert!(!r.game.players[0].face_up_security.contains(&key));
    assert_eq!(ids(&r, 0, "hand"), ["TOP"]);
}
#[test]
fn add_top_security_to_hand_on_empty_stack_is_a_no_op_that_does_not_abort_the_clause() {
    let mut r = runner(ADD, &[], &[], &[]);
    probe(&mut r);
    assert!(ids(&r, 0, "hand").is_empty());
    assert_eq!(r.memory(), 1);
    no_prompt(&r);
}
#[test]
fn add_top_security_to_hand_records_the_added_card() {
    let y=ADD.replace("      - gain_memory: 1","      - if:\n          condition: { effect_added_any_card_to_hand: true }\n          then:\n            - gain_memory: 1");
    for (sec, mem) in [(vec!["BOT", "TOP"], 1), (vec![], 0)] {
        let mut r = runner(&y, &[], &sec, &[]);
        probe(&mut r);
        assert_eq!(r.memory(), mem);
    }
}
#[test]
fn return_top_security_to_deck_bottom_puts_the_card_at_deck_index_zero_and_records_the_return() {
    let mut r = runner(RETURN, &[], &[], &["BOT", "TOP"]);
    probe(&mut r);
    assert_eq!(ids(&r, 1, "deck"), ["TOP", "D1", "D2"]);
    assert_eq!(ids(&r, 1, "security"), ["BOT"]);
    assert_eq!(r.memory(), 1);
    no_prompt(&r);
}
#[test]
fn return_top_security_to_deck_top_puts_it_on_the_next_draw() {
    let mut r = runner(
        &RETURN.replace("position: bottom", "position: top"),
        &[],
        &[],
        &["BOT", "TOP"],
    );
    probe(&mut r);
    assert_eq!(ids(&r, 1, "deck"), ["D1", "D2", "TOP"]);
    r.game.players[1].draw();
    assert_eq!(ids(&r, 1, "hand"), ["TOP"]);
}
#[test]
fn return_top_security_to_deck_routes_to_the_card_owner() {
    let mut r = runner(RETURN, &[], &[], &["BOT", "TOP"]);
    r.game.players[1].security[1].owner = 0;
    probe(&mut r);
    assert_eq!(ids(&r, 0, "deck"), ["TOP", "D1", "D2"]);
    assert_eq!(ids(&r, 1, "deck"), ["D1", "D2"]);
}
#[test]
fn return_top_security_to_deck_on_empty_stack_records_nothing() {
    let mut r = runner(RETURN, &[], &[], &[]);
    probe(&mut r);
    assert_eq!(ids(&r, 1, "deck"), ["D1", "D2"]);
    assert_eq!(r.memory(), 0);
    no_prompt(&r);
}
#[test]
fn return_top_security_to_deck_fires_on_lose_security_once_with_the_effect_bit() {
    let mut r = runner(RETURN, &[LOSE], &[], &["BOT", "TOP"]);
    r.place_on_field(1, "OBS", Some(0));
    probe(&mut r);
    assert_eq!(r.memory(), -2);
    assert_eq!(ids(&r, 1, "security"), ["BOT"]);
}
fn place_case(y: &str, lone: bool, restriction: Option<u8>) -> DebugRunner {
    let mut r = runner(y, &[], &["BOT", "TOP"], &["BOT", "TOP"]);
    if let Some(p) = restriction {
        restrict(&mut r, p);
    }
    let h = if lone {
        r.place_on_field(0, "PROBE", Some(0))
    } else {
        r.place_stack(0, &["UNDER", "PROBE"])
    };
    r.game.players[0].battle_area[h.index as usize].is_suspended = true;
    fire(&mut r, h);
    no_prompt(&r);
    r
}
#[test]
fn security_place_top_card_moves_the_visible_top_card_face_down_to_the_bottom_and_keeps_the_stack_in_play(
) {
    let r = place_case(PLACE, false, None);
    assert_eq!(ids(&r, 0, "security"), ["PROBE", "BOT", "TOP"]);
    let p = &r.game.players[0].battle_area[0];
    assert_eq!(p.card_sources.len(), 1);
    assert_eq!(p.top_card().card_id(&r.game.card_data), "UNDER");
    assert!(p.is_suspended);
    assert!(r.game.players[0].face_up_security.is_empty());
    assert!(ids(&r, 0, "trash").is_empty());
}
#[test]
fn security_place_top_card_top_and_face_up_variants() {
    for (position, face) in [("top", "down"), ("bottom", "up"), ("top", "up")] {
        let y = PLACE
            .replace("position: bottom", &format!("position: {position}"))
            .replace("face: down", &format!("face: {face}"));
        let r = place_case(&y, false, None);
        let idx = if position == "top" { 2 } else { 0 };
        let c = &r.game.players[0].security[idx];
        assert_eq!(c.card_id(&r.game.card_data), "PROBE");
        assert_eq!(
            r.game.players[0].face_up_security.contains(&c.card_index),
            face == "up"
        );
        assert!(r.game.players[0].battle_area[0].is_suspended);
    }
}
fn assert_place_blocked(r: &DebugRunner, n: usize) {
    assert_eq!(ids(r, 0, "security"), ["BOT", "TOP"]);
    assert_eq!(ids(r, 1, "security"), ["BOT", "TOP"]);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), n);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "PROBE"
    );
}
#[test]
fn security_place_top_card_with_a_lone_card_is_a_no_op() {
    assert_place_blocked(&place_case(PLACE, true, None), 1);
}
#[test]
fn security_place_top_card_blocked_by_the_acting_players_cannot_add_security_by_effect() {
    assert_place_blocked(&place_case(PLACE, false, Some(0)), 2);
}
#[test]
fn security_place_top_card_blocked_by_the_destination_players_restriction() {
    assert_place_blocked(
        &place_case(&PLACE.replace("of: you", "of: opponent"), false, Some(1)),
        2,
    );
}
fn trash_runner(extra: &[&str]) -> DebugRunner {
    let mut r = runner(TRASH, extra, &[], &["SA", "SB", "SC"]);
    probe(&mut r);
    r
}
#[test]
fn trash_selected_security_surfaces_a_real_security_pick_over_the_opponents_stack() {
    let mut r = trash_runner(&[]);
    let p = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(p.kind, SelectionKind::Security);
    assert_eq!(p.zone_owner, Some(1));
    assert_eq!(ids(&r, 1, "security"), ["SA", "SB", "SC"]);
    pick(&mut r, &[50, 51, 52], 51);
}
#[test]
fn trash_selected_security_trashes_the_chosen_middle_card() {
    let mut r = trash_runner(&[]);
    pick(&mut r, &[50, 51, 52], 51);
    assert_eq!(ids(&r, 1, "security"), ["SA", "SC"]);
    assert_eq!(ids(&r, 1, "trash"), ["SB"]);
    assert!(ids(&r, 0, "trash").is_empty());
}
#[test]
fn trash_selected_security_pass_changes_nothing() {
    let mut r = trash_runner(&[]);
    pick(&mut r, &[50, 51, 52], PASS);
    assert_eq!(ids(&r, 1, "security"), ["SA", "SB", "SC"]);
    assert!(ids(&r, 1, "trash").is_empty());
}
#[test]
fn trash_selected_security_with_empty_stack_installs_no_selection() {
    let mut r = runner(TRASH, &[], &[], &[]);
    probe(&mut r);
    no_prompt(&r);
    assert!(ids(&r, 1, "trash").is_empty());
}
#[test]
fn trash_selected_security_fires_on_discard_security_on_the_trashed_card_only() {
    for (a, m) in [(51, 2), (50, 3), (PASS, 3)] {
        let mut r = trash_runner(&[DISCARD]);
        r.game.memory = 3;
        pick(&mut r, &[50, 51, 52], a);
        assert_eq!(r.memory(), m);
    }
}
fn play_runner(extra: &[&str]) -> DebugRunner {
    let mut r = runner(PLAY, extra, &["TAM", "DIG"], &[]);
    r.game.memory = 3;
    r.place_on_field(0, "X0", Some(0));
    r
}
#[test]
fn play_security_card_offers_only_filter_matches_and_plays_the_pick_free() {
    let mut r = play_runner(&[]);
    probe(&mut r);
    pick(&mut r, &[40], 40);
    assert_eq!(r.battle_area_size(0), 3);
    assert_eq!(
        r.game.players[0].battle_area[2]
            .top_card()
            .card_id(&r.game.card_data),
        "TAM"
    );
    assert_eq!(ids(&r, 0, "security"), ["DIG"]);
    assert!(ids(&r, 0, "hand").is_empty());
    assert_eq!(r.memory(), 3);
}
#[test]
fn play_security_card_pass_changes_nothing() {
    let mut r = play_runner(&[]);
    probe(&mut r);
    pick(&mut r, &[40], PASS);
    assert_eq!(r.battle_area_size(0), 2);
    assert_eq!(ids(&r, 0, "security"), ["TAM", "DIG"]);
    assert_eq!(r.memory(), 3);
}
#[test]
fn play_security_card_with_a_binding_that_left_the_stack_is_a_no_op() {
    let y = PLAY.replace(
        "      - play_security_card:",
        "      - add_bottom_security_to_hand: { of: you }\n      - play_security_card:",
    );
    let mut r = runner(&y, &[], &["TAM", "DIG"], &[]);
    probe(&mut r);
    pick(&mut r, &[40], 40);
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(ids(&r, 0, "hand"), ["TAM"]);
    assert_eq!(ids(&r, 0, "security"), ["DIG"]);
}
#[test]
fn play_security_card_fires_on_lose_security_once() {
    let mut r = play_runner(&[LOSE]);
    r.place_on_field(0, "OBS", Some(0));
    probe(&mut r);
    pick(&mut r, &[40], 40);
    assert_eq!(r.memory(), 6);
    assert_eq!(ids(&r, 0, "security"), ["DIG"]);
}
fn observer_case(y: &str, obs: &str, p: u8) -> DebugRunner {
    let mut r = runner(y, &[obs], &["BOT", "TOP"], &["BOT", "TOP"]);
    r.place_on_field(p, "OBS", Some(0));
    probe(&mut r);
    r
}
#[test]
fn on_lose_security_fires_once_for_an_effect_removal_with_the_effect_bit() {
    let r = observer_case(REMOVE, LOSE, 0);
    assert_eq!(r.memory(), 3);
    assert_eq!(ids(&r, 0, "security"), ["BOT"]);
    no_prompt(&r);
}
#[test]
fn on_lose_security_fires_for_a_security_check_without_the_effect_bit() {
    let mut r = runner(REMOVE, &[LOSE], &["BOT", "TOP"], &[]);
    r.place_on_field(0, "OBS", Some(0));
    let a = r.place_on_field(1, "X1", Some(0));
    let before = r.memory();
    assert!(matches!(
        r.attack_player(a, 0, true),
        digimon_engine::combat::AttackResult::AttackerDeletedBySecurity
    ));
    assert_eq!(r.memory(), before + 1);
    assert_eq!(r.security_count(0), 1);
}
#[test]
fn on_lose_security_does_not_fire_for_the_other_players_stack() {
    let r = observer_case(&REMOVE.replace("of: you", "of: opponent"), LOSE, 0);
    assert_eq!(r.memory(), 0);
    assert_eq!(r.security_count(1), 1);
}
#[test]
fn on_lose_security_does_not_fire_when_security_is_added() {
    let r = observer_case(
        &REMOVE.replace(
            "trash_top_security: { of: you }",
            "recover: { of: you, count: 1 }",
        ),
        LOSE,
        0,
    );
    assert_eq!(r.memory(), 0);
    assert_eq!(r.security_count(0), 3);
}
#[test]
fn on_lose_security_and_on_opponent_security_removed_are_disjoint() {
    let opp = OPP;
    for (y, m) in [
        (REMOVE.to_string(), 3),
        (REMOVE.replace("of: you", "of: opponent"), 4),
    ] {
        let mut r = runner(&y, &[LOSE, opp], &["BOT", "TOP"], &["BOT", "TOP"]);
        r.place_on_field(0, "OBS", Some(0));
        r.place_on_field(0, "OPP", Some(0));
        probe(&mut r);
        assert_eq!(r.memory(), m);
    }
}
#[test]
fn on_lose_security_security_scoped_self_clause_fires_once_unscoped_does_not() {
    for (scoped, m) in [(true, 3), (false, 0)] {
        let y = if scoped {
            LOSE.replace(
                "when: on_lose_security",
                "when: on_lose_security\n    scope: security",
            )
        } else {
            LOSE.to_string()
        };
        let mut r = runner(REMOVE, &[&y], &["BOT", "OBS"], &[]);
        probe(&mut r);
        assert_eq!(r.memory(), m);
        assert_eq!(ids(&r, 0, "trash"), ["OBS"]);
    }
}
#[test]
fn on_lose_security_optional_variant_offers_a_real_prompt() {
    let y = LOSE.replace(
        "when: on_lose_security",
        "when: on_lose_security\n    optional: true",
    );
    for accept in [true, false] {
        let mut r = observer_case(REMOVE, &y, 0);
        let actions = r
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .clone();
        assert!(!actions.is_empty());
        assert_eq!(r.memory(), 0);
        let a = if accept { actions[0] } else { PASS };
        pick(&mut r, &actions, a);
        assert_eq!(r.memory(), if accept { 3 } else { 0 });
    }
}
fn owner_case(y: &str, restriction: Option<u8>, swap: bool) -> DebugRunner {
    let mut r = runner(y, &[], &[], &[]);
    r.place_on_field(0, "X0", Some(0));
    r.place_on_field(1, "X1", Some(0));
    if swap {
        r.game.players[0].battle_area[0]
            .card_sources
            .last_mut()
            .unwrap()
            .owner = 1;
    }
    if let Some(p) = restriction {
        restrict(&mut r, p);
    }
    probe(&mut r);
    r
}
fn owner_pick(r: &mut DebugRunner, expected: &[u16], memory: i32) {
    assert_eq!(r.memory() as i32, memory);
    pick(r, expected, *expected.last().unwrap());
    assert_eq!(r.battle_area_size(0), 2);
    assert_eq!(r.battle_area_size(1), 1);
}
#[test]
fn owner_can_add_security_true_without_restrictions() {
    let mut r = owner_case(OWNER, None, false);
    owner_pick(&mut r, &[100, 101, 115], 1);
}
#[test]
fn owner_can_add_security_excludes_digimon_whose_owner_cannot_add_security() {
    let mut r = owner_case(OWNER, Some(1), false);
    owner_pick(&mut r, &[100, 101], 0);
}
#[test]
fn owner_can_add_security_reads_the_top_cards_owner_not_the_controller() {
    let mut r = owner_case(OWNER, Some(1), true);
    owner_pick(&mut r, &[101], 0);
}
#[test]
fn owner_can_add_security_ignores_the_acting_player() {
    let mut r = owner_case(OWNER, Some(0), false);
    owner_pick(&mut r, &[115], 1);
}
#[test]
fn owner_can_add_security_false_inverts() {
    let mut r = owner_case(
        &OWNER.replace(
            "owner_can_add_security: true",
            "owner_can_add_security: false",
        ),
        Some(1),
        false,
    );
    owner_pick(&mut r, &[115], 1);
}

// Successful commit, not the temporary hand transit, is the removal event.
const CANCEL_PLAY: &str = r#"
card: CANCEL
name: Cancel play witness
kind: tamer
color: [yellow]
cost: 3
effects:
  - kind: replacement
    trigger: when_would_play
    optional: true
    active_when: { replacement_subject_is_mine: true }
    process:
      - cancel_replacement: {}
"#;
const OWN_REMOVAL: &str = r#"
card: OBS
name: Own removal witness
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_own_security_removed
    active_when: { all_turns: true }
    process:
      - gain_memory: 3
"#;
const OPP_REMOVAL: &str = r#"
card: OBS
name: Opponent removal witness
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_opponent_security_removed
    active_when: { all_turns: true }
    process:
      - gain_memory: 3
"#;
#[test]
fn play_security_card_resumed_commit_fires_each_removal_timing_once_and_not_the_new_body() {
    for (witness, player, delta) in [(LOSE, 0, 3), (OWN_REMOVAL, 0, 3), (OPP_REMOVAL, 1, -3)] {
        let entering = witness.replace("card: OBS", "card: TAM");
        let mut r = runner(
            PLAY,
            &[witness, &entering, CANCEL_PLAY],
            &["TAM", "DIG"],
            &[],
        );
        r.game.memory = 5;
        r.place_on_field(player, "OBS", Some(0));
        r.place_on_field(0, "CANCEL", Some(0));
        probe(&mut r);
        assert_eq!(
            r.game.pending_selection.as_ref().unwrap().valid_action_ids,
            [40]
        );
        let field_before = r.battle_area_size(0);
        r.execute_action(0, 40).unwrap();
        let replacement = r
            .game
            .pending_selection
            .as_ref()
            .expect("play parked on real replacement");
        assert!(replacement.is_optional);
        assert_eq!(r.memory(), 5, "transit is not a committed removal");
        r.execute_action(0, PASS).unwrap();
        no_prompt(&r);
        assert_eq!(
            r.memory(),
            5 + delta,
            "one chain, newly entering TAM cannot see its own departure"
        );
        assert_eq!(ids(&r, 0, "security"), ["DIG"]);
        assert!(ids(&r, 0, "hand").is_empty());
        assert_eq!(r.battle_area_size(0), field_before + 1);
        assert_eq!(
            r.game.players[0]
                .battle_area
                .last()
                .unwrap()
                .top_card()
                .card_id(&r.game.card_data),
            "TAM"
        );
        assert!(r.game.pending_security.is_none());
    }
}
#[test]
fn play_security_card_cancelled_commit_emits_no_removal() {
    for (witness, player) in [(LOSE, 0), (OWN_REMOVAL, 0), (OPP_REMOVAL, 1)] {
        let entering = witness.replace("card: OBS", "card: TAM");
        let mut r = runner(
            PLAY,
            &[witness, &entering, CANCEL_PLAY],
            &["DIG", "TAM"],
            &[],
        );
        r.game.memory = 5;
        r.place_on_field(player, "OBS", Some(0));
        r.place_on_field(0, "CANCEL", Some(0));
        probe(&mut r);
        r.execute_action(0, 41).unwrap();
        let p = r
            .game
            .pending_selection
            .as_ref()
            .expect("real cancel choice");
        assert!(p.is_optional);
        let a = p.valid_action_ids[0];
        assert_ne!(a, PASS);
        r.execute_action(0, a).unwrap();
        no_prompt(&r);
        assert_eq!(r.memory(), 5);
        assert_eq!(ids(&r, 0, "security"), ["DIG", "TAM"]);
        assert!(ids(&r, 0, "hand").is_empty());
        assert_eq!(r.battle_area_size(0), if player == 0 { 3 } else { 2 });
    }
}
#[test]
fn play_security_card_rejected_commit_emits_no_removal() {
    for (witness, player) in [(LOSE, 0), (OWN_REMOVAL, 0), (OPP_REMOVAL, 1)] {
        let mut r = runner(PLAY, &[witness], &["TAM", "DIG"], &[]);
        r.game.memory = 5;
        r.place_on_field(player, "OBS", Some(0));
        r.game.modifiers.add_player_modifier(
            0,
            PlayerModifierEntry::simple(
                ModifierType::CannotPlayTamerByEffect,
                0,
                Expiry::Permanent,
                None,
                0,
            ),
        );
        probe(&mut r);
        pick(&mut r, &[40], 40);
        assert_eq!(r.memory(), 5);
        assert_eq!(ids(&r, 0, "security"), ["TAM", "DIG"]);
        assert_eq!(r.battle_area_size(0), if player == 0 { 2 } else { 1 });
    }
}
const OPP: &str = r#"
card: OPP
name: Opponent removal witness
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_opponent_security_removed
    active_when: { all_turns: true }
    process:
      - gain_memory: 4
"#;
