//! Identity/state predicates: CR 4.3 §§2-3-1-2, 2-3-4-3, 2-4-3, 2-6-1,
//! 4-2-3-1, 4-6-6-2, 4-7-7/9, 4-13, 11-1-2, 11-2-1/5/7, 15-7-3/4.
//! Card-text idioms: BT16-085 / BT25-052 (names), BT25-092 / P-094 (sources),
//! BT24-037 / EX13-077 (attack permission), EX3-024 / EX13-024 (orientation).
//! `on_field` means battle area (BT21-093), narrower than CR 3-4-6.
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, PASS},
};
use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    CardColor, CardData, CardKind, EffectTiming, Expiry, Keyword, ModifierType, TriggerSource,
};

fn card(
    id: &str,
    name: &str,
    cost: u8,
    kind: CardKind,
    traits: &[&str],
    colors: &[CardColor],
) -> CardData {
    let mut c = make_test_card(id, name);
    c.play_cost = cost.into();
    c.card_kind = kind;
    c.traits = traits.iter().map(|s| s.to_string()).collect();
    c.colors = colors.to_vec();
    if kind != CardKind::Digimon {
        c.level = None;
        c.dp = None;
    }
    c
}
fn runner(y: &str, hand: &[&str]) -> DebugRunner {
    use CardColor::*;
    use CardKind::*;
    let mut b = DebugRunner::builder().from_dsl_yaml(y).unwrap();
    for (id, name, cost, kind, traits, colors) in [
        ("TARGET", "Agumon", 3, Digimon, vec![], vec![Red]),
        ("OTHER", "Agumon", 2, Digimon, vec![], vec![Red]),
        ("TARGETX", "Agumon", 7, Digimon, vec![], vec![Red]),
        ("FIVE", "Galacticmon", 5, Digimon, vec![], vec![Red]),
        ("V", "Veemon", 3, Digimon, vec![], vec![Blue]),
        ("W", "Wormmon", 3, Digimon, vec![], vec![Green]),
        ("EX", "ExVeemon", 3, Digimon, vec![], vec![Blue]),
        ("DEMI", "DemiVeemon", 3, Digimon, vec![], vec![Blue]),
        ("LOWER", "veemon", 3, Digimon, vec![], vec![Blue]),
        ("SAUR", "Dinosaur", 3, Digimon, vec!["Dinosaur"], vec![Red]),
        (
            "PLESIO",
            "Plesiosaur",
            3,
            Digimon,
            vec!["Plesiosaur"],
            vec![Blue],
        ),
        (
            "SAURIAN",
            "Saurian",
            3,
            Digimon,
            vec!["Saurian"],
            vec![Yellow],
        ),
        ("DRAGON", "Dragon", 3, Digimon, vec!["Dragonkin"], vec![Red]),
        ("OPT", "Option", 3, Option, vec![], vec![Red]),
        ("TAMER", "Tamer", 3, Tamer, vec![], vec![Red]),
        ("VEM", "Vemmon", 3, Digimon, vec![], vec![Red]),
        ("TWO", "Two", 3, Digimon, vec![], vec![Red, Blue]),
        (
            "THREE",
            "Three",
            3,
            Digimon,
            vec![],
            vec![Red, Blue, Yellow],
        ),
        ("DUP", "Duplicate", 3, Digimon, vec![], vec![Red, Red]),
    ] {
        b = b.add_card(card(id, name, cost, kind, &traits, &colors));
    }
    for id in ["TST-ALIAS"] {
        let mut c = make_test_card(id, "Flamedramon");
        c.also_treated_as = vec!["Veemon".into()];
        b = b.add_card(c);
    }
    let mut r = b
        .hand(0, hand)
        .deck(0, &["TARGET"; 20])
        .deck(1, &["TARGET"; 20])
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
fn probe(r: &mut DebugRunner) {
    let h = r.place_on_field(0, "PROBE", Some(0));
    fire(r, h);
}
fn answer(r: &mut DebugRunner, expected: &[u16]) {
    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("real optional choice");
    assert!(p.is_optional);
    assert_eq!(p.valid_action_ids, expected);
    assert_eq!(p.selecting_player, 0);
    let mask = build_action_mask(&r.game, 0);
    for a in expected {
        assert_eq!(mask[*a as usize], 1.0);
    }
    assert_eq!(mask[PASS as usize], 1.0);
    r.execute_action(0, *expected.last().unwrap()).unwrap();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 6, "continuation ran after real choice");
}
fn modifier(r: &mut DebugRunner, h: PermanentHandle, m: ModifierType) {
    r.game
        .modifiers
        .add(h, ModifierEntry::simple(m, 0, Expiry::Permanent, 0));
}

const NUMBER: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { card_number_is: TARGET }
      - gain_memory: 1
"#;

