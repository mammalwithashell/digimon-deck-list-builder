//! Distinct-name/color constraints over one physical hand/trash picker.
//! Uses the same name equality and injective printed-color assignment as the
//! established count-capped picker. Source movement occurs only after completion.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{PASS, TRASH_EFFECT_START},
    },
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, GamePhase},
};

const PICKER: &str = r#"
card: TEST-MIXED-DISTINCT
name: Mixed distinct probe
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - select_zone_cards:
          zones: [hand, trash]
          filter: {trait_has: Hybrid}
          min: 0
          max: 3
          distinct_by: name
          bind_as: selected
          prompt: Choose distinct Hybrid cards
      - if:
          condition: {binding_present: selected}
          then:
            - place_cards_as_bottom_sources: {cards: selected, target: source}
            - gain_memory: 1
"#;
fn setup(mode: Option<&str>, max: u8, hand: &[&str], trash: &[&str]) -> DebugRunner {
    setup_min(mode, max, 0, hand, trash)
}
fn setup_min(mode: Option<&str>, max: u8, min: u8, hand: &[&str], trash: &[&str]) -> DebugRunner {
    setup_budget(mode, max, min, None, hand, trash)
}
fn setup_budget(
    mode: Option<&str>,
    max: u8,
    min: u8,
    budget: Option<i32>,
    hand: &[&str],
    trash: &[&str],
) -> DebugRunner {
    setup_zones(mode, max, min, budget, "hand, trash", hand, trash)
}
fn setup_zones(
    mode: Option<&str>,
    max: u8,
    min: u8,
    budget: Option<i32>,
    zones: &str,
    hand: &[&str],
    trash: &[&str],
) -> DebugRunner {
    let constraint = mode
        .map(|v| format!("          distinct_by: {v}\n"))
        .unwrap_or_default();
    let mut yaml = PICKER
        .replace("zones: [hand, trash]", &format!("zones: [{zones}]"))
        .replace("          distinct_by: name\n", &constraint)
        .replace("min: 0", &format!("min: {min}"))
        .replace("max: 3", &format!("max: {max}"));
    if let Some(budget) = budget {
        yaml = yaml.replace(
            "          bind_as: selected",
            &format!("          play_cost_budget: {budget}\n          bind_as: selected"),
        );
    }
    let mut b = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("distinct mixed-zone YAML");
    for (id, name, colors) in [
        ("TEST-A", "Same", vec![CardColor::Red]),
        ("TEST-B", "Same", vec![CardColor::Blue]),
        ("TEST-C", "Other", vec![CardColor::Green]),
        ("TEST-RB", "Dual", vec![CardColor::Red, CardColor::Blue]),
        ("TEST-R", "Red", vec![CardColor::Red]),
        ("TEST-R4", "Expensive red", vec![CardColor::Red]),
        ("TEST-BLUE", "Blue", vec![CardColor::Blue]),
        ("TEST-G", "Green", vec![CardColor::Green]),
        ("TEST-ZERO", "Colorless", vec![]),
    ] {
        let mut c = make_test_card(id, name);
        c.colors = colors;
        c.play_cost = match id {
            "TEST-A" | "TEST-R" => 1,
            "TEST-B" | "TEST-C" | "TEST-R4" | "TEST-BLUE" | "TEST-G" => 4,
            _ => 3,
        };
        c.traits = vec!["Hybrid".into()];
        b = b.add_card(c);
    }
    let mut r = b
        .deck(0, &["TEST-C"; 12])
        .deck(1, &["TEST-C"; 12])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.game.current_phase = GamePhase::Main;
    r.game.players[0].hand.clear();
    r.add_to_hand(0, "TEST-MIXED-DISTINCT");
    for id in hand {
        r.add_to_hand(0, id);
    }
    for id in trash {
        r.inject_trash(0, id);
    }
    r.play(0, 0).expect("play test probe");
    r
}
fn mask(r: &DebugRunner, expected: &[u16]) {
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("pending mixed picker");
    let actual = build_action_mask(&r.game, pending.selecting_player);
    for (id, &legal) in actual.iter().enumerate() {
        assert_eq!(legal > 0.5, expected.contains(&(id as u16)), "action {id}");
    }
}
fn pick(r: &mut DebugRunner, id: u16) {
    let player = r
        .game
        .pending_selection
        .as_ref()
        .expect("pending pick")
        .selecting_player;
    assert!(build_action_mask(&r.game, player)[id as usize] > 0.5);
    r.execute_action(player, id).expect("legal choice resolves");
}
fn sources(r: &DebugRunner) -> Vec<String> {
    r.game.players[0].battle_area[0]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}
