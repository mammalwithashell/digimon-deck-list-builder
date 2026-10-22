//! Cost, result and event constructs: CR 4.3 Â§Â§15-7-1/4, 15-8-5, 15-5-2/3, 2-3, 11-6 and 15-16-13-2.
//! Card-text idioms: EX4-073 / EX7-073 (count), EX13-045 / LM-020 (kind).
//! Tail idioms: EX12-036 / EX13-018 (ally play); EX13-018 / EX13-039 (event source identity).
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    CardKind, EffectTiming, Expiry, Keyword, TriggerSource,
};
fn runner(y: &str) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(y)
        .unwrap()
        .from_dsl_yaml(ADDED_OBSERVER)
        .unwrap()
        .from_dsl_yaml(YOUR_MAIN)
        .unwrap();
    for (id, name, kind) in [
        ("FUEL", "Fuel", CardKind::Digimon),
        ("PLAIN", "Plain", CardKind::Digimon),
        ("OPT", "Option", CardKind::Option),
        ("TAM", "Tamer", CardKind::Tamer),
        ("DUAL", "Dual", CardKind::Dual),
        ("AGU", "Agumon", CardKind::Digimon),
        ("GAB", "Gabumon", CardKind::Digimon),
        ("PAT", "Patamon", CardKind::Digimon),
        ("ALIAS", "Koromon", CardKind::Digimon),
        ("WING", "Wingdramon", CardKind::Digimon),
        ("IND", "Indramon", CardKind::Digimon),
        ("SUKA", "Sukamon", CardKind::Digimon),
    ] {
        let mut c = make_test_card(id, name);
        c.card_kind = kind;
        if id == "FUEL" {
            c.traits = vec!["Fuel".into()];
        }
        if id == "ALIAS" {
            c.also_treated_as = vec!["Gabumon".into()];
        }
        b = b.add_card(c);
    }
    let mut r = b
        .deck(0, &["PLAIN"; 20])
        .deck(1, &["PLAIN"; 20])
        .memory(0)
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
fn mask(r: &DebugRunner, ids: &[u16], optional: bool) {
    let s = r.game.pending_selection.as_ref().expect("selection");
    assert_eq!(s.is_optional, optional);
    let mut got = s.valid_action_ids.clone();
    got.sort();
    let mut want = ids.to_vec();
    if optional
        && !matches!(
            s.kind,
            digimon_engine::selection::SelectionKind::SourceMulti { .. }
                | digimon_engine::selection::SelectionKind::PlayCostBudget { .. }
        )
    {
        want.retain(|a| *a != PASS);
    }
    want.sort();
    assert_eq!(got, want);
    let m = build_action_mask(&r.game, s.selecting_player);
    for (i, v) in m.iter().enumerate() {
        assert_eq!(
            *v,
            if ids.contains(&(i as u16)) { 1.0 } else { 0.0 },
            "action {i}"
        );
    }
}
fn pick(r: &mut DebugRunner, a: u16) {
    let p = r.game.pending_selection.as_ref().unwrap().selecting_player;
    r.execute_action(p, a).unwrap();
}
fn first(r: &mut DebugRunner) {
    let a = *r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    pick(r, a);
}

const COST: &str = r#"
card: TST-CT-REDUCER
name: Cost Target Reducer
kind: tamer
color: [red]
cost: 2
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    active_when:
      all_of:
        - your_turn: true
        - cost_target: { trait_has: Probe }
    amount: 1
    summary: "[Your Turn] When you would pay for a [Probe] card, reduce the cost by 1"
"#;

const SOURCE_COST: &str = r#"
card: TST-TSSC
name: Source Cost Probe
kind: digimon
level: 5
color: [red]
cost: 5
dp: 5000
effects:
  - when: on_play
    process:
      - select_own_sources:
          target: source
          min: 0
          max: 2
          filter: { trait_has: Fuel }
          bind_as: paid
          prompt: "Trash 2 [Fuel] digivolution cards"
          then:
            - trash_selected_sources_as_cost: { source_refs: paid, min_count: 2 }
            - draw: { of: you, count: 1 }
      - gain_memory: 1
"#;

const COUNT: &str = r#"
card: TST-BCE
name: Binding Count Probe
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
effects:
  - when: on_play
    process:
      - select_opponent_permanent:
          bind_as: victim
          optional: true
          continue_on_decline: true
          filter: { kind: digimon }
          prompt: "You may choose 1 of your opponent's Digimon"
      - if:
          condition: { binding_count_eq: { binding: victim, n: 1 } }
          then:
            - draw: { of: you, count: 1 }
      - if:
          condition: { binding_count_eq: { binding: victim, n: 0 } }
          then:
            - gain_memory: 1
      - select_own_sources:
          target: source
          min: 0
          max: 2
          bind_as: picked
          prompt: "Choose up to 2 digivolution cards"
          then:
            - if:
                condition: { binding_count_eq: { binding: picked, n: 2 } }
                then:
                  - gain_memory: 2
"#;

const KIND: &str = r#"
card: TST-BCK
name: Binding Kind Probe
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - reveal_top_deck: { of: you, count: 1, bind_as: revealed }
      - if:
          condition: { binding_card_kind: { binding: revealed, kind: option } }
          then:
            - gain_memory: 1
      - if:
          condition: { binding_card_kind: { binding: revealed, kind: digimon } }
          then:
            - gain_memory: 2
      - place_remainder_on_deck: { of: you, position: bottom }
