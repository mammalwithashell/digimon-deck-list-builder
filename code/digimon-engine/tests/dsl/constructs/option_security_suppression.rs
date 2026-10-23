//! Attacker-local, Option-only Security activation restriction (BT20-015 ESS).
//! Image and BT20_015.cs inspected 2026-10-22. CR 4.3 §§13-1-6/8,
//! 15-11-2-3-2 and 15-16-10 distinguish checks, continuous live conditions,
//! immediate Security activation and ordinary effect-driven security removal.
//! Inline fixtures deliberately isolate the reusable modifier from the card.

use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, HAND_EFFECT_START, PASS, PLAY_HAND_START, SECURITY_TARGET},
    },
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardKind, EffectSourceKind, GamePhase},
    permanent::PermanentHandle,
    selection::SelectionKind,
};

const SOURCE: &str = r#"
card: OSS-SOURCE
name: Option Security suppression source
kind: digimon
color: [red]
level: 5
cost: 7
dp: 7000
effects:
  - kind: aura
    scope: inherited
    active_when: { your_turn: true }
    target: {}
    modifier: DoesNotActivateOptionSecurityEffects
"#;

const HOST: &str = r#"
card: OSS-HOST
name: Security checking host
kind: digimon
color: [red]
level: 6
cost: 10
dp: 15000
effects:
  - when: on_security_check
    process:
      - draw: { of: you, count: 1 }
"#;

const SECURITY: &str = r#"
card: OSS-SECURITY
name: Checked security witness
kind: option
color: [red]
cost: 0
effects:
  - scope: security
    when: on_security
    process:
      - draw: { of: you, count: 2 }
      - gain_memory: 1
  - when: on_discard_security
    process:
      - draw: { of: you, count: 3 }
"#;

const PROMPT_SECURITY: &str = r#"
card: OSS-SECURITY
name: Optional checked security witness
kind: option
color: [red]
cost: 0
effects:
  - scope: security
    when: on_security
    process:
      - select_effect_choice:
          labels: [Draw one, Draw two]
          bind_as: choice
          prompt: Choose Security draw
      - if:
          condition: { equals: [choice, 0] }
          then:
            - draw: { of: you, count: 1 }
          else:
            - draw: { of: you, count: 2 }
"#;

// Both clauses are already collected before the first selection parks. The
// first body's memory change enables the inherited continuous modifier; its
// own remaining draw must finish, while the second clause must not activate.
const LIVE_SECURITY: &str = r#"
card: OSS-SECURITY
name: Live activation boundary witness
kind: option
color: [red]
cost: 0
effects:
  - scope: security
    when: on_security
    process:
      - select_effect_choice:
          labels: [Enable suppression, Leave suppression inactive]
          bind_as: choice
          prompt: Choose the memory change
      - if:
          condition: { equals: [choice, 0] }
          then:
            - gain_memory: 4
      - draw: { of: you, count: 1 }
  - scope: security
    when: on_security
    process:
      - draw: { of: you, count: 4 }
"#;

const TRASHER: &str = r#"
card: OSS-TRASHER
name: Ordinary security removal
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    process:
      - trash_top_security: { of: opponent, count: 1 }
"#;

fn runner(owner: u8, source: &str, security: &str, host: &str) -> DebugRunner {
    let mut filler = make_test_card("OSS-FILLER", "Deck witness");
    filler.card_kind = CardKind::Digimon;
    filler.level = Some(3);
    filler.dp = Some(1000);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(source)
        .expect("source construct compiles")
        .from_dsl_yaml(security)
        .expect("security fixture compiles")
        .from_dsl_yaml(host)
        .expect("host fixture compiles")
        .from_dsl_yaml(TRASHER)
        .expect("security removal fixture compiles")
        .add_card(filler)
        .deck(0, &["OSS-FILLER"; 30])
        .deck(1, &["OSS-FILLER"; 30])
        .security(0, &["OSS-SECURITY"; 4])
        .security(1, &["OSS-SECURITY"; 4])
        .memory(8)
        .start();
    r.skip_mulligan();
    r.set_first_player(owner);
    r.game.enter_main_phase();
    r
}