const NUMBER_LOWER: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { card_number_is: target }
      - gain_memory: 1
"#;

const NUMBER_GATE: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    condition: { any_permanent: { of: you, zone: [battle_area], card_number_is: TARGET } }
    process: [{ gain_memory: 1 }]
"#;

const NAMES: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { name_in: [Veemon, Wormmon] }
      - gain_memory: 1
"#;

const NAMES_GATE: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    condition: { any_permanent: { of: you, zone: [battle_area], name_in: [Veemon, Wormmon] } }
    process: [{ gain_memory: 1 }]
"#;

const AGUMON_GATE: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    condition: { any_permanent: { of: you, zone: [battle_area], name_in: [Agumon] } }
    process: [{ gain_memory: 1 }]
"#;

const TRAITS: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { trait_contains: saur }
      - gain_memory: 1
"#;

const AQUA: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    condition: { any_permanent: { of: you, zone: [battle_area], trait_contains: AQUA } }
    process: [{ gain_memory: 1 }]
"#;

const DINO: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { trait_contains: Dino }
      - gain_memory: 1
"#;

const DINO_EXACT: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { trait_has: Dino }
      - gain_memory: 1
"#;

const COST: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { play_cost: { op: eq, value: 3 } }
      - gain_memory: 1
"#;

const COST_RANGE: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { play_cost: [{ op: gte, value: 3 }, { op: lte, value: 5 }] }
      - gain_memory: 1
"#;

const COST_NONE: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { play_cost: { op: lte, value: 1 } }
      - gain_memory: 1
"#;

const COST_TOP: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    condition: { any_permanent: { of: you, zone: [battle_area], play_cost: { op: eq, value: 5 } } }
    process: [{ gain_memory: 1 }]
"#;

const COST_BOTTOM: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    condition: { any_permanent: { of: you, zone: [battle_area], play_cost: { op: eq, value: 3 } } }
    process: [{ gain_memory: 1 }]
"#;

const SOURCES: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          prompt: Pick a permanent
          bind_as: pick
          optional: true
          filter: { kind: digimon, source_count: { filter: { name_contains: Vemmon }, at_least: 2 } }
      - gain_memory: 1
"#;

const SOURCES_ZERO: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          prompt: Pick a permanent
          bind_as: pick
          optional: true
          filter: { kind: digimon, source_count: { filter: {}, at_least: 0 } }
      - gain_memory: 1
"#;

const SOURCE_OPTION: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          prompt: Pick a permanent
          bind_as: pick
          optional: true
          filter: { kind: digimon, source_count: { filter: { kind: option }, at_least: 1 } }
      - gain_memory: 1
"#;

const SOURCE_IF: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - if:
          condition: { source_count: { filter: {}, at_least: 1 } }
          then: [{ gain_memory: 1 }]
"#;

const ATTACK_IF: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - if:
          condition: { can_attack: {} }
          then: [{ gain_memory: 1 }]
"#;

const ORIENT_IF: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - if:
          condition: { can_change_orientation: true }
          then: [{ gain_memory: 1 }]
"#;

const LINK_IF: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - if:
          condition: { link_card_count: { filter: {}, at_least: 1 } }
          then: [{ gain_memory: 1 }]
"#;

const COLORS: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { self_color_count_gte: 2 }
      - gain_memory: 1
"#;

const FACE_DOWN: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - if:
          condition: { face_down_sources_under_tamers_gte: 2 }
          then: [{ gain_memory: 1 }]
"#;

const FACE_DOWN_ONE: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - if:
          condition: { face_down_sources_under_tamers_gte: 1 }
          then: [{ gain_memory: 1 }]
"#;

const ATTACK: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          prompt: Pick a permanent
          bind_as: pick
          optional: true
          filter: { can_attack: { targets: any } }
      - gain_memory: 1
"#;

const ATTACK_FREE: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          prompt: Pick a permanent
          bind_as: pick
          optional: true
          filter: { can_attack: { targets: digimon, without_suspending: true } }
      - gain_memory: 1
"#;

const ORIENT: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          prompt: Pick a permanent
          bind_as: pick
          optional: true
          filter: { can_change_orientation: true }
      - gain_memory: 1
"#;

const NO_ORIENT: &str = r#"
card: PROBE
name: Probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent:
          prompt: Pick a permanent
          bind_as: pick
          optional: true
          filter: { can_change_orientation: false }
      - gain_memory: 1
"#;