"#;

const NAMES: &str = r#"
card: TST-NNIB
name: Different Names Probe
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - reveal_top_deck: { of: you, count: 2, bind_as: seen }
      - select_hand:
          of: you
          bind_as: pick
          filter: { name_not_in_binding: seen }
          prompt: "Trash 1 card whose name was not revealed"
      - trash_from_hand_by_index: { of: you, hand_index: pick }
      - place_remainder_on_deck: { of: you, position: bottom }
"#;

const OWN_DELETE: &str = r#"
card: TST-EDEL-OWN
name: Own Deletion Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: sac
          filter: { other: true }
          prompt: "Delete 1 of your other permanents"
      - delete_permanent: { target: sac }
      - if:
          condition: { effect_deleted_any_own_digimon: true }
          then:
            - gain_memory: 1
"#;

const OPP_DELETE: &str = r#"
card: TST-EDEL-OPP
name: Opponent Deletion Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_opponent_permanent:
          bind_as: victim
          optional: true
          continue_on_decline: true
          filter: { kind: digimon }
          prompt: "Delete 1 of your opponent's Digimon"
      - delete_permanent: { target: victim }
      - if:
          condition: { effect_deleted_any_opponent_digimon: false }
          then:
            - draw: { of: you, count: 1 }
      - if:
          condition: { effect_deleted_any_opponent_digimon: true }
          then:
            - gain_memory: 1
      - if:
          condition: { effect_deleted_any_own_digimon: true }
          then:
            - gain_memory: 5
"#;

const ADD: &str = r#"
card: TST-ADD-HOST
name: Added Card Probe
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
effects:
  - when: on_play
    process:
      - place_as_bottom_source: { source: { deck_top: you }, target: source, face_down: false }
      - place_as_bottom_source: { source: { deck_top: you }, target: source, face_down: false }
  - when: on_add_digivolution_cards
    trigger_if:
      all_of:
        - event_host_permanent_is_source: true
        - event_added_card_any: { kind: option }
    process:
      - gain_memory: 1
"#;

const EVENT_NAME: &str = r#"
card: TST-ETNC
name: Event Name Probe
kind: tamer
color: [green]
cost: 2
effects:
  - when: on_suspend
    trigger_if:
      all_of:
        - event_target_owner: you
        - event_target_name_contains: dramon
    process:
      - gain_memory: 1
"#;

const ATTACK: &str = r#"
card: TST-DURATK
name: During Attack Probe
kind: digimon
level: 6
color: [red]
cost: 6
dp: 12000
effects:
  - when: when_attacking
    process:
      - if:
          condition: { during_attack: true }
          then:
            - gain_memory: 1
  - when: end_of_attack
    process:
      - if:
          condition: { during_attack: true }
          then:
            - draw: { of: you, count: 1 }
  - when: on_play
    process:
      - if:
          condition: { during_attack: false }
          then:
            - gain_memory: 2
"#;

const OPP_MAIN: &str = r#"
card: TST-SOOMP
name: Opp Main Probe
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: start_of_opponents_main_phase
    process:
      - draw: { of: you, count: 1 }
"#;