fn main_action(r: &mut DebugRunner, owner: u8, id: u16) {
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.turn_player(), owner);
    assert_eq!(build_action_mask(&r.game, owner)[id as usize], 1.0);
    r.game.decode_action(id, owner);
}

fn attack(r: &mut DebugRunner, host: PermanentHandle) {
    main_action(
        r,
        host.player,
        encode_attack(host.index as u16, SECURITY_TARGET),
    );
}

fn settled(r: &DebugRunner) {
    assert!(
        r.game.pending_selection.is_none(),
        "no suppressed Security prompt may leak"
    );
    assert!(r.game.security_resolution.is_none());
    assert!(r.game.pending_security.is_none());
    assert!(r.game.pending_attack.is_none());
}

fn choose(r: &mut DebugRunner, owner: u8, action: u16) {
    let s = r
        .game
        .pending_selection
        .as_ref()
        .expect("Security has a real choice");
    assert_eq!(s.selecting_player, owner);
    assert_eq!(s.kind, SelectionKind::EffectChoice);
    assert_eq!(s.source_kind, EffectSourceKind::Option);
    assert_eq!(s.source_permanent, None);
    assert_eq!(
        s.source_card,
        r.game.pending_security.as_ref().unwrap().card.handle(),
        "choice provenance stays on the checked Option"
    );
    assert!(!s.is_optional);
    assert_eq!(
        s.valid_action_ids,
        vec![HAND_EFFECT_START, HAND_EFFECT_START + 1]
    );
    let mask = build_action_mask(&r.game, owner);
    assert_eq!(mask[HAND_EFFECT_START as usize], 1.0);
    assert_eq!(mask[HAND_EFFECT_START as usize + 1], 1.0);
    assert_eq!(mask[PASS as usize], 0.0);
    r.execute_action(owner, HAND_EFFECT_START + action)
        .expect("legal public selection");
}

#[test]
fn option_security_is_suppressed_only_for_the_actual_inheriting_attacker() {
    for owner in [0, 1] {
        let defender = 1 - owner;
        let mut r = runner(owner, SOURCE, SECURITY, HOST);
        let host = r.place_stack(owner, &["OSS-SOURCE", "OSS-HOST"]);
        let before = (
            r.hand_size(owner),
            r.hand_size(defender),
            r.memory(),
            r.security_count(defender),
        );
        let checked = r.game.players[defender as usize]
            .security
            .last()
            .unwrap()
            .handle();
        attack(&mut r, host);
        settled(&r);
        assert_eq!(
            r.hand_size(defender),
            before.1,
            "Option Security draw is suppressed"
        );
        assert_eq!(r.memory(), before.2, "Option Security memory is suppressed");
        assert_eq!(
            r.hand_size(owner),
            before.0 + 1,
            "check observer remains live"
        );
        assert_eq!(r.security_count(defender), before.3 - 1);
        assert!(r.game.players[defender as usize]
            .trash
            .iter()
            .any(|c| c.handle() == checked));
    }
}

#[test]
fn digimon_and_tamer_security_effects_still_activate_with_the_same_source() {
    for kind in ["digimon", "tamer"] {
        let sec = SECURITY.replace("kind: option", &format!("kind: {kind}"));
        let sec = if kind == "digimon" {
            sec.replace("cost: 0", "cost: 0\nlevel: 3\ndp: 1000")
        } else {
            sec
        };
        let mut r = runner(0, SOURCE, &sec, HOST);
        let host = r.place_stack(0, &["OSS-SOURCE", "OSS-HOST"]);
        let hand = r.hand_size(1);
        let memory = r.memory();
        attack(&mut r, host);
        settled(&r);
        assert_eq!(
            r.hand_size(1),
            hand + 2,
            "{kind} Security is not an Option effect"
        );
        assert_eq!(r.memory(), memory - 1);
        assert_eq!(r.battle_area_size(0), 1);
    }
}

