//! Protection constructs: CR 4.3 §§2-5-2/3, 3-1-3-3, 4-7-1/2/8,
//! 4-16-3, 15-7-1/4, 15-8-5-4, 15-15-5, 16-12.
//! Card-text idioms: BT26-029/085 (opponent scope), BT20-027/EX13-024 (paid plural prevention).
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    EffectTiming, Expiry, ModifierType, TriggerSource,
};
fn runner(ys: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder();
    for y in ys {
        b = b.from_dsl_yaml(y).unwrap();
    }
    for (id, level, dp, guard) in [
        ("L3", 3, 3000, false),
        ("L4", 4, 4000, false),
        ("G1", 5, 5000, true),
        ("G2", 5, 5000, true),
        ("X", 5, 5000, false),
        ("BIG", 6, 12000, false),
    ] {
        let mut c = make_test_card(id, id);
        c.level = Some(level);
        c.dp = Some(dp);
        if guard {
            c.traits = vec!["Guarded".into()];
        }
        b = b.add_card(c);
    }
    if ys.contains(&SEC) {
        b = b.security(1, &["SEC"]);
    }
    let mut r = b
        .deck(0, &["L3"; 20])
        .deck(1, &["L3"; 20])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
fn fire(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}
fn pick(r: &mut DebugRunner, player: u8, count: usize, optional: bool, index: usize) {
    let s = r
        .game
        .pending_selection
        .as_ref()
        .expect("real player choice");
    assert_eq!(s.selecting_player, player);
    assert_eq!(s.is_optional, optional);
    let ids = s.valid_action_ids.clone();
    let choices: Vec<_> = ids.iter().copied().filter(|a| *a != PASS).collect();
    assert_eq!(choices.len(), count, "{ids:?}");
    let mask = build_action_mask(&r.game, player);
    let wrong = build_action_mask(&r.game, 1 - player);
    for (a, v) in mask.iter().enumerate() {
        assert_eq!(
            *v,
            if choices.contains(&(a as u16)) || (optional && a == PASS as usize) {
                1.0
            } else {
                0.0
            },
            "mask {a}"
        );
    }
    assert!(wrong.iter().all(|v| *v == 0.0));
    r.execute_action(player, choices[index]).unwrap();
}
fn grant(y: &str, id: &str) -> (DebugRunner, PermanentHandle) {
    let mut r = runner(&[y]);
    let h = r.place_stack(0, &["L3", "L4", id]);
    r.place_stack(0, &["L3", "L4", id]);
    fire(&mut r, h);
    (r, h)
}
fn attack(r: &mut DebugRunner, y: &str, player: u8) {
    // Each attacker is registered with its own literal YAML before the runner starts.
    let id = y.lines().find_map(|l| l.strip_prefix("card: ")).unwrap();
    let h = r.place_on_field(player, id, None);
    fire(r, h);
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().valid_action_ids,
        vec![
            digimon_engine::action::space::encode_attack(0, 0),
            digimon_engine::action::space::encode_attack(0, 1)
        ]
    );
    pick(r, player, 2, false, 0);
}

const NARROW: &str = r#"
card: NARROW
name: NARROW
kind: digimon
level: 5
color: [black]
cost: 4
dp: 5000
effects:
  - when: on_play
    process:
      - grant_narrow_opponent_effect_protection: { target: source, expiry: end_of_opponents_turn }
"#;

const STACK: &str = r#"
card: STACK
name: STACK
kind: digimon
level: 5
color: [black]
cost: 4
dp: 5000
effects:
  - when: on_play
    process:
      - grant_stack_protection_from_opponent_effects: { target: source, expiry: end_of_opponents_turn }
"#;

const MINUS: &str = r#"
card: MINUS
name: MINUS
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - add_dp_modifier: { target: t, value: -3000, expiry: end_of_turn }
"#;

const PLUS: &str = r#"
card: PLUS
name: PLUS
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - add_dp_modifier: { target: t, value: 3000, expiry: end_of_turn }
"#;