fn source_setup(y: &str) -> (DebugRunner, PermanentHandle) {
    let mut r = runner(y);
    let h = r.place_stack(0, &["FUEL", "FUEL", "PLAIN", "TST-TSSC"]);
    fire(&mut r, h);
    assert!(matches!(
        r.game.pending_selection.as_ref().unwrap().kind,
        digimon_engine::selection::SelectionKind::SourceMulti {
            min: 0,
            max: 2,
            picked: 0
        }
    ));
    (r, h)
}
fn source_assert(r: &DebugRunner, h: PermanentHandle, n: usize, trash: usize, hand: usize, m: i16) {
    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        r.game.players[0].battle_area[h.index as usize]
            .card_sources
            .len(),
        n
    );
    assert_eq!(r.trash_size(0), trash);
    assert_eq!(r.hand_size(0), hand);
    assert_eq!(r.memory(), m);
}
#[test]
fn source_cost_full_short_and_decline() {
    for picks in 0..=2 {
        let (mut r, h) = source_setup(SOURCE_COST);
        mask(&r, &[2000, 2001, PASS], true);
        if picks > 0 {
            pick(&mut r, 2000);
            mask(&r, &[2001, PASS], true);
        }
        if picks == 2 {
            pick(&mut r, 2001);
            source_assert(&r, h, 2, 2, 1, 1);
        } else {
            pick(&mut r, PASS);
            source_assert(&r, h, 4, 0, 0, 0);
        }
    }
}
fn protect(r: &mut DebugRunner, h: PermanentHandle, index: usize) {
    use digimon_engine::{
        modifiers::{ModifierEntry, ModifierPayload},
        ModifierType,
    };
    let c = r.game.players[0].battle_area[h.index as usize].card_sources[index].handle();
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotTrash, 0, Expiry::Permanent, 0)
            .with_payload(ModifierPayload::ProtectedSourceCards(vec![c])),
    );
}
#[test]
fn source_cost_refused_pick_is_atomic() {
    let (mut r, h) = source_setup(SOURCE_COST);
    pick(&mut r, 2000);
    protect(&mut r, h, 0);
    pick(&mut r, 2001);
    source_assert(&r, h, 4, 0, 0, 0);
}
#[test]
fn source_cost_below_min_not_offered() {
    let y = SOURCE_COST.replace("min: 0", "min: 2");
    let mut r = runner(&y);
    let h = r.place_stack(0, &["FUEL", "FUEL", "PLAIN", "TST-TSSC"]);
    protect(&mut r, h, 0);
    fire(&mut r, h);
    source_assert(&r, h, 4, 0, 0, 0);
}
#[test]
fn binding_count_single_list_and_decline() {
    for (take, two) in [(true, true), (true, false), (false, true), (false, false)] {
        let mut r = runner(COUNT);
        let h = r.place_stack(0, &["PLAIN", "PLAIN", "TST-BCE"]);
        r.place_on_field(1, "PLAIN", Some(0));
        r.place_on_field(1, "FUEL", Some(0));
        fire(&mut r, h);
        mask(&r, &[100, 101, PASS], true);
        pick(&mut r, if take { 100 } else { PASS });
        mask(&r, &[2000, 2001, PASS], true);
        pick(&mut r, 2000);
        mask(&r, &[2001, PASS], true);
        pick(&mut r, if two { 2001 } else { PASS });
        assert_eq!(r.memory(), i16::from(!take) + 2 * i16::from(two));
        assert_eq!(r.hand_size(0), usize::from(take));
        assert!(r.game.pending_selection.is_none());
    }
}
#[test]
fn own_deletion_completed_and_evade() {
    for (id, evade, expected) in [("PLAIN", false, 1), ("TAM", false, 0), ("PLAIN", true, 0)] {
        let mut r = runner(OWN_DELETE);
        let h = r.place_on_field(0, "TST-EDEL-OWN", Some(0));
        let v = r.place_on_field(0, id, Some(0));
        r.place_on_field(0, "FUEL", Some(0));
        if evade {
            r.game
                .modifiers
                .grant_keyword(v, Keyword::Evade, Expiry::Permanent, 0);
        }
        fire(&mut r, h);
        mask(&r, &[101, 102], false);
        pick(&mut r, 101);
        if evade {
            evade_accept(&mut r, 0);
            assert!(r.game.players[0].battle_area[1].is_suspended);
        }
        assert_eq!(r.memory(), expected);
        assert_eq!(r.trash_size(0), usize::from(!evade));
        assert!(r.game.pending_selection.is_none());
    }
}
#[test]
fn opponent_deletion_completed_declined_and_evade() {
    for mode in 0..3 {
        let mut r = runner(OPP_DELETE);
        let h = r.place_on_field(0, "TST-EDEL-OPP", Some(0));
        let v = r.place_on_field(1, "PLAIN", Some(0));
        r.place_on_field(1, "FUEL", Some(0));
        if mode == 2 {
            r.game
                .modifiers
                .grant_keyword(v, Keyword::Evade, Expiry::Permanent, 1);
        }
        fire(&mut r, h);
        mask(&r, &[100, 101, PASS], true);
        pick(&mut r, if mode == 1 { PASS } else { 100 });
        if mode == 2 {
            evade_accept(&mut r, 1);
            assert!(r.game.players[1].battle_area[0].is_suspended);
        }
        assert_eq!(r.memory(), if mode == 0 { 1 } else { 0 });
        assert_eq!(r.hand_size(0), usize::from(mode != 0));
        assert!(r.game.pending_selection.is_none());
    }
}
#[test]
fn deletion_cost_prevention_not_credited() {
    let mut r = runner(COST_DELETE);
    let h = r.place_on_field(0, "TST-EDEL-OWN", Some(0));
    let v = r.place_on_field(0, "PLAIN", Some(0));
    r.place_on_field(0, "FUEL", Some(0));
    r.game
        .modifiers
        .grant_keyword(v, Keyword::Evade, Expiry::Permanent, 0);
    fire(&mut r, h);
    mask(&r, &[101, 102], false);
    pick(&mut r, 101);
    evade_accept(&mut r, 0);
    assert_eq!(r.memory(), 0);
    assert_eq!(r.trash_size(0), 0);
    assert!(r.game.pending_selection.is_none());
}