#[test]
fn mixed_zone_distinct_name_excludes_same_name_other_printing_and_preserves_origins() {
    let mut r = setup(Some("name"), 3, &["TEST-A"], &["TEST-B", "TEST-C"]);
    mask(&r, &[0, TRASH_EFFECT_START, TRASH_EFFECT_START + 1, PASS]);
    pick(&mut r, 0);
    mask(&r, &[TRASH_EFFECT_START + 1, PASS]);
    assert_eq!(
        r.game.players[0].hand.len(),
        1,
        "picks remain in their zones until commit"
    );
    pick(&mut r, TRASH_EFFECT_START + 1);
    assert!(
        r.game.pending_selection.is_none(),
        "exhausted distinct names finish automatically"
    );
    assert_eq!(sources(&r), ["TEST-C", "TEST-A", "TEST-MIXED-DISTINCT"]);
    assert!(r.game.players[0].hand.is_empty());
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(
        r.game.players[0].trash[0].card_id(&r.game.card_data),
        "TEST-B"
    );
    assert_eq!(r.memory(), 4);
}
#[test]
fn mixed_zone_distinct_color_uses_injective_matching_and_excludes_colorless() {
    let mut r = setup(
        Some("color"),
        3,
        &["TEST-RB", "TEST-ZERO"],
        &["TEST-R", "TEST-BLUE", "TEST-G"],
    );
    mask(
        &r,
        &[
            0,
            TRASH_EFFECT_START,
            TRASH_EFFECT_START + 1,
            TRASH_EFFECT_START + 2,
            PASS,
        ],
    );
    pick(&mut r, 0);
    mask(
        &r,
        &[
            TRASH_EFFECT_START,
            TRASH_EFFECT_START + 1,
            TRASH_EFFECT_START + 2,
            PASS,
        ],
    );
    pick(&mut r, TRASH_EFFECT_START);
    mask(&r, &[TRASH_EFFECT_START + 2, PASS]);
    pick(&mut r, TRASH_EFFECT_START + 2);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        sources(&r),
        ["TEST-G", "TEST-R", "TEST-RB", "TEST-MIXED-DISTINCT"]
    );
    assert_eq!(
        r.game.players[0].hand[0].card_id(&r.game.card_data),
        "TEST-ZERO"
    );
    assert_eq!(
        r.game.players[0].trash[0].card_id(&r.game.card_data),
        "TEST-BLUE"
    );
}
#[test]
fn mixed_zone_distinct_optional_zero_keeps_binding_unbound_and_zones_unchanged() {
    let mut r = setup(Some("name"), 3, &["TEST-A"], &["TEST-C"]);
    pick(&mut r, PASS);
    assert_eq!(sources(&r), ["TEST-MIXED-DISTINCT"]);
    assert_eq!(r.game.players[0].hand.len(), 1);
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(r.memory(), 3);
}
#[test]
fn mixed_zone_distinct_partial_pick_clone_can_finish_or_continue_independently() {
    let mut r = setup(Some("name"), 3, &["TEST-A"], &["TEST-B", "TEST-C"]);
    pick(&mut r, 0);
    let saved = r.game.clone();
    mask(&r, &[TRASH_EFFECT_START + 1, PASS]);
    pick(&mut r, TRASH_EFFECT_START + 1);
    assert_eq!(sources(&r).len(), 3);
    r.game = saved;
    mask(&r, &[TRASH_EFFECT_START + 1, PASS]);
    pick(&mut r, PASS);
    assert_eq!(sources(&r), ["TEST-A", "TEST-MIXED-DISTINCT"]);
    assert_eq!(r.game.players[0].trash.len(), 2);
}
#[test]
fn mixed_zone_distinct_omitted_retains_physical_card_only_uniqueness() {
    let mut r = setup(None, 3, &["TEST-A"], &["TEST-B"]);
    pick(&mut r, 0);
    mask(&r, &[TRASH_EFFECT_START, PASS]);
    pick(&mut r, TRASH_EFFECT_START);
    assert_eq!(sources(&r), ["TEST-B", "TEST-A", "TEST-MIXED-DISTINCT"]);
}

#[test]
fn mixed_zone_distinct_required_floor_with_insufficient_distinct_names_does_not_prompt() {
    let r = setup_min(Some("name"), 2, 2, &["TEST-A"], &["TEST-B"]);
    assert!(
        r.game.pending_selection.is_none(),
        "two physical cards sharing a name cannot pay a two-distinct-card floor"
    );
    assert_eq!(sources(&r), ["TEST-MIXED-DISTINCT"]);
    assert_eq!(r.memory(), 3);
    assert_eq!(r.game.players[0].hand.len(), 1);
    assert_eq!(r.game.players[0].trash.len(), 1);
}

#[test]
fn mixed_zone_distinct_required_floor_uses_color_matching_rank() {
    let r = setup_min(Some("color"), 3, 3, &["TEST-RB"], &["TEST-R", "TEST-BLUE"]);
    assert!(
        r.game.pending_selection.is_none(),
        "three cards using only red/blue cannot pay a three-color floor"
    );
    assert_eq!(sources(&r), ["TEST-MIXED-DISTINCT"]);
    assert_eq!(r.memory(), 3);
}