const DEDI: &str = r#"
card: DEDI
name: DEDI
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - de_digivolve: { target: t, amount: 1 }
"#;

const KILL: &str = r#"
card: KILL
name: KILL
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - delete_permanent: { target: t }
"#;

const BOUNCE: &str = r#"
card: BOUNCE
name: BOUNCE
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - return_to_hand: { target: t }
"#;

const DECK: &str = r#"
card: DECK
name: DECK
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - return_to_deck: { target: t, position: bottom }
"#;

const ALL: &str = r#"
card: ALL
name: ALL
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - trash_all_sources: { target: t }
"#;

const TOP: &str = r#"
card: TOP
name: TOP
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - trash_top_source: { target: t }
"#;

const SOURCES_DECK: &str = r#"
card: SOURCES_DECK
name: SOURCES_DECK
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - return_all_sources_to_deck: { target: t, position: bottom }
"#;

const PICK_TRASH: &str = r#"
card: PICK_TRASH
name: PICK_TRASH
kind: tamer
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - select_opponent_permanent: { bind_as: t, filter: { kind: digimon }, prompt: Choose stack }
      - select_opponent_sources:
          target: t
          min: 1
          max: 1
          bind_as: s
          prompt: Choose source
          then:
            - trash_selected_sources: { source_refs: s }
"#;

const PICK_HAND: &str = r#"
card: PICK_HAND
name: PICK_HAND
kind: tamer
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - select_opponent_permanent: { bind_as: t, filter: { kind: digimon }, prompt: Choose stack }
      - select_opponent_sources:
          target: t
          min: 1
          max: 1
          bind_as: s
          prompt: Choose source
          then:
            - return_selected_sources_to_hand: { source_refs: s }
"#;

const LITERAL: &str = r#"
card: LITERAL
name: LITERAL
kind: tamer
color: [yellow]
cost: 2
effects:
  - kind: aura
    dp_modifier: -3000
    target: { of: opponent, zone: [battle_area], kind: digimon }
"#;

const FORMULA: &str = r#"
card: FORMULA
name: FORMULA
kind: tamer
color: [yellow]
cost: 2
effects:
  - kind: aura
    dp_modifier_fn: { base: -3000, per: material_count, delta: 0 }
    target: { of: opponent, zone: [battle_area], kind: digimon }
"#;

const PAYER: &str = r#"
card: PAYER
name: Payer
kind: tamer
color: [black]
cost: 3
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    once_per_turn: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - trait_has: Guarded
        - none_of: [{ replacement_cause: battle }]
    process:
      - suspend_and_cancel_all_matching_leaves:
          target: source
          filter:
            all_of:
              - replacement_subject_is_mine: true
              - trait_has: Guarded
              - none_of: [{ replacement_cause: battle }]
"#;