fn top(r: &mut DebugRunner, p: u8, id: &str) {
    r.add_to_hand(p, id);
    let c = r.game.players[p as usize].hand.pop().unwrap();
    r.game.players[p as usize].deck.push(c);
}
#[test]
fn binding_card_kind_categories_and_multilist() {
    for (id, want) in [("OPT", 1), ("PLAIN", 2), ("DUAL", 3), ("TAM", 0)] {
        let mut r = runner(KIND);
        top(&mut r, 0, id);
        let h = r.place_on_field(0, "TST-BCK", Some(0));
        fire(&mut r, h);
        assert_eq!(r.memory(), want);
        remainder(&mut r, 1);
        assert!(r.game.pending_selection.is_none());
    }
    let mut r = runner(&KIND.replace("count: 1, bind_as", "count: 2, bind_as"));
    top(&mut r, 0, "OPT");
    top(&mut r, 0, "PLAIN");
    let h = r.place_on_field(0, "TST-BCK", Some(0));
    fire(&mut r, h);
    assert_eq!(r.memory(), 0);
    remainder(&mut r, 2);
}
#[test]
fn names_exclude_printed_and_alias_clashes() {
    for (printed, alias) in [(true, true), (true, false), (false, true), (false, false)] {
        let mut r = runner(NAMES);
        top(&mut r, 0, if printed { "AGU" } else { "PLAIN" });
        top(&mut r, 0, if alias { "GAB" } else { "FUEL" });
        for id in ["AGU", "PAT", "ALIAS"] {
            r.add_to_hand(0, id);
        }
        let h = r.place_on_field(0, "TST-NNIB", Some(0));
        fire(&mut r, h);
        let mut ids = vec![1];
        if !printed {
            ids.push(0);
        }
        if !alias {
            ids.push(2);
        }
        mask(&r, &ids, false);
        pick(&mut r, 1);
        assert_eq!(r.trash_size(0), 1);
        assert_eq!(
            r.game
                .card_data_for_handle(r.game.players[0].trash[0].handle())
                .unwrap()
                .card_id,
            "PAT"
        );
        remainder(&mut r, 2);
        assert!(r.game.pending_selection.is_none());
    }
}
#[test]
fn added_cards_batch_any_and_adjacent_timing() {
    for opt in [true, false] {
        let mut r = runner(ADD);
        top(&mut r, 0, "PLAIN");
        top(&mut r, 0, if opt { "OPT" } else { "PLAIN" });
        let h = r.place_on_field(0, "TST-ADD-HOST", Some(0));
        fire(&mut r, h);
        assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 3);
        assert_eq!(r.memory(), i16::from(opt));
        r.trash_one_source(h);
        assert_eq!(r.memory(), i16::from(opt));
        assert!(r.game.pending_selection.is_none());
    }
    let mut r = runner(OTHER_TIMING);
    let h = r.place_on_field(0, "TST-ADD-HOST", Some(0));
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
    assert_eq!(r.memory(), 0);
}
#[test]
fn event_target_names_and_deleted_snapshot() {
    let mut r = runner(EVENT_NAME);
    r.place_on_field(0, "TST-ETNC", Some(0));
    for (id, want) in [("WING", 1), ("AGU", 1), ("IND", 1)] {
        let h = r.place_on_field(0, id, Some(0));
        r.game.suspend(h);
        r.game.drain_effect_queue();
        assert_eq!(r.memory(), want);
    }
    let mut r = runner(DELETED_NAME);
    r.place_on_field(0, "TST-ETNC", Some(0));
    let h = r.place_on_field(0, "SUKA", Some(0));
    r.game.delete_permanents_batch(
        vec![h],
        digimon_engine::replacement::ReplacementCause::OwnEffect,
    );
    r.game.drain_effect_queue();
    assert_eq!(r.memory(), 1);
    assert_eq!(r.trash_size(0), 1);
}
#[test]
fn opponent_main_phase_owner_and_breeding() {
    for turn in [0, 1] {
        let mut r = runner(OPP_MAIN);
        r.place_on_field(0, "TST-SOOMP", Some(0));
        r.place_on_field(1, "TST-SOOMP", Some(0));
        r.place_in_breeding(0, "TST-SOOMP");
        r.place_in_breeding(1, "TST-SOOMP");
        r.set_first_player(turn);
        r.game.enter_main_phase();
        assert_eq!(r.hand_size(turn), 0);
        assert_eq!(r.hand_size(1 - turn), 1);
        assert!(r.game.pending_selection.is_none());
    }
}
#[test]
fn during_attack_lasts_through_end() {
    let mut r = runner(ATTACK);
    let h = r.place_on_field(0, "TST-DURATK", Some(0));
    r.set_turn(2);
    r.add_to_hand(1, "PLAIN");
    let c = r.game.players[1].hand.pop().unwrap();
    r.game.players[1].security.push(c);
    let attack = digimon_engine::action::space::encode_attack(
        h.index.into(),
        digimon_engine::action::space::SECURITY_TARGET,
    );
    assert_eq!(build_action_mask(&r.game, 0)[attack as usize], 1.0);
    r.game.decode_action(attack, 0);
    while r.game.pending_selection.is_some() {
        let ids = r
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids
            .clone();
        assert!(ids.contains(&PASS));
        pick(&mut r, PASS);
    }
    assert_eq!(r.memory(), 1);
    assert_eq!(r.hand_size(0), 1);
    assert!(r.game.pending_attack.is_none());
    fire(&mut r, h);
    assert_eq!(r.memory(), 3);
    assert!(r.game.pending_attack.is_none());
}

const EVO: &str = r#"
card: EVO
name: Evo
kind: digimon
level: 4
color: [red]
cost: 5
dp: 4000
traits: [Probe]
alt_paths:
  - kind: digivolve
    from: { level_eq: 3, color_is: red }
    cost: 3
"#;
const SOURCE_REDUCER: &str = r#"
card: REDUCER
name: Reducer
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - kind: cost_reduction
    reduction_timing: before_pay_cost
    active_when: {source_is_cost_target_permanent: true}
    amount: 1
"#;
fn cost_runner(y: &str, plain: bool) -> DebugRunner {
    let mut c = make_test_card("BASE", "Base");
    c.level = Some(3);
    c.colors = vec![digimon_engine::CardColor::Red];
    let ev = if plain {
        EVO.replace("traits: [Probe]", "traits: []")
    } else {
        EVO.to_string()
    };
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(y)
        .unwrap()
        .from_dsl_yaml(&ev)
        .unwrap()
        .from_dsl_yaml(ADDED_OBSERVER)
        .unwrap()
        .add_card(c)
        .hand(0, &["EVO"])
        .deck(0, &["BASE"; 20])
        .deck(1, &["BASE"; 20])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}