#[test]
fn card_number_is_offers_exactly_the_matching_printings() {
    let mut r = runner(NUMBER, &["TARGET", "OTHER", "TARGETX", "TARGET"]);
    probe(&mut r);
    answer(&mut r, &[0, 3]);
}
#[test]
fn card_number_is_gates_any_permanent_on_our_side_only() {
    for side in [None, Some(0), Some(1)] {
        let mut r = runner(NUMBER_GATE, &[]);
        if let Some(p) = side {
            r.place_on_field(p, "TARGET", Some(0));
        }
        probe(&mut r);
        assert_eq!(r.memory(), if side == Some(0) { 6 } else { 5 });
    }
}
#[test]
fn card_number_is_is_case_sensitive() {
    let mut r = runner(NUMBER_LOWER, &["TARGET", "TARGET"]);
    probe(&mut r);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 6);
}
// Deliberate sole-candidate regression: optional selection must not auto-pick.
#[test]
fn card_number_is_single_candidate_is_still_a_real_pick() {
    let mut r = runner(NUMBER, &["OTHER", "TARGET"]);
    probe(&mut r);
    answer(&mut r, &[1]);
}
#[test]
fn name_in_matches_exact_names_and_also_treated_as_aliases() {
    let mut r = runner(NAMES, &["V", "W", "EX", "DEMI", "LOWER", "TST-ALIAS"]);
    probe(&mut r);
    answer(&mut r, &[0, 1, 5]);
}
#[test]
fn name_in_rejects_substrings_and_case_variants() {
    let mut r = runner(NAMES, &["V", "EX", "DEMI", "LOWER", "W"]);
    probe(&mut r);
    answer(&mut r, &[0, 4]);
}
#[test]
fn name_in_on_a_permanent_reads_the_overlay_name() {
    for (yaml, name, want) in [(NAMES_GATE, "Wormmon", 6), (AGUMON_GATE, "Gabumon", 5)] {
        let mut r = runner(yaml, &[]);
        let h = r.place_on_field(0, "TARGET", Some(0));
        r.game.modifiers.add(
            h,
            ModifierEntry::simple(ModifierType::ChangeBaseCardName, 0, Expiry::Permanent, 0)
                .with_payload(ModifierPayload::Name {
                    value: name.into(),
                    base: true,
                }),
        );
        probe(&mut r);
        assert_eq!(r.memory(), want);
    }
}
#[test]
fn name_in_gates_any_permanent() {
    for id in ["V", "OTHER"] {
        let mut r = runner(NAMES_GATE, &[]);
        r.place_on_field(0, id, Some(0));
        probe(&mut r);
        assert_eq!(r.memory(), if id == "V" { 6 } else { 5 });
    }
}
#[test]
fn trait_contains_matches_substrings_case_insensitively() {
    let mut r = runner(TRAITS, &["SAUR", "PLESIO", "SAURIAN", "DRAGON", "OTHER"]);
    probe(&mut r);
    answer(&mut r, &[0, 1, 2]);
}
#[test]
fn trait_contains_reads_granted_traits_on_permanents() {
    for grant in [false, true] {
        let mut r = runner(AQUA, &[]);
        let h = r.place_on_field(0, "OTHER", Some(0));
        if grant {
            r.game.modifiers.add(
                h,
                ModifierEntry::simple(ModifierType::ChangeTraits, 0, Expiry::Permanent, 0)
                    .with_payload(ModifierPayload::Traits {
                        add: vec!["Aquatic".into()],
                        replace: false,
                    }),
            );
        }
        probe(&mut r);
        assert_eq!(r.memory(), if grant { 6 } else { 5 });
    }
}
#[test]
fn trait_contains_versus_trait_has_contrast() {
    let mut r = runner(DINO, &["SAUR", "SAUR"]);
    probe(&mut r);
    answer(&mut r, &[0, 1]);
    let mut r = runner(DINO_EXACT, &["SAUR", "SAUR"]);
    probe(&mut r);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 6);
}
#[test]
fn play_cost_eq_offers_cost_three_digimon_and_option() {
    let mut r = runner(COST, &["OTHER", "TARGET", "FIVE", "OPT", "TARGETX"]);
    probe(&mut r);
    answer(&mut r, &[1, 3]);
}
#[test]
fn play_cost_range_gte_lte() {
    let mut r = runner(COST_RANGE, &["OTHER", "TARGET", "FIVE", "OPT", "TARGETX"]);
    probe(&mut r);
    answer(&mut r, &[1, 2, 3]);
}
#[test]
fn play_cost_unmet_installs_no_prompt() {
    let mut r = runner(COST_NONE, &["OTHER", "TARGET", "FIVE", "OPT", "TARGETX"]);
    probe(&mut r);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 6);
}
#[test]
fn play_cost_on_a_permanent_reads_the_top_card_not_the_sum() {
    for (yaml, want) in [(COST_TOP, 6), (COST_BOTTOM, 5)] {
        let mut r = runner(yaml, &[]);
        r.place_stack(0, &["TARGET", "FIVE"]);
        probe(&mut r);
        assert_eq!(r.memory(), want);
    }
}
#[test]
fn source_count_counts_sources_below_the_top_card_matching_the_filter() {
    let mut r = runner(SOURCES, &[]);
    let a = r.place_stack(0, &["VEM", "VEM", "FIVE"]);
    let a2 = r.place_stack(0, &["VEM", "VEM", "FIVE"]);
    r.place_stack(0, &["VEM", "FIVE"]);
    r.place_stack(0, &["VEM", "VEM"]);
    probe(&mut r);
    answer(
        &mut r,
        &[
            encode_attack(0, a.index as u16),
            encode_attack(0, a2.index as u16),
        ],
    );
}
#[test]
fn source_count_at_least_zero_is_always_true() {
    let mut r = runner(SOURCES_ZERO, &[]);
    r.place_on_field(0, "TARGET", Some(0));
    r.place_stack(0, &["VEM", "FIVE"]);
    probe(&mut r);
    answer(&mut r, &[encode_attack(0, 0), encode_attack(0, 1)]);
}
// DCGO BT25_085 CanTrashCardSource->IsOption->CardKinds uses printed data even
// face down. Preserve sheet row D pending CR 4-7-9 reconciliation (ledger).
#[test]
fn source_count_face_down_row_d_matches_current_dcgo_behavior() {
    let mut r = runner(SOURCES, &[]);
    for _ in 0..2 {
        let h = r.place_stack(0, &["VEM", "VEM", "FIVE"]);
        r.game.players[0].battle_area[h.index as usize].card_sources[1].face_down = true;
    }
    probe(&mut r);
    answer(&mut r, &[encode_attack(0, 0), encode_attack(0, 1)]);
}
#[test]
fn source_count_in_an_if_without_a_permanent_subject_is_false() {
    let mut r = runner(SOURCE_IF, &[]);
    probe(&mut r);
    assert_eq!(r.memory(), 5);
}
#[test]
fn can_attack_in_an_if_without_a_permanent_subject_is_false() {
    let mut r = runner(ATTACK_IF, &[]);
    probe(&mut r);
    assert_eq!(r.memory(), 5);
}
#[test]
fn can_change_orientation_in_an_if_without_a_permanent_subject_is_false() {
    let mut r = runner(ORIENT_IF, &[]);
    probe(&mut r);
    assert_eq!(r.memory(), 5);
}
#[test]
fn link_card_count_in_an_if_without_a_permanent_subject_is_false() {
    let mut r = runner(LINK_IF, &[]);
    probe(&mut r);
    assert_eq!(r.memory(), 5);
}
#[test]
fn self_color_count_gte_on_hand_cards_counts_printed_colours() {
    let mut r = runner(COLORS, &["TARGET", "TWO", "THREE"]);
    probe(&mut r);
    answer(&mut r, &[1, 2]);
}
#[test]
fn self_color_count_gte_deduplicates() {
    let mut r = runner(COLORS, &["DUP", "TWO", "THREE"]);
    probe(&mut r);
    answer(&mut r, &[1, 2]);
}
fn face_down_host(r: &mut DebugRunner, p: u8, top: &str, n: usize) -> PermanentHandle {
    let mut ids = vec!["TARGET"; n];
    ids.push(top);
    let h = r.place_stack(p, &ids);
    for c in &mut r.game.players[p as usize].battle_area[h.index as usize].card_sources[..n] {
        c.face_down = true;
    }
    h
}
#[test]
fn face_down_under_tamers_counts_across_tamers() {
    let mut r = runner(FACE_DOWN, &[]);
    face_down_host(&mut r, 0, "TAMER", 1);
    face_down_host(&mut r, 0, "TAMER", 1);
    probe(&mut r);
    assert_eq!(r.memory(), 6);
}
#[test]
fn face_down_under_tamers_counts_two_under_one_tamer() {
    let mut r = runner(FACE_DOWN, &[]);
    face_down_host(&mut r, 0, "TAMER", 2);
    probe(&mut r);
    assert_eq!(r.memory(), 6);
}
#[test]
fn face_down_under_tamers_ignores_face_up_digimon_sources_and_opponent_tamers() {
    for case in 0..3 {
        let mut r = runner(FACE_DOWN, &[]);
        let h = face_down_host(
            &mut r,
            if case == 2 { 1 } else { 0 },
            if case == 1 { "TARGET" } else { "TAMER" },
            2,
        );
        if case == 0 {
            r.game.players[0].battle_area[h.index as usize].card_sources[0].face_down = false;
        }
        probe(&mut r);
        assert_eq!(r.memory(), 5, "single-fact negative {case}");
    }
}
#[test]
fn face_down_under_tamers_stops_at_the_first_protected_card_from_the_bottom() {
    for upper in [false, true] {
        for (yaml, want) in [(FACE_DOWN, 5), (FACE_DOWN_ONE, if upper { 6 } else { 5 })] {
            let mut r = runner(yaml, &[]);
            let h = face_down_host(&mut r, 0, "TAMER", 2);
            let c = r.game.players[0].battle_area[h.index as usize].card_sources
                [usize::from(upper)]
            .handle();
            r.game.modifiers.add(
                h,
                ModifierEntry::simple(ModifierType::CannotTrash, 0, Expiry::Permanent, 0)
                    .with_payload(ModifierPayload::ProtectedSourceCards(vec![c])),
            );
            probe(&mut r);
            assert_eq!(r.memory(), want);
        }
    }
}
const COLOR_AURA: &str = r#"
card: PROBE
name: Color probe
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
effects:
  - kind: aura
    target: {}
    active_when: { all_turns: true, self_color_count_gte: 3 }
    dp_modifier: 1000