fn protected(y: &str, id: &str, extra: &str) -> (DebugRunner, PermanentHandle) {
    let mut r = runner(&[y, extra]);
    let h = r.place_stack(0, &["L3", "L4", id]);
    r.place_stack(0, &["L3", "L4", id]);
    fire(&mut r, h);
    (r, h)
}
#[test]
fn narrow_opponent_formula_b4() {
    let (mut r, h) = protected(NARROW, "NARROW", FORMULA);
    r.place_on_field(1, "FORMULA", None);
    assert_eq!(r.game.effective_dp(h), Some(5000));
    assert_eq!(
        r.game.effective_dp(PermanentHandle {
            player: 0,
            index: 1
        }),
        Some(2000)
    );
}
#[test]
fn narrow_literal_b3() {
    let (mut r, h) = protected(NARROW, "NARROW", LITERAL);
    r.place_on_field(1, "LITERAL", None);
    r.game.drain_effect_queue();
    assert_eq!(r.game.effective_dp(h), Some(5000));
    assert_eq!(
        r.game.effective_dp(PermanentHandle {
            player: 0,
            index: 1
        }),
        Some(2000)
    );
}
#[test]
fn narrow_direct_b1_own_n1_positive_n7() {
    for (player, y, want) in [
        (1, MINUS, 5000),
        (0, MINUS, 2000),
        (1, PLUS, 8000),
        (0, PLUS, 8000),
    ] {
        let (mut r, h) = protected(NARROW, "NARROW", y);
        attack(&mut r, y, player);
        assert_eq!(r.game.effective_dp(h), Some(want));
    }
}
#[test]
fn narrow_de_digivolve_scope_b6_n2() {
    for player in [0, 1] {
        let (mut r, h) = protected(NARROW, "NARROW", DEDI);
        attack(&mut r, DEDI, player);
        assert_eq!(
            r.game.players[0].battle_area[h.index as usize]
                .card_sources
                .len(),
            if player == 0 { 2 } else { 3 }
        );
    }
}
#[test]
fn narrow_does_not_block_bounce_or_source_trash() {
    for y in [BOUNCE, ALL] {
        let (mut r, _) = protected(NARROW, "NARROW", y);
        attack(&mut r, y, 1);
        if y == BOUNCE {
            assert_eq!(r.battle_area_size(0), 1);
            assert_eq!(r.game.players[0].hand.len(), 1);
        } else {
            assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
            assert_eq!(r.game.players[0].trash.len(), 2);
        }
    }
}
#[test]
fn narrow_and_stack_do_not_block_deletion_or_battle() {
    for (y, id) in [(NARROW, "NARROW"), (STACK, "STACK")] {
        let (mut r, _) = protected(y, id, KILL);
        attack(&mut r, KILL, 1);
        assert_eq!(r.battle_area_size(0), 1);
        assert_eq!(r.game.players[0].trash.len(), 3);
        let (mut r, h) = grant(y, id);
        let big = r.place_on_field(1, "BIG", Some(0));
        r.battle_digimon(big, h);
        assert_eq!(r.battle_area_size(0), 1);
        assert_eq!(r.game.players[0].trash.len(), 3);
    }
}
#[test]
fn stack_s1_s2_s4_s5_s6_s9() {
    for y in [ALL, TOP, DEDI, BOUNCE, DECK, SOURCES_DECK] {
        let (mut r, h) = protected(STACK, "STACK", y);
        let deck = r.game.players[0].deck.len();
        attack(&mut r, y, 1);
        assert_eq!(r.battle_area_size(0), 2, "{y}");
        assert_eq!(
            r.game.players[0].battle_area[h.index as usize]
                .card_sources
                .len(),
            3,
            "{y}"
        );
        assert!(r.game.players[0].trash.is_empty());
        assert!(r.game.players[0].hand.is_empty());
        assert_eq!(r.game.players[0].deck.len(), deck);
    }
}
#[test]
fn stack_s3_selected_trash_s8_selected_return() {
    for y in [PICK_TRASH, PICK_HAND] {
        let (mut r, h) = protected(STACK, "STACK", y);
        attack(&mut r, y, 1);
        pick(&mut r, 1, 2, false, 0);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(
            r.game.players[0].battle_area[h.index as usize]
                .card_sources
                .len(),
            3
        );
        assert!(r.game.players[0].hand.is_empty());
        assert!(r.game.players[0].trash.is_empty());
    }
}
#[test]
fn stack_own_source_trash_and_bounce() {
    for y in [ALL, BOUNCE] {
        let (mut r, _) = protected(STACK, "STACK", y);
        attack(&mut r, y, 0);
        if y == ALL {
            assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
        } else {
            assert_eq!(r.battle_area_size(0), 2);
            assert_eq!(r.game.players[0].hand.len(), 1);
        }
    }
}
fn plural() -> (DebugRunner, Vec<PermanentHandle>, PermanentHandle) {
    let mut r = runner(&[PAYER]);
    let hs = ["G1", "G2", "X"]
        .iter()
        .map(|id| r.place_on_field(0, id, None))
        .collect();
    let p = r.place_on_field(0, "PAYER", None);
    (r, hs, p)
}
fn delete(r: &mut DebugRunner, hs: Vec<PermanentHandle>, cause: ReplacementCause, player: u8) {
    r.game.set_effect_source_player_for_test(Some(player));
    r.game.delete_permanents_batch(hs, cause);
}
#[test]
fn plural_accept_saves_two_not_x() {
    let (mut r, hs, p) = plural();
    delete(&mut r, hs, ReplacementCause::OpponentEffect, 1);
    pick(&mut r, 0, 1, true, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.battle_area_size(0), 3);
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .find(|x| x.top_card().card_id(&r.game.card_data) == "PAYER")
            .unwrap()
            .is_suspended
    );
    let _ = p;
}
#[test]
fn plural_pass_leaves_entire_batch() {
    let (mut r, hs, _) = plural();
    delete(&mut r, hs, ReplacementCause::OpponentEffect, 1);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    r.execute_action(0, PASS).unwrap();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.game.players[0].trash.len(), 3);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
}
#[test]
fn plural_ungated_unpayable_not_offered() {
    for cannot in [false, true] {
        let (mut r, hs, p) = plural();
        if cannot {
            r.game.modifiers.add(
                p,
                ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 0),
            );
        } else {
            r.game.players[0].battle_area[p.index as usize].is_suspended = true;
        }
        delete(&mut r, hs, ReplacementCause::OpponentEffect, 1);
        assert!(
            r.game.pending_selection.is_none(),
            "CR 15-7-4: unpayable cannot be chosen"
        );
        assert_eq!(r.game.players[0].trash.len(), 3);
    }
}
#[test]
fn plural_own_effect_allowed_battle_excluded() {
    for cause in [ReplacementCause::OwnEffect, ReplacementCause::Battle] {
        let (mut r, hs, _) = plural();
        delete(&mut r, hs, cause, 0);
        if cause == ReplacementCause::OwnEffect {
            pick(&mut r, 0, 1, true, 0);
            assert_eq!(r.game.players[0].trash.len(), 1);
        } else {
            assert!(r.game.pending_selection.is_none());
            assert_eq!(r.game.players[0].trash.len(), 3);
        }
    }
}