#[test]
fn a_sibling_attacker_does_not_borrow_the_sources_restriction() {
    let mut r = runner(0, SOURCE, SECURITY, HOST);
    r.place_stack(0, &["OSS-SOURCE", "OSS-HOST"]);
    let sibling = r.place_on_field(0, "OSS-FILLER", Some(0));
    let hand = r.hand_size(1);
    let memory = r.memory();
    attack(&mut r, sibling);
    settled(&r);
    assert_eq!(r.hand_size(1), hand + 2);
    assert_eq!(r.memory(), memory - 1);
}

#[test]
fn a_top_card_or_face_down_source_does_not_supply_the_inherited_restriction() {
    for face_down in [false, true] {
        let mut r = runner(0, SOURCE, SECURITY, HOST);
        let host = if face_down {
            let h = r.place_stack(0, &["OSS-SOURCE", "OSS-HOST"]);
            r.game.players[0].battle_area[h.index as usize].card_sources[0].face_down = true;
            h
        } else {
            r.place_on_field(0, "OSS-SOURCE", Some(0))
        };
        let hand = r.hand_size(1);
        let memory = r.memory();
        attack(&mut r, host);
        settled(&r);
        assert_eq!(r.hand_size(1), hand + 2);
        assert_eq!(r.memory(), memory - 1);
    }
}

#[test]
fn removing_the_inherited_source_clears_its_live_restriction() {
    let mut r = runner(0, SOURCE, SECURITY, HOST);
    let host = r.place_stack(0, &["OSS-SOURCE", "OSS-HOST"]);
    r.game.tick_declarative_effects();
    r.trash_one_source(host);
    let hand = r.hand_size(1);
    let memory = r.memory();
    attack(&mut r, host);
    settled(&r);
    assert_eq!(r.hand_size(1), hand + 2);
    assert_eq!(r.memory(), memory - 1);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
}

#[test]
fn an_off_turn_effect_attack_does_not_enable_a_your_turn_restriction() {
    let off_turn_host = HOST.replace("  - when: on_security_check", "  - when: start_of_opponents_main_phase\n    process:\n      - force_attack: { attacker: source, targets: any }\n  - when: on_security_check");
    let mut r = runner(0, SOURCE, SECURITY, &off_turn_host);
    r.place_stack(0, &["OSS-SOURCE", "OSS-HOST"]);
    r.game.tick_declarative_effects();
    main_action(&mut r, 0, PASS);
    assert_eq!(r.game.turn_player(), 1);
    let hand = r.hand_size(1); // Exclude the normal draw phase.
    let memory = r.memory();
    if r.game.current_phase == GamePhase::Breeding {
        main_action(&mut r, 1, PASS);
    }
    let s = r
        .game
        .pending_selection
        .as_ref()
        .expect("off-turn attacker chooses a legal target");
    assert_eq!(s.selecting_player, 0);
    let target = encode_attack(0, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[target as usize], 1.0);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 0.0);
    r.execute_action(0, target)
        .expect("legal off-turn effect attack target");
    settled(&r);
    assert_eq!(r.hand_size(1), hand + 2);
    assert_eq!(
        r.memory(),
        memory + 1,
        "Security owner is the active player here"
    );
}

#[test]
fn suppressed_option_security_does_not_offer_its_choice_but_sibling_does() {
    let mut r = runner(0, SOURCE, PROMPT_SECURITY, HOST);
    let host = r.place_stack(0, &["OSS-SOURCE", "OSS-HOST"]);
    let sibling = r.place_on_field(0, "OSS-FILLER", Some(0));
    let hand = r.hand_size(1);
    attack(&mut r, host);
    settled(&r);
    assert_eq!(r.hand_size(1), hand);
    attack(&mut r, sibling);
    choose(&mut r, 1, 1);
    settled(&r);
    assert_eq!(r.hand_size(1), hand + 2);
}

#[test]
fn ordinary_security_trashing_keeps_its_discard_trigger_and_has_no_security_activation() {
    let mut r = runner(0, SOURCE, SECURITY, HOST);
    r.place_stack(0, &["OSS-SOURCE", "OSS-HOST"]);
    r.game.tick_declarative_effects();
    let hand_index = r.add_to_hand(0, "OSS-TRASHER");
    let hand = r.hand_size(1);
    let memory = r.memory();
    let security = r.security_count(1);
    main_action(&mut r, 0, PLAY_HAND_START + hand_index as u16);
    settled(&r);
    assert_eq!(
        r.hand_size(1),
        hand + 3,
        "on-discard body runs; on-Security does not"
    );
    assert_eq!(r.memory(), memory);
    assert_eq!(r.security_count(1), security - 1);
}