"#;
fn color_aura(grant: bool, face_down: bool) -> DebugRunner {
    let mut r = runner(COLOR_AURA, &[]);
    let h = r.place_stack(0, &["V", "SAURIAN", "PROBE"]);
    r.game.players[0].battle_area[0].card_sources[1].face_down = face_down;
    if grant {
        modifier(&mut r, h, ModifierType::AddColor);
    }
    r.game.tick_declarative_effects();
    r
}
#[test]
fn self_color_count_gte_on_a_permanent_counts_effective_colours() {
    for (grant, want) in [(false, 4000), (true, 5000)] {
        let r = color_aura(grant, false);
        assert_eq!(
            r.effective_dp(PermanentHandle {
                player: 0,
                index: 0
            }),
            Some(want)
        );
    }
}
#[test]
fn self_color_count_gte_ignores_face_down_source_colours() {
    let r = color_aura(true, true);
    assert_eq!(
        r.effective_dp(PermanentHandle {
            player: 0,
            index: 0
        }),
        Some(4000)
    );
}
const DUAL: &str = r#"
card: DUAL
name: Dual
kind: dual
level: 6
color: [red]
dp: 12000
dual:
  digimon: { level: 6, dp: 12000, colors: [red], traits: [], effect_text: '', inherited_text: '' }
  option: { use_cost: 5, colors: [green], effect_text: '', security_text: '', keywords: [] }