const SEC: &str = r#"
card: SEC
name: SEC
kind: option
color: [purple]
cost: 2
effects:
  - when: on_security
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - add_dp_modifier: { target: t, value: -3000, expiry: end_of_turn }
"#;

const TOP_DECK: &str = r#"
card: TOP_DECK
name: DECK
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - return_top_stacked_to_deck: { targets: t, count: 1, position: bottom, prompt: Order the returned cards }
"#;

const OWN_PICK: &str = r#"
card: OWN_PICK
name: OWN_PICK
kind: tamer
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent: { bind_as: t, filter: { kind: digimon }, prompt: Choose stack }
      - select_own_sources:
          target: t
          min: 1
          max: 1
          bind_as: s
          prompt: Choose source
          then:
            - trash_selected_sources: { source_refs: s }
"#;

const OWN_HAND: &str = r#"
card: OWN_HAND
name: OWN_HAND
kind: tamer
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent: { bind_as: t, filter: { kind: digimon }, prompt: Choose stack }
      - select_own_sources:
          target: t
          min: 1
          max: 1
          bind_as: s
          prompt: Choose source
          then:
            - return_selected_sources_to_hand: { source_refs: s }
"#;

const BOUNCE_ALL: &str = r#"
card: BOUNCE_ALL
name: Bounce all
kind: tamer
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - return_all_to_hand: { over: { of: opponent, zone: [battle_area], kind: digimon } }
"#;

const OBS: &str = r#"
card: OBS
name: Observer
kind: tamer
color: [black]
cost: 2
effects:
  - when: on_suspend
    trigger_if: { event_target_name_contains: Payer }
    process:
      - gain_memory: 1
"#;