#[test]
fn mixed_zone_distinct_budget_excludes_name_prefix_that_cannot_complete_minimum() {
    for first in [0, TRASH_EFFECT_START + 1] {
        let mut r = setup_budget(
            Some("name"),
            2,
            2,
            Some(5),
            &["TEST-A"],
            &["TEST-B", "TEST-C"],
        );
        mask(&r, &[0, TRASH_EFFECT_START + 1]);
        pick(&mut r, first);
        let remaining = if first == 0 {
            TRASH_EFFECT_START + 1
        } else {
            0
        };
        mask(&r, &[remaining]);
        pick(&mut r, remaining);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(sources(&r).len(), 3);
        assert_eq!(r.game.players[0].trash.len(), 1);
        assert_eq!(
            r.game.players[0].trash[0].card_id(&r.game.card_data),
            "TEST-B"
        );
        assert_eq!(r.memory(), 4);
    }
}
#[test]
fn mixed_zone_distinct_budget_excludes_color_prefix_that_cannot_complete_minimum() {
    let mut r = setup_budget(
        Some("color"),
        2,
        2,
        Some(5),
        &["TEST-R"],
        &["TEST-R4", "TEST-BLUE"],
    );
    mask(&r, &[0, TRASH_EFFECT_START + 1]);
    pick(&mut r, TRASH_EFFECT_START + 1);
    mask(&r, &[0]);
    pick(&mut r, 0);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(sources(&r), ["TEST-R", "TEST-BLUE", "TEST-MIXED-DISTINCT"]);
    assert_eq!(
        r.game.players[0].trash[0].card_id(&r.game.card_data),
        "TEST-R4"
    );
}

#[test]
fn mixed_zone_distinct_budget_color_completion_uses_matching_after_contraction() {
    let mut r = setup_budget(
        Some("color"),
        3,
        3,
        Some(8),
        &["TEST-RB", "TEST-R"],
        &["TEST-BLUE", "TEST-G"],
    );
    mask(&r, &[0, 1, TRASH_EFFECT_START + 1]);
    pick(&mut r, TRASH_EFFECT_START + 1);
    mask(&r, &[0, 1]);
    pick(&mut r, 0);
    mask(&r, &[1]);
    pick(&mut r, 1);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(
        sources(&r),
        ["TEST-R", "TEST-RB", "TEST-G", "TEST-MIXED-DISTINCT"]
    );
}
#[test]
fn mixed_zone_distinct_budget_does_not_count_an_unencodable_completion() {
    let mut hand = vec!["TEST-B"; 30];
    hand.push("TEST-A");
    let r = setup_budget(Some("name"), 2, 2, Some(5), &hand, &["TEST-C"]);
    assert!(
        r.game.pending_selection.is_none(),
        "the only cheap completion is outside the hand action range"
    );
    assert_eq!(sources(&r), ["TEST-MIXED-DISTINCT"]);
    assert_eq!(r.memory(), 3);
}

#[test]
fn mixed_zone_distinct_budget_zero_floor_keeps_an_affordable_partial_pick_legal() {
    let mut r = setup_budget(
        Some("name"),
        2,
        0,
        Some(5),
        &["TEST-A"],
        &["TEST-B", "TEST-C"],
    );
    mask(&r, &[0, TRASH_EFFECT_START, TRASH_EFFECT_START + 1, PASS]);
    pick(&mut r, TRASH_EFFECT_START);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(sources(&r), ["TEST-B", "TEST-MIXED-DISTINCT"]);
    assert_eq!(r.memory(), 4);
}

#[test]
fn mixed_zone_distinct_preflight_does_not_count_one_multicolor_card_twice() {
    let r = setup_zones(Some("color"), 2, 2, None, "hand, hand", &["TEST-RB"], &[]);
    assert!(
        r.game.pending_selection.is_none(),
        "repeated zones do not create a second physical material"
    );
    assert_eq!(sources(&r), ["TEST-MIXED-DISTINCT"]);
    assert_eq!(r.game.players[0].hand.len(), 1);
    assert_eq!(r.memory(), 3);
}
#[test]
fn mixed_zone_distinct_preflight_only_counts_currently_encodable_cards() {
    let mut hand = vec!["TEST-A"; 30];
    hand.push("TEST-C");
    let r = setup_min(Some("name"), 2, 2, &hand, &[]);
    assert!(
        r.game.pending_selection.is_none(),
        "the only second name is outside the hand action range"
    );
    assert_eq!(sources(&r), ["TEST-MIXED-DISTINCT"]);
    assert_eq!(r.game.players[0].hand.len(), 31);
    assert_eq!(r.memory(), 3);
}