effects: []
"#;
#[test]
fn source_count_reads_a_dual_source_as_an_option() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SOURCE_OPTION)
        .unwrap()
        .from_dsl_yaml(DUAL)
        .unwrap()
        .add_card(make_test_card("TOP", "Top"))
        .memory(5)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_stack(0, &["DUAL", "TOP"]);
    r.place_stack(0, &["DUAL", "TOP"]);
    r.place_stack(0, &["TOP", "TOP"]);
    probe(&mut r);
    answer(&mut r, &[encode_attack(0, 0), encode_attack(0, 1)]);
}
#[test]
fn face_down_under_tamers_ignores_an_opponent_only_immunity_for_our_own_effect() {
    use digimon_engine::modifiers::{EffectControllerFilter, EffectImmunityFilter};
    let mut r = runner(FACE_DOWN, &[]);
    let h = face_down_host(&mut r, 0, "TAMER", 2);
    let mut entry = ModifierEntry::simple(
        ModifierType::ImmuneFromStackTrashing,
        0,
        Expiry::Permanent,
        0,
    );
    entry.effect_immunity_filter = Some(EffectImmunityFilter {
        controller: EffectControllerFilter::OpponentOnly,
        source_kind: None,
    });
    r.game.modifiers.add(h, entry);
    probe(&mut r);
    assert_eq!(r.memory(), 6);
}
const ON_FIELD: &str = r#"
card: PROBE
name: Field probe
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    condition: { on_field: true }
    process:
      - gain_memory: 1
      - if:
          condition: { on_field: false }
          then: [{ gain_memory: 2 }]
"#;
const OFF_FIELD: &str = r#"
card: PROBE
name: Off field probe
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - if:
          condition: { on_field: false }
          then: [{ gain_memory: 2 }]
"#;
const LEAVE_FIELD: &str = r#"
card: PROBE
name: Leaving probe
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - select_hand: { of: you, prompt: Pause, bind_as: pause, optional: true, filter: {} }
      - if:
          condition: { on_field: true }
          then: [{ gain_memory: 2 }]