const CHOSEN: &str = r#"
card: CHOSEN
name: Payer
kind: tamer
color: [black]
cost: 3
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    once_per_turn: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - trait_has: Guarded
        - none_of: [{ replacement_cause: battle }]
        - any_permanent: { of: you, kind: digimon, trait_has: Guarded, is_unsuspended: true, can_change_orientation: true }
    process:
      - select_own_permanent:
          bind_as: chosen
          prompt: Choose a payable Guarded Digimon
          optional: true
          filter: { kind: digimon, trait_has: Guarded, is_unsuspended: true, can_change_orientation: true }
      - suspend_and_cancel_all_matching_leaves:
          target: chosen
          filter:
            all_of:
              - replacement_subject_is_mine: true
              - trait_has: Guarded
              - none_of: [{ replacement_cause: battle }]
"#;

const GATED: &str = r#"
card: GATED
name: Payer
kind: tamer
color: [black]
cost: 3
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    once_per_turn: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - trait_has: Guarded
        - none_of: [{ replacement_cause: battle }]
        - source_is_unsuspended: true
        - any_permanent: { of: you, is_source: true, can_change_orientation: true }
    process:
      - suspend_and_cancel_all_matching_leaves:
          target: source
          filter:
            all_of:
              - replacement_subject_is_mine: true
              - trait_has: Guarded
              - none_of: [{ replacement_cause: battle }]
"#;

const SUSPEND: &str = r#"
card: SUSPEND
name: SUSPEND
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - suspend: { target: t }
"#;

const CANNOT_ATTACK: &str = r#"
card: CANNOT_ATTACK
name: CANNOT_ATTACK
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - add_modifier: { target: t, modifier: CannotAttack, value: 0, expiry: end_of_turn }
"#;

const TUCK: &str = r#"
card: TUCK
name: TUCK
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - place_as_bottom_source: { source: t, target: source }
"#;