#[test]
fn cost_target_matches_digivolve_and_play() {
    for plain in [false, true] {
        let mut r = cost_runner(COST, plain);
        r.place_on_field(0, "TST-CT-REDUCER", Some(0));
        let h = r.place_on_field(0, "BASE", Some(0));
        r.game.tick_declarative_effects();
        let a = digimon_engine::action::space::encode_digivolve(0, h.index.into());
        assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
        r.game.decode_action(a, 0);
        assert_eq!(r.memory(), if plain { 2 } else { 3 });
        assert!(r.game.pending_selection.is_none());
    }
    for reducer in [false, true] {
        let mut r = cost_runner(COST, false);
        if reducer {
            r.place_on_field(0, "TST-CT-REDUCER", Some(0));
        }
        r.game.tick_declarative_effects();
        assert_eq!(build_action_mask(&r.game, 0)[0], 1.0);
        r.game.decode_action(0, 0);
        assert_eq!(r.memory(), i16::from(reducer));
    }
}
#[test]
fn source_cost_target_carrier_and_inheritance() {
    for inherited in [false, true] {
        for target in [0, 1] {
            let y = if inherited {
                SOURCE_REDUCER.replace(
                    "reduction_timing:",
                    "scope: inherited\n    reduction_timing:",
                )
            } else {
                SOURCE_REDUCER.into()
            };
            let mut r = cost_runner(&y, true);
            if inherited {
                r.place_stack(0, &["REDUCER", "BASE"]);
            } else {
                r.place_on_field(0, "REDUCER", Some(0));
            }
            r.place_on_field(0, "BASE", Some(0));
            r.game.tick_declarative_effects();
            let a = digimon_engine::action::space::encode_digivolve(0, target);
            assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
            r.game.decode_action(a, 0);
            assert_eq!(r.memory(), if target == 0 { 3 } else { 2 });
            assert!(r.game.pending_selection.is_none());
        }
    }
    let mut r = cost_runner(SOURCE_REDUCER, true);
    r.place_on_field(0, "REDUCER", Some(0));
    r.game.tick_declarative_effects();
    r.game.decode_action(0, 0);
    assert_eq!(r.memory(), 0);
}
const BOUND_DELETE: &str = r#"
card: BOUND
name: Bound deletion
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_opponent_play_cost_budget:
          play_cost_budget: 20
          min_picks: 0
          filter: { kind: digimon }
          bind_as: victims
          prompt: Choose deletion targets
          then:
            - delete_bound_permanents: { binding: victims }
            - if:
                condition: { effect_deleted_any_opponent_digimon: true }
                then: [{gain_memory: 1}]
            - if:
                condition: { effect_deleted_any_opponent_digimon: false }
                then: [{draw: {of: you, count: 1}}]
"#;
#[test]
fn bound_deletion_evade_not_logged() {
    let mut r = runner(BOUND_DELETE);
    let h = r.place_on_field(0, "BOUND", Some(0));
    let v = r.place_on_field(1, "PLAIN", Some(0));
    r.place_on_field(1, "FUEL", Some(0));
    r.game
        .modifiers
        .grant_keyword(v, Keyword::Evade, Expiry::Permanent, 1);
    fire(&mut r, h);
    mask(&r, &[100, 101, PASS], true);
    pick(&mut r, 100);
    mask(&r, &[101, PASS], true);
    pick(&mut r, PASS);
    evade_accept(&mut r, 1);
    assert_eq!(r.memory(), 0);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.trash_size(1), 0);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn deletion_results_include_tokens() {
    for owner in [0, 1] {
        let mut r = runner(if owner == 0 { OWN_DELETE } else { OPP_DELETE });
        let h = r.place_on_field(
            0,
            if owner == 0 {
                "TST-EDEL-OWN"
            } else {
                "TST-EDEL-OPP"
            },
            Some(0),
        );
        let v = digimon_engine::effect_context::EffectContext::new(
            &mut r.game,
            digimon_engine::card_source::CardHandle(0),
            None,
            owner,
        )
        .play_token(owner, "petrification")
        .unwrap();
        r.place_on_field(owner, "FUEL", Some(0));
        fire(&mut r, h);
        pick(&mut r, 100 + u16::from(v.index));
        assert_eq!(r.memory(), 1);
        assert_eq!(
            r.game.players[owner as usize].battle_area.len(),
            if owner == 0 { 2 } else { 1 }
        );
        assert_eq!(r.trash_size(owner), 0);
        assert!(r.game.pending_selection.is_none());
    }
}
#[test]
fn source_payment_two_cards_one_observer_event() {
    let y=format!("{}{}",SOURCE_COST,"\n  - when: on_digivolution_card_trashed\n    trigger_if: { event_host_permanent_is_source: true }\n    process: [{gain_memory: 2}]\n");
    let (mut r, h) = source_setup(&y);
    pick(&mut r, 2000);
    pick(&mut r, 2001);
    source_assert(&r, h, 2, 2, 1, 3);
}
const ADDED_OBSERVER: &str = r#"
card: OBS
name: Addition observer
kind: tamer
color: [red]
cost: 1
effects:
  - when: on_add_digivolution_cards
    process: [{draw: {of: you, count: 1}}]