"#;
#[test]
fn on_field_true_in_the_battle_area_and_the_if_reads_the_source() {
    let mut r = runner(ON_FIELD, &[]);
    probe(&mut r);
    assert_eq!(r.memory(), 6);
}
#[test]
fn on_field_false_from_the_breeding_area() {
    use digimon_engine::{
        action::space::BREEDING_TARGET,
        dsl_cards::predicate::{eval_predicate, PredicateSubject},
        effect_context::EffectReadContext,
    };
    let mut r = runner(ON_FIELD, &[]);
    r.place_in_breeding(0, "PROBE");
    let h = PermanentHandle {
        player: 0,
        index: BREEDING_TARGET as u8,
    };
    let c = r.game.players[0]
        .breeding_area
        .as_ref()
        .unwrap()
        .top_card()
        .handle();
    let ctx = EffectReadContext::new(&r.game, c, Some(h), 0);
    let pred = digimon_dsl::compiled::CompiledPredicate {
        on_field: Some(true),
        ..Default::default()
    };
    assert!(!eval_predicate(&pred, &ctx, PredicateSubject::Permanent(h)));
    // Actual battle-area clause companion proves that the fixture is live.
    probe(&mut r);
    assert_eq!(r.memory(), 6);
}
#[test]
fn on_field_false_for_an_option_used_from_hand() {
    let mut r = runner(OFF_FIELD, &["PROBE"]);
    r.place_on_field(0, "TARGET", Some(0));
    assert_eq!(build_action_mask(&r.game, 0)[0], 1.0);
    r.game.decode_action(0, 0);
    assert_eq!(r.memory(), 7);
    assert_eq!(r.game.players[0].trash.len(), 1);
}
#[test]
fn on_field_pending_continuation_does_not_reuse_a_departed_source_slot() {
    for replace in [false, true] {
        let mut r = runner(LEAVE_FIELD, &["TARGET", "OTHER"]);
        let h = r.place_on_field(0, "PROBE", Some(0));
        let original = r.game.players[0].battle_area[0].top_card().handle();
        fire(&mut r, h);
        assert_eq!(
            r.game.pending_selection.as_ref().unwrap().valid_action_ids,
            vec![0, 1]
        );
        let departed = r.game.players[0].battle_area.remove(0);
        r.game.players[0].trash.extend(departed.card_sources);
        if replace {
            let new = r.place_on_field(0, "PROBE", Some(0));
            assert_eq!(new, h);
            assert_ne!(
                original,
                r.game.players[0].battle_area[0].top_card().handle()
            );
        }
        assert_eq!(build_action_mask(&r.game, 0)[1], 1.0);
        r.execute_action(0, 1).unwrap();
        assert_eq!(
            r.memory(),
            5,
            "departed source (same-slot replacement={replace}) must not count as in play"
        );
    }
}
const REPLACE_COST: &str = r#"
card: PROBE
name: Cost replacement
kind: tamer
color: [red]
cost: 2
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    optional: true
    active_when: { play_cost: { op: gte, value: 5 } }
    outcome: prevent