#[test]
fn narrow_security_b2() {
    let (mut r, h) = protected(NARROW, "NARROW", SEC);
    r.game.players[0].battle_area[h.index as usize].turn_played = 0;
    let action = digimon_engine::action::space::encode_attack(
        h.index.into(),
        digimon_engine::action::space::SECURITY_TARGET,
    );
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    pick(&mut r, 1, 2, false, 0);
    assert_eq!(r.game.effective_dp(h), Some(5000));
    assert!(r.game.pending_selection.is_none());
}
#[test]
fn stack_s7_top_stacked_return() {
    let (mut r, h) = protected(STACK, "STACK", TOP_DECK);
    attack(&mut r, TOP_DECK, 1);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        r.game.players[0].battle_area[h.index as usize]
            .card_sources
            .len(),
        3
    );
}
#[test]
fn own_selected_source_trash_and_return_allowed() {
    for y in [OWN_PICK, OWN_HAND] {
        let (mut r, h) = protected(STACK, "STACK", y);
        attack(&mut r, y, 0);
        pick(&mut r, 0, 2, false, 0);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(
            r.game.players[0].battle_area[h.index as usize]
                .card_sources
                .len(),
            2
        );
        assert_eq!(
            r.game.players[0].hand.len() + r.game.players[0].trash.len(),
            1
        );
    }
}
#[test]
fn grants_expire_after_opponents_turn() {
    for (y, id, atk) in [(NARROW, "NARROW", MINUS), (STACK, "STACK", ALL)] {
        let (mut r, h) = protected(y, id, atk);
        r.pass_turn();
        assert_eq!(r.game.turn_player(), 1);
        r.game.enter_main_phase();
        attack(&mut r, atk, 1);
        assert_eq!(r.game.effective_dp(h), Some(5000));
        assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 3);
        r.pass_turn();
        assert_eq!(r.game.turn_player(), 0);
        r.game.enter_main_phase();
        attack(&mut r, atk, 1);
        if id == "NARROW" {
            assert_eq!(r.game.effective_dp(h), Some(2000));
        } else {
            assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
        }
    }
}
#[test]
fn plural_opt_not_merely_suspension() {
    let (mut r, hs, _) = plural();
    delete(&mut r, hs, ReplacementCause::OpponentEffect, 1);
    pick(&mut r, 0, 1, true, 0);
    r.game.players[0].battle_area[2].is_suspended = false;
    let hs = vec![
        PermanentHandle {
            player: 0,
            index: 0,
        },
        PermanentHandle {
            player: 0,
            index: 1,
        },
    ];
    delete(&mut r, hs, ReplacementCause::OpponentEffect, 1);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[0].trash.len(), 3);
}
#[test]
fn plural_one_suspend_observer() {
    let mut r = runner(&[PAYER, OBS]);
    let payer = r.place_on_field(0, "PAYER", None);
    r.place_on_field(0, "OBS", None);
    let g1 = r.place_on_field(0, "G1", None);
    let g2 = r.place_on_field(0, "G2", None);
    let x = r.place_on_field(0, "X", None);
    let memory = r.memory();
    delete(&mut r, vec![g1, g2, x], ReplacementCause::OpponentEffect, 1);
    pick(&mut r, 0, 1, true, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), memory + 1);
    assert!(r.game.players[0].battle_area[payer.index as usize].is_suspended);
    assert_eq!(r.game.players[0].trash.len(), 1);
}
#[test]
fn plural_simultaneous_bounce() {
    let mut r = runner(&[PAYER, BOUNCE_ALL]);
    r.place_on_field(0, "PAYER", None);
    r.place_on_field(0, "G1", None);
    r.place_on_field(0, "G2", None);
    r.place_on_field(0, "X", None);
    let a = r.place_on_field(1, "BOUNCE_ALL", None);
    fire(&mut r, a);
    pick(&mut r, 0, 1, true, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.battle_area_size(0), 3);
    assert_eq!(r.game.players[0].hand.len(), 1);
    assert_eq!(r.game.players[0].hand[0].card_id(&r.game.card_data), "X");
}
#[test]
fn plural_authored_gates_unpayable() {
    for cannot in [false, true] {
        let mut r = runner(&[GATED]);
        let p = r.place_on_field(0, "GATED", None);
        let g = r.place_on_field(0, "G1", None);
        if cannot {
            r.game.modifiers.add(
                p,
                ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 0),
            );
        } else {
            r.game.players[0].battle_area[0].is_suspended = true;
        }
        delete(&mut r, vec![g], ReplacementCause::OpponentEffect, 1);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.game.players[0].trash.len(), 1);
    }
}
#[test]
fn plural_chosen_payer_excludes_unpayable_and_unqualified() {
    let mut r = runner(&[CHOSEN]);
    r.place_on_field(0, "CHOSEN", None);
    let g1 = r.place_on_field(0, "G1", None);
    let g2 = r.place_on_field(0, "G2", None);
    let suspended = r.place_on_field(0, "G1", None);
    r.game.players[0].battle_area[suspended.index as usize].is_suspended = true;
    let blocked = r.place_on_field(0, "G2", None);
    r.game.modifiers.add(
        blocked,
        ModifierEntry::simple(ModifierType::CannotSuspend, 0, Expiry::Permanent, 0),
    );
    r.place_on_field(0, "X", None);
    delete(&mut r, vec![g1, g2], ReplacementCause::OpponentEffect, 1);
    pick(&mut r, 0, 1, true, 0);
    let ids = &r.game.pending_selection.as_ref().unwrap().valid_action_ids;
    assert_eq!(
        ids,
        &vec![
            digimon_engine::action::space::encode_attack(0, 1),
            digimon_engine::action::space::encode_attack(0, 2)
        ]
    );
    pick(&mut r, 0, 2, true, 1);
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.players[0].battle_area[2].is_suspended);
    assert!(!r.game.players[0].battle_area[1].is_suspended);
    assert!(r.game.players[0].trash.is_empty());
}