#[test]
fn live_restriction_is_rechecked_between_security_clauses_after_a_cloned_choice() {
    let conditional = SOURCE.replace(
        "active_when: { your_turn: true }",
        "active_when: { your_turn: true, own_memory_lte: 4 }",
    );
    let mut r = runner(0, &conditional, LIVE_SECURITY, HOST);
    let host = r.place_stack(0, &["OSS-SOURCE", "OSS-HOST"]);
    let hand = r.hand_size(1);
    attack(&mut r, host);
    assert_eq!(r.game.pending_security.as_ref().unwrap().defender, 1);
    assert_eq!(
        r.game.security_resolution.as_ref().unwrap().attacker,
        Some(host)
    );
    let snapshot = r.game.clone();
    choose(&mut r, 1, 0);
    settled(&r);
    assert_eq!(
        r.hand_size(1),
        hand + 1,
        "finish activated body, suppress later clause"
    );
    assert_eq!(r.memory(), 4);

    r.game = snapshot;
    choose(&mut r, 1, 1);
    settled(&r);
    assert_eq!(
        r.hand_size(1),
        hand + 5,
        "unrestricted clone resolves both clauses"
    );
    assert_eq!(r.memory(), 8);
}

#[test]
fn moving_the_checked_option_to_hand_does_not_erase_its_activation_restriction() {
    let conditional = SOURCE.replace(
        "active_when: { your_turn: true }",
        "active_when: { your_turn: true, own_memory_lte: 4 }",
    );
    let returning_security = LIVE_SECURITY.replace(
        "      - draw: { of: you, count: 1 }",
        "      - add_this_option_to_hand: {}\n      - draw: { of: you, count: 1 }",
    );
    let mut r = runner(0, &conditional, &returning_security, HOST);
    let host = r.place_stack(0, &["OSS-SOURCE", "OSS-HOST"]);
    let hand = r.hand_size(1);
    let deck = r.deck_size(1);
    let security = r.security_count(1);
    let checked = r.game.players[1].security.last().unwrap().handle();
    attack(&mut r, host);
    let snapshot = r.game.clone();

    for choice in [0, 1] {
        r.game = snapshot.clone();
        choose(&mut r, 1, choice);
        settled(&r);
        let draw_count = if choice == 0 { 1 } else { 5 };
        assert_eq!(r.deck_size(1), deck - draw_count);
        assert_eq!(
            r.hand_size(1),
            hand + 1 + draw_count,
            "returning the checked card cannot allow a suppressed later clause to activate"
        );
        assert_eq!(r.security_count(1), security - 1);
        assert_eq!(
            r.game.players[1]
                .hand
                .iter()
                .filter(|c| c.handle() == checked)
                .count(),
            1,
            "the real checked Option moved to hand exactly once"
        );
        assert!(!r.game.players[1]
            .trash
            .iter()
            .any(|c| c.handle() == checked));
        assert_eq!(r.memory(), if choice == 0 { 4 } else { 8 });
    }
}

#[test]
fn security_precedes_check_observers_that_would_later_enable_suppression() {
    let conditional = SOURCE.replace(
        "active_when: { your_turn: true }",
        "active_when: { your_turn: true, own_memory_lte: 4 }",
    );
    let host_yaml = HOST.replace("- draw: { of: you, count: 1 }", "- gain_memory: -4");
    let mut r = runner(0, &conditional, SECURITY, &host_yaml);
    let host = r.place_stack(0, &["OSS-SOURCE", "OSS-HOST"]);
    let hand = r.hand_size(1);
    attack(&mut r, host);
    settled(&r);
    assert_eq!(
        r.hand_size(1),
        hand + 2,
        "Security activates before the check observer"
    );
    assert_eq!(r.memory(), 3);
}