"#;
#[test]
fn source_added_observer_nonhost_once_and_after_body() {
    let y = ADD
        .replace(
            "  - when: on_add_digivolution_cards",
            "      - gain_memory: 10\n  - when: on_add_digivolution_cards",
        )
        .replace(
            "- event_added_card_any: { kind: option }",
            "- event_added_card_any: { kind: option }\n        - memory_gte: 8",
        );
    let mut r = runner(&y);
    r.game.memory = -2;
    r.place_on_field(0, "OBS", Some(0));
    top(&mut r, 0, "OPT");
    top(&mut r, 0, "OPT");
    let h = r.place_on_field(0, "TST-ADD-HOST", Some(0));
    fire(&mut r, h);
    if let Some(p) = r.game.pending_selection.as_ref() {
        assert_eq!(
            p.kind,
            digimon_engine::selection::SelectionKind::TriggerOrder
        );
        assert_eq!(p.selecting_player, 0);
        let ids = p.valid_action_ids.clone();
        assert_eq!(ids.len(), 2);
        mask(&r, &ids, false);
        first(&mut r);
    }
    assert_eq!(r.memory(), 9);
    assert_eq!(r.hand_size(0), 1);
    assert!(r.game.pending_selection.is_none());
}
#[test]
fn bound_deletion_two_targets_one_evades() {
    let y=BOUND_DELETE.replace("- if:\n                condition: { effect_deleted_any_opponent_digimon: true }", "- if:\n                condition: { effect_deleted_opponent_digimon_dp_gte: 9000 }\n                then: [{gain_memory: 5}]\n            - if:\n                condition: { effect_deleted_any_opponent_digimon: true }");
    let mut r = runner(&y);
    let h = r.place_on_field(0, "BOUND", Some(0));
    let v = r.place_on_field(1, "PLAIN", Some(0));
    r.place_on_field(1, "FUEL", Some(0));
    r.game.modifiers.add(
        v,
        digimon_engine::modifiers::ModifierEntry::simple(
            digimon_engine::ModifierType::ChangeDp,
            10000,
            Expiry::Permanent,
            0,
        ),
    );
    r.game
        .modifiers
        .grant_keyword(v, Keyword::Evade, Expiry::Permanent, 1);
    fire(&mut r, h);
    mask(&r, &[100, 101, PASS], true);
    pick(&mut r, 100);
    mask(&r, &[101, PASS], true);
    pick(&mut r, 101);
    if r.game
        .pending_selection
        .as_ref()
        .is_some_and(|p| p.selecting_player == 0)
    {
        pick(&mut r, PASS);
    }
    evade_accept(&mut r, 1);
    assert_eq!(r.memory(), 1);
    assert_eq!(r.trash_size(1), 1);
    assert_eq!(r.hand_size(0), 0);
    assert!(r.game.pending_selection.is_none());
}

const YOUR_MAIN: &str = r#"
card: YOUR
name: Your main observer
kind: digimon
level: 3
color: [red]
cost: 1
dp: 1000
effects:
  - when: start_of_your_main_phase
    process: [{gain_memory: 1}]
"#;
#[test]
fn main_phase_both_timings_share_pool_turn_player_first() {
    let mut r = runner(OPP_MAIN);
    r.place_on_field(0, "YOUR", Some(0));
    r.place_on_field(0, "YOUR", Some(0));
    r.place_on_field(1, "TST-SOOMP", Some(0));
    r.place_on_field(1, "TST-SOOMP", Some(0));
    r.game.enter_main_phase();
    let p = r.game.pending_selection.as_ref().unwrap();
    assert_eq!(
        p.kind,
        digimon_engine::selection::SelectionKind::TriggerOrder
    );
    assert_eq!(p.selecting_player, 0);
    assert_eq!(r.hand_size(1), 0);
    let ids = p.valid_action_ids.clone();
    assert_eq!(ids.len(), 2);
    mask(&r, &ids, false);
    first(&mut r);
    assert_eq!(r.memory(), 2);
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().selecting_player,
        1
    );
    let ids = r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids
        .clone();
    assert_eq!(ids.len(), 2);
    mask(&r, &ids, false);
    first(&mut r);
    assert_eq!(r.hand_size(1), 2);
    assert!(r.game.pending_selection.is_none());
}
#[test]
fn ordinary_digivolve_never_addition_event() {
    let mut r = cost_runner(COST, true);
    r.place_on_field(0, "OBS", Some(0));
    let h = r.place_on_field(0, "BASE", Some(0));
    let a = digimon_engine::action::space::encode_digivolve(0, h.index.into());
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
    assert_eq!(r.memory(), 2);
    assert_eq!(r.hand_size(0), 1, "digivolution draw only");
    assert!(r.game.pending_selection.is_none());
}
#[test]
fn during_attack_false_in_effect_battle() {
    let mut r = runner(EFFECT_BATTLE);
    let h = r.place_on_field(0, "TST-DURATK", Some(0));
    let v = r.place_on_field(1, "PLAIN", Some(0));
    r.battle_digimon(h, v);
    assert_eq!(r.memory(), 1);
    assert!(r.game.pending_attack.is_none());
    assert!(r.game.pending_selection.is_none());
}

const COST_DELETE: &str = r#"
card: TST-EDEL-OWN
name: Own Deletion Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          bind_as: sac
          filter: { other: true }
          prompt: "Delete 1 of your other permanents"
      - delete_for_cost_reduction: { target: sac }
      - if:
          condition: { effect_deleted_any_own_digimon: true }
          then:
            - gain_memory: 1
"#;

const DELETED_NAME: &str = r#"
card: TST-ETNC
name: Event Name Probe
kind: tamer
color: [green]
cost: 2
effects:
  - when: on_any_deletion
    trigger_if:
      all_of:
        - event_target_owner: you
        - event_target_name_contains: Sukamon
    process:
      - gain_memory: 1