#[test]
fn narrow_adjacent_deck_suspend_and_restriction() {
    for y in [DECK, SUSPEND, CANNOT_ATTACK] {
        let (mut r, h) = protected(NARROW, "NARROW", y);
        attack(&mut r, y, 1);
        assert!(r.game.pending_selection.is_none());
        if y == DECK {
            assert_eq!(r.battle_area_size(0), 1);
        } else if y == SUSPEND {
            assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
        } else {
            assert!(!r
                .game
                .modifiers
                .get(h, ModifierType::CannotAttack)
                .is_empty());
        }
    }
}
#[test]
fn stack_allows_dp_suspension_and_source_placement() {
    for y in [MINUS, SUSPEND, TUCK] {
        let (mut r, h) = protected(STACK, "STACK", y);
        attack(&mut r, y, 1);
        assert!(r.game.pending_selection.is_none());
        if y == MINUS {
            assert_eq!(r.game.effective_dp(h), Some(2000));
        } else if y == SUSPEND {
            assert!(r.game.players[0].battle_area[h.index as usize].is_suspended);
        } else {
            assert_eq!(r.battle_area_size(0), 1);
            assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 2);
            assert_eq!(r.game.players[0].trash.len(), 2);
        }
    }
}

const FORMULA_OWN: &str = r#"
card: FORMULA_OWN
name: FORMULA_OWN
kind: tamer
color: [yellow]
cost: 2
effects:
  - kind: aura
    dp_modifier_fn: { base: -3000, per: material_count, delta: 0 }
    target: { of: you, zone: [battle_area], kind: digimon }
"#;

const FORMULA_PLUS: &str = r#"
card: FORMULA_PLUS
name: FORMULA_PLUS
kind: tamer
color: [yellow]
cost: 2
effects:
  - kind: aura
    dp_modifier_fn: { base: 3000, per: material_count, delta: 0 }
    target: { of: opponent, zone: [battle_area], kind: digimon }
"#;

const OWN_COST: &str = r#"
card: OWN_COST
name: STACK
kind: digimon
level: 5
color: [black]
cost: 4
dp: 5000
effects:
  - when: on_play
    process:
      - grant_stack_protection_from_opponent_effects: { target: source, expiry: end_of_opponents_turn }
  - when: main_on_field
    optional: true
    once_per_turn: true
    process:
      - select_own_sources:
          target: source
          min: 1
          max: 1
          bind_as: fuel
          prompt: Pay by trashing a source
          then:
            - trash_selected_sources_as_cost: { source_refs: fuel, min_count: 1 }
            - gain_memory: 1
"#;

const DEDI_GUARDED: &str = r#"
card: DEDI_GUARDED
name: DEDI_GUARDED
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, trait_has: Guarded }, prompt: Choose a protected stack }
      - de_digivolve: { target: t, amount: 1 }
"#;

#[test]
fn formula_own_negative_and_opponent_positive_apply() {
    for (y, id, player, expected) in [
        (FORMULA_OWN, "FORMULA_OWN", 0, 2000),
        (FORMULA_PLUS, "FORMULA_PLUS", 1, 8000),
    ] {
        let (mut r, h) = protected(NARROW, "NARROW", y);
        r.place_on_field(player, id, None);
        assert_eq!(r.game.effective_dp(h), Some(expected));
    }
}
#[test]
fn plural_de_digivolve_adjacent_does_not_offer() {
    let mut r = runner(&[PAYER, DEDI_GUARDED]);
    r.place_on_field(0, "PAYER", None);
    let h = r.place_stack(0, &["L3", "G1"]);
    r.place_stack(0, &["L3", "G2"]);
    let a = r.place_on_field(1, "DEDI_GUARDED", None);
    fire(&mut r, a);
    pick(&mut r, 1, 2, false, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        r.game.players[0].battle_area[h.index as usize]
            .card_sources
            .len(),
        1
    );
    assert!(!r.game.players[0].battle_area[0].is_suspended);
}
#[test]
fn stack_own_source_activation_cost_payable() {
    let (mut r, h) = grant(OWN_COST, "OWN_COST");
    let action = 1002;
    assert_eq!(build_action_mask(&r.game, 0)[action], 1.0);
    let memory = r.memory();
    r.game.decode_action(action as u16, 0);
    pick(&mut r, 0, 2, false, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), memory + 1);
    assert_eq!(
        r.game.players[0].battle_area[h.index as usize]
            .card_sources
            .len(),
        2
    );
}