"#;
#[test]
fn replacement_active_when_with_play_cost_gte_reads_the_subject() {
    use digimon_engine::{action::space::REPLACEMENT_ACCEPT, replacement::ReplacementCause};
    for (target, protect) in [("FIVE", true), ("TARGET", false)] {
        let mut r = runner(REPLACE_COST, &[]);
        r.place_on_field(0, "PROBE", Some(0));
        let h = r.place_on_field(0, target, Some(0));
        r.game.set_effect_source_player_for_test(Some(1));
        r.game
            .delete_permanents_batch(vec![h], ReplacementCause::OpponentEffect);
        if protect {
            let p = r
                .game
                .pending_selection
                .as_ref()
                .expect("other subject gets cost-based replacement");
            assert!(p.is_optional);
            assert_eq!(p.valid_action_ids, vec![REPLACEMENT_ACCEPT]);
            assert_eq!(
                build_action_mask(&r.game, 0)[REPLACEMENT_ACCEPT as usize],
                1.0
            );
            r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
            assert_eq!(r.battle_area_size(0), 2);
        } else {
            assert!(r.game.pending_selection.is_none());
            assert_eq!(r.battle_area_size(0), 1);
        }
    }
}
fn player_modifier(r: &mut DebugRunner, m: ModifierType) {
    r.game.modifiers.add_player_modifier(
        0,
        digimon_engine::modifiers::PlayerModifierEntry::simple(m, 0, Expiry::Permanent, None, 0),
    );
}
fn attack_row(row: u8, ordinary: bool, free: bool) {
    for (yaml, want) in [(ATTACK, ordinary), (ATTACK_FREE, free)] {
        let mut r = runner(yaml, &[]);
        let fresh = matches!(row, 4 | 5);
        let id = if row == 13 { "TAMER" } else { "TARGET" };
        let a = r.place_on_field(0, id, if fresh { None } else { Some(0) });
        let b = r.place_on_field(0, id, if fresh { None } else { Some(0) });
        if !matches!(row, 1 | 9) {
            let target = r.place_on_field(1, "TARGET", Some(0));
            r.game.players[1].battle_area[target.index as usize].is_suspended = row != 10;
            if row == 11 {
                modifier(&mut r, target, ModifierType::CannotAttackTarget);
            }
        }
        for h in [a, b] {
            match row {
                3 => r.game.players[0].battle_area[h.index as usize].is_suspended = true,
                5 => {
                    r.game
                        .modifiers
                        .grant_keyword(h, Keyword::Rush, Expiry::Permanent, 0);
                }
                6 => modifier(&mut r, h, ModifierType::CannotAttack),
                9 => modifier(&mut r, h, ModifierType::CannotAttackPlayer),
                10 => modifier(&mut r, h, ModifierType::CanAttackUnsuspended),
                _ => {}
            }
        }
        if row == 7 {
            player_modifier(&mut r, ModifierType::CannotAttack);
        }
        if row == 12 {
            r.set_first_player(1);
        }
        probe(&mut r);
        if want {
            answer(&mut r, &[encode_attack(0, 0), encode_attack(0, 1)]);
        } else {
            assert!(r.game.pending_selection.is_none(), "row {row}");
            assert_eq!(r.memory(), if row == 12 { 4 } else { 6 });
        }
    }
}
#[test]
fn can_attack_row_1_no_digimon_target() {
    attack_row(1, true, false);
}
#[test]
fn can_attack_row_2_suspended_target() {
    attack_row(2, true, true);
}
#[test]
fn can_attack_row_3_suspended_attacker() {
    attack_row(3, false, true);
}
#[test]
fn can_attack_row_4_fresh() {
    attack_row(4, false, false);
}
#[test]
fn can_attack_row_5_fresh_rush() {
    attack_row(5, true, true);
}
#[test]
fn can_attack_row_6_cannot_attack() {
    attack_row(6, false, false);
}
#[test]
fn can_attack_row_7_player_cannot_attack() {
    attack_row(7, false, false);
}
#[test]
fn can_attack_row_9_cannot_attack_player() {
    attack_row(9, false, false);
}
#[test]
fn can_attack_row_10_unsuspended_target_permission() {
    attack_row(10, true, true);
}
#[test]
fn can_attack_row_11_protected_target() {
    attack_row(11, true, false);
}
#[test]
fn can_attack_row_12_opponent_turn() {
    attack_row(12, false, false);
}
#[test]
fn can_attack_row_13_tamer() {
    attack_row(13, false, false);
}
#[test]
fn can_change_orientation_partitions_the_board() {
    for blocked_player in [false, true] {
        for (yaml, can) in [(ORIENT, true), (NO_ORIENT, false)] {
            let mut r = runner(yaml, &[]);
            let u = r.place_on_field(0, "TARGET", Some(0));
            let us = r.place_on_field(0, "TARGET", Some(0));
            let s = r.place_on_field(0, "TARGET", Some(0));
            let su = r.place_on_field(0, "TARGET", Some(0));
            let st = r.place_on_field(0, "TAMER", Some(0));
            modifier(&mut r, us, ModifierType::CannotSuspend);
            modifier(&mut r, su, ModifierType::CannotUnsuspend);
            for h in [s, su, st] {
                r.game.players[0].battle_area[h.index as usize].is_suspended = true;
            }
            if blocked_player {
                player_modifier(&mut r, ModifierType::CannotUnsuspend);
            }
            probe(&mut r);
            let indices = match (can, blocked_player) {
                (true, false) => vec![u.index, s.index, st.index, 5],
                (true, true) => vec![u.index, st.index, 5],
                (false, false) => vec![us.index, su.index],
                (false, true) => vec![us.index, s.index, su.index],
            };
            answer(
                &mut r,
                &indices
                    .into_iter()
                    .map(|i| encode_attack(0, i as u16))
                    .collect::<Vec<_>>(),
            );
        }
    }
}
#[test]
fn can_change_orientation_excludes_breeding() {
    let mut r = runner(ORIENT, &[]);
    r.place_in_breeding(0, "TARGET");
    r.place_on_field(0, "TARGET", Some(0));
    probe(&mut r);
    answer(&mut r, &[encode_attack(0, 0), encode_attack(0, 1)]);
}
#[test]
fn can_change_orientation_all_cannot_installs_no_prompt() {
    let mut r = runner(ORIENT, &[]);
    let a = r.place_on_field(0, "TARGET", Some(0));
    let h = r.place_on_field(0, "PROBE", Some(0));
    for x in [a, h] {
        modifier(&mut r, x, ModifierType::CannotSuspend);
    }
    fire(&mut r, h);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 6);
}

const SOURCE_CARD: &str = r#"
card: PROBE
name: Card subject probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { source_count: { filter: {}, at_least: 0 } }
      - gain_memory: 1
"#;

const LINK_CARD: &str = r#"
card: PROBE
name: Card subject probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { link_card_count: { filter: {}, at_least: 0 } }
      - gain_memory: 1
"#;