"#;

const OTHER_TIMING: &str = r#"
card: TST-ADD-HOST
name: Added Card Probe
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
effects:
  - when: on_play
    process:
      - place_as_bottom_source: { source: { deck_top: you }, target: source, face_down: false }
      - place_as_bottom_source: { source: { deck_top: you }, target: source, face_down: false }
  - when: when_attacking
    trigger_if:
      all_of:
        - event_added_card_any: { kind: option }
    process:
      - gain_memory: 1
"#;

const EFFECT_BATTLE: &str = r#"
card: TST-DURATK
name: During Attack Probe
kind: digimon
level: 6
color: [red]
cost: 6
dp: 12000
effects:
  - when: end_of_battle
    process:
      - if:
          condition: { during_attack: false }
          then:
            - gain_memory: 1
  - when: end_of_attack
    process:
      - if:
          condition: { during_attack: false }
          then:
            - draw: { of: you, count: 1 }
  - when: on_play
    process:
      - if:
          condition: { during_attack: false }
          then:
            - gain_memory: 2
"#;

const DELETE_REDUCTION: &str = r#"
card: REDUCTION
name: Delete reduction
kind: tamer
color: [red]
cost: 1
effects:
  - kind: cost_reduction
    active_when: { your_turn: true }
    when_any_ally_played: { trait_has: Probe }
    optional: true
    pay_cost:
      - select_own_permanent:
          bind_as: sacrifice
          filter: { kind: digimon }
          prompt: Choose deletion payment
      - delete_for_cost_reduction: { target: sacrifice }
"#;
#[test]
fn deletion_cost_reduction_credits_only_completed_payment() {
    for evade in [false, true] {
        let mut r = cost_runner(DELETE_REDUCTION, false);
        r.place_on_field(0, "REDUCTION", Some(0));
        let v = r.place_on_field(0, "BASE", Some(0));
        r.place_on_field(0, "BASE", Some(0));
        if evade {
            r.game
                .modifiers
                .grant_keyword(v, Keyword::Evade, Expiry::Permanent, 0);
        }
        r.game.tick_declarative_effects();
        assert_eq!(build_action_mask(&r.game, 0)[0], 1.0);
        r.game.decode_action(0, 0);
        assert_eq!(
            r.game.pending_selection.as_ref().unwrap().selecting_player,
            0
        );
        mask(&r, &[30, PASS], true);
        pick(&mut r, 30);
        mask(&r, &[101, 102], false);
        pick(&mut r, 101);
        if evade {
            evade_accept(&mut r, 0);
        }
        assert_eq!(r.memory(), if evade { 0 } else { 3 });
        assert_eq!(r.trash_size(0), usize::from(!evade));
        assert!(r.game.pending_selection.is_none());
    }
}
fn evade_accept(r: &mut DebugRunner, owner: u8) {
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().selecting_player,
        owner
    );
    mask(r, &[59, PASS], true);
    pick(r, 59);
}
#[test]
fn decoy_substitute_is_not_original_effect_deletion() {
    let mut r = runner(OPP_DELETE);
    let h = r.place_on_field(0, "TST-EDEL-OPP", Some(0));
    r.place_on_field(1, "PLAIN", Some(0));
    let decoy = r.place_on_field(1, "FUEL", Some(0));
    r.game
        .modifiers
        .grant_keyword(decoy, Keyword::Decoy(0), Expiry::Permanent, 1);
    fire(&mut r, h);
    mask(&r, &[100, 101, PASS], true);
    pick(&mut r, 100);
    evade_accept(&mut r, 1);
    assert_eq!(r.memory(), 0);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.trash_size(1), 1);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    assert!(r.game.pending_selection.is_none());
}