const LINK_FORMULA: &str = r#"
card: LINK_FORMULA
name: LINK_FORMULA
kind: digimon
level: 3
dp: 2000
color: [yellow]
cost: 2
effects:
  - scope: linked
    kind: aura
    dp_modifier_fn: { base: -3000, per: material_count, delta: 0 }
    target: { of: opponent, zone: [battle_area], kind: digimon }
"#;

const SECURITY_MOVE: &str = r#"
card: SECURITY_MOVE
name: SECURITY_MOVE
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK] }, prompt: Choose a protected stack }
      - place_on_security: { source: { permanent: t }, to_owner: true, position: bottom }
"#;

const THRESHOLD: &str = r#"
card: THRESHOLD
name: THRESHOLD
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: t, filter: { kind: digimon, name_in: [NARROW, STACK], dp_lte: 5000 }, prompt: Choose a protected stack }
      - delete_permanent: { target: t }
"#;

#[test]
fn narrow_linked_formula_b4() {
    let (mut r, h) = protected(NARROW, "NARROW", LINK_FORMULA);
    let host = r.place_on_field(1, "BIG", None);
    r.push_linked_owned(host, "LINK_FORMULA", 1);
    assert_eq!(r.game.effective_dp(h), Some(5000));
    assert_eq!(
        r.game.effective_dp(PermanentHandle {
            player: 0,
            index: 1
        }),
        Some(2000)
    );
}
#[test]
fn narrow_formula_immunity_expires_live() {
    let (mut r, h) = protected(NARROW, "NARROW", FORMULA);
    r.place_on_field(1, "FORMULA", None);
    assert_eq!(r.game.effective_dp(h), Some(5000));
    r.pass_turn();
    r.game.enter_main_phase();
    assert_eq!(r.game.effective_dp(h), Some(5000));
    r.pass_turn();
    r.game.enter_main_phase();
    assert_eq!(r.game.effective_dp(h), Some(2000));
}
#[test]
fn stack_does_not_block_security_placement() {
    let (mut r, _) = protected(STACK, "STACK", SECURITY_MOVE);
    attack(&mut r, SECURITY_MOVE, 1);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.game.players[0].security.len(), 1);
    assert_eq!(r.game.players[0].trash.len(), 2);
}
#[test]
fn narrow_threshold_deletion_is_not_dp_reduction() {
    let (mut r, _) = protected(NARROW, "NARROW", THRESHOLD);
    attack(&mut r, THRESHOLD, 1);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.game.players[0].trash.len(), 3);
}

#[test]
fn stack_source_return_expiry_and_own_deck() {
    let (mut r, h) = protected(STACK, "STACK", SOURCES_DECK);
    r.pass_turn();
    r.game.enter_main_phase();
    attack(&mut r, SOURCES_DECK, 1);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 3);
    r.pass_turn();
    r.game.enter_main_phase();
    let deck = r.game.players[0].deck.len();
    attack(&mut r, SOURCES_DECK, 1);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        r.game.players[0].battle_area[h.index as usize]
            .card_sources
            .len(),
        1
    );
    assert_eq!(r.game.players[0].deck.len(), deck + 2);
    let (mut r, _) = protected(STACK, "STACK", DECK);
    let deck = r.game.players[0].deck.len();
    attack(&mut r, DECK, 0);
    assert_eq!(r.battle_area_size(0), 2);
    assert_eq!(r.game.players[0].deck.len(), deck + 1);
    assert_eq!(r.game.players[0].trash.len(), 2);
}