const ATTACK_CARD: &str = r#"
card: PROBE
name: Card subject probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { can_attack: {} }
      - gain_memory: 1
"#;

const ORIENT_CARD: &str = r#"
card: PROBE
name: Card subject probe
kind: tamer
color: [red]
cost: 2
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          prompt: Pick a card
          bind_as: pick
          optional: true
          filter: { can_change_orientation: true }
      - gain_memory: 1
"#;

#[test]
fn permanent_only_leaves_reject_card_subjects() {
    for yaml in [SOURCE_CARD, LINK_CARD, ATTACK_CARD, ORIENT_CARD] {
        let mut r = runner(yaml, &["TARGET", "OTHER"]);
        probe(&mut r);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.memory(), 6);
    }
}

const REPLACE_EQ: &str = r#"
card: PROBE
name: Subject replacement
kind: tamer
color: [red]
cost: 2
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    optional: true
    active_when: { play_cost: { op: eq, value: 5 } }
    outcome: prevent
"#;

const REPLACE_TRAIT: &str = r#"
card: PROBE
name: Subject replacement
kind: tamer
color: [red]
cost: 2
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    optional: true
    active_when: { trait_contains: saur }
    outcome: prevent
"#;

const REPLACE_COLOR: &str = r#"
card: PROBE
name: Subject replacement
kind: tamer
color: [red]
cost: 2
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    optional: true
    active_when: { self_color_count_gte: 2 }
    outcome: prevent
"#;

#[test]
fn replacement_identity_leaves_read_the_other_subject() {
    use digimon_engine::{action::space::REPLACEMENT_ACCEPT, replacement::ReplacementCause};
    for (yaml, yes, no) in [
        (REPLACE_EQ, "FIVE", "TARGET"),
        (REPLACE_TRAIT, "SAUR", "TARGET"),
        (REPLACE_COLOR, "TWO", "TARGET"),
    ] {
        for (target, protect) in [(yes, true), (no, false)] {
            let mut r = runner(yaml, &[]);
            r.place_on_field(0, "PROBE", Some(0));
            let h = r.place_on_field(0, target, Some(0));
            r.game.set_effect_source_player_for_test(Some(1));
            r.game
                .delete_permanents_batch(vec![h], ReplacementCause::OpponentEffect);
            if protect {
                let p = r
                    .game
                    .pending_selection
                    .as_ref()
                    .expect("other subject is eligible");
                assert!(p.is_optional);
                assert_eq!(
                    build_action_mask(&r.game, 0)[REPLACEMENT_ACCEPT as usize],
                    1.0
                );
                r.execute_action(0, REPLACEMENT_ACCEPT).unwrap();
                assert_eq!(r.battle_area_size(0), 2);
            } else {
                assert!(r.game.pending_selection.is_none());
                assert_eq!(r.battle_area_size(0), 1);
            }
        }
    }
}

const SELF_LEAVE: &str = r#"
card: PROBE
name: Synchronous departure
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
effects:
  - when: on_play
    process:
      - delete_permanent: { target: source }
      - if:
          condition: { on_field: true }
          then: [{ gain_memory: 2 }]
      - gain_memory: 1
"#;
#[test]
fn on_field_false_after_the_source_left_the_field() {
    let mut r = runner(SELF_LEAVE, &[]);
    probe(&mut r);
    assert_eq!(r.battle_area_size(0), 0);
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(
        r.memory(),
        6,
        "tail must execute once, excluding departed-source bonus"
    );
    // Supplement the executed clause with the stale-context same-slot case.
    // The replacement is even the same printing: only physical identity differs.
    let departed = r.game.players[0].trash[0].handle();
    let h = r.place_on_field(0, "PROBE", Some(0));
    assert_eq!(h.index, 0);
    assert_ne!(r.top_card(h), departed);
    let ctx = digimon_engine::effect_context::EffectReadContext::new(&r.game, departed, Some(h), 0);
    let pred = digimon_dsl::compiled::CompiledPredicate {
        on_field: Some(true),
        ..Default::default()
    };
    assert!(!digimon_engine::dsl_cards::predicate::eval_predicate(
        &pred,
        &ctx,
        digimon_engine::dsl_cards::predicate::PredicateSubject::None
    ));
    // An explicit subject is the new permanent, not the departed source.
    assert!(digimon_engine::dsl_cards::predicate::eval_predicate(
        &pred,
        &ctx,
        digimon_engine::dsl_cards::predicate::PredicateSubject::Permanent(h)
    ));
}

#[test]
fn source_count_no_matching_permanent_installs_no_prompt() {
    let mut r = runner(SOURCES, &[]);
    r.place_stack(0, &["VEM", "FIVE"]);
    r.place_stack(0, &["VEM", "VEM"]);
    probe(&mut r);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), 6, "continuation runs with no eligible host");
}