fn remainder(r: &mut DebugRunner, count: u16) {
    for n in (1..=count).rev() {
        if r.game.pending_selection.is_none() {
            assert_eq!(n, 1);
            break;
        }
        assert!(matches!(
            r.game.pending_selection.as_ref().unwrap().kind,
            digimon_engine::selection::SelectionKind::OrderedPermutation { .. }
        ));
        let ids: Vec<_> = (30..30 + n).collect();
        mask(r, &ids, false);
        pick(r, 30);
    }
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn source_cost_zero_sources_aborts_outer_tail() {
    let mut r = runner(SOURCE_COST);
    let h = r.place_on_field(0, "TST-TSSC", Some(0));
    fire(&mut r, h);
    source_assert(&r, h, 1, 0, 0, 0);
}

#[test]
fn source_trash_never_dispatches_unconditional_addition_observer() {
    let mut r = runner(ADD);
    r.place_on_field(0, "OBS", Some(0));
    let h = r.place_on_field(0, "TST-ADD-HOST", Some(0));
    fire(&mut r, h);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(
        r.game.players[0].battle_area[h.index as usize]
            .card_sources
            .len(),
        3
    );
    r.trash_one_source(h);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(
        r.game.players[0].battle_area[h.index as usize]
            .card_sources
            .len(),
        2
    );
    assert_eq!(r.trash_size(0), 1);
    assert!(r.game.pending_selection.is_none());
}

// EX12-036 / EX13-018: own play observers; EX13-018 / EX13-039:
// event_target_is_source distinguishes the newly played carrier from observers.
const ALLY_TAIL: &str = r#"
card: TST-ALLY-TAIL
name: Ally play observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_ally_played
    process:
      - draw: { of: you, count: 1 }
"#;
const EVENT_SELF_TAIL: &str = r#"
card: TST-ALLY-TAIL
name: Event self observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_ally_played
    trigger_if: { event_target_is_source: true }
    process:
      - draw: { of: you, count: 1 }
"#;
const EVENT_OTHER_TAIL: &str = r#"
card: TST-ALLY-TAIL
name: Event other observer
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_ally_played
    trigger_if: { event_target_is_source: false }
    process:
      - draw: { of: you, count: 1 }
"#;
fn tail_play(r: &mut DebugRunner, player: u8, id: &str) {
    r.set_first_player(player);
    r.game.memory = 5;
    r.add_to_hand(player, id);
    let action = (r.hand_size(player) - 1) as u16;
    assert_eq!(build_action_mask(&r.game, player)[action as usize], 1.0);
    r.game.decode_action(action, player);
    assert!(r.game.pending_selection.is_none());
}
#[test]
fn tail_on_ally_played_own_opponent_and_adjacent_digivolve() {
    for player in [0, 1] {
        let mut r = cost_runner(ALLY_TAIL, true);
        r.place_on_field(0, "TST-ALLY-TAIL", Some(0));
        let before = r.hand_size(0);
        tail_play(&mut r, player, "BASE");
        assert_eq!(r.hand_size(0), before + usize::from(player == 0));
    }
    let mut r = cost_runner(ALLY_TAIL, true);
    r.place_on_field(0, "TST-ALLY-TAIL", Some(0));
    let h = r.place_on_field(0, "BASE", Some(0));
    let before = r.hand_size(0);
    let action = digimon_engine::action::space::encode_digivolve(0, h.index.into());
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    // One evolution card leaves hand and the ordinary evolution bonus draws one.
    assert_eq!(r.hand_size(0), before);
    assert!(r.game.pending_selection.is_none());
}
#[test]
fn tail_event_target_is_source_live_self_other_and_adjacent_timing() {
    for (yaml, self_draw, other_draw) in [(EVENT_SELF_TAIL, 1, 0), (EVENT_OTHER_TAIL, 0, 1)] {
        let mut r = cost_runner(yaml, true);
        let before = r.hand_size(0);
        tail_play(&mut r, 0, "TST-ALLY-TAIL");
        assert_eq!(r.hand_size(0), before + self_draw);
        let before = r.hand_size(0);
        tail_play(&mut r, 0, "BASE");
        assert_eq!(r.hand_size(0), before + other_draw);
    }
    let mut r = cost_runner(EVENT_SELF_TAIL, true);
    r.place_on_field(0, "TST-ALLY-TAIL", Some(0));
    let h = r.place_on_field(0, "BASE", Some(0));
    let before = r.hand_size(0);
    let action = digimon_engine::action::space::encode_digivolve(0, h.index.into());
    assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
    r.game.decode_action(action, 0);
    assert_eq!(r.hand_size(0), before, "only the ordinary evolution draw");
    assert!(r.game.pending_selection.is_none());
}

const EVENT_INHERITED_TAIL: &str = r#"
card: TST-INHERITED-TAIL
name: Inherited event identity
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_suspend
    scope: inherited
    trigger_if: { event_target_is_source: true }
    process:
      - draw: { of: you, count: 1 }
"#;
#[test]
fn tail_event_target_is_source_inherited_physical_carrier() {
    for attack_carrier in [true, false] {
        let mut r = runner(EVENT_INHERITED_TAIL);
        let carrier = r.place_stack(0, &["TST-INHERITED-TAIL", "PLAIN"]);
        let other = r.place_on_field(0, "PLAIN", Some(0));
        r.set_turn(2);
        r.add_to_hand(1, "PLAIN");
        let security = r.game.players[1].hand.pop().unwrap();
        r.game.players[1].security.push(security);
        let attacker = if attack_carrier { carrier } else { other };
        let action = digimon_engine::action::space::encode_attack(
            attacker.index.into(),
            digimon_engine::action::space::SECURITY_TARGET,
        );
        assert_eq!(build_action_mask(&r.game, 0)[action as usize], 1.0);
        r.game.decode_action(action, 0);
        assert_eq!(r.hand_size(0), usize::from(attack_carrier));
        assert!(r.game.pending_selection.is_none());
    }
}

const EVENT_ABSENT_TAIL: &str = r#"
card: TST-ABSENT-TAIL
name: Missing event target
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_lose_security
    trigger_if: { event_target_is_source: true }
    process:
      - draw: { of: you, count: 1 }
  - when: on_lose_security
    trigger_if: { event_target_is_source: false }
    process:
      - gain_memory: 1
"#;
#[test]
fn tail_event_target_is_source_absent_permanent_target() {
    let mut r = runner(EVENT_ABSENT_TAIL);
    r.place_on_field(0, "TST-ABSENT-TAIL", Some(0));
    let card = r.game.players[0].deck[0].handle();
    // A security-card event has a card but no permanent target. This explicit
    // dispatch isolates the missing-target predicate; live dispatch is above.
    r.game.enqueue_triggered(
        EffectTiming::OnLoseSecurity,
        TriggerSource::SecurityRevealed { defender: 0, card },
    );
    r.game.drain_effect_queue();
    assert_eq!(r.hand_size(0), 0, "missing target must reject true");
    assert_eq!(r.memory(), 1, "missing target must satisfy false");
    assert!(r.game.pending_selection.is_none());
}
