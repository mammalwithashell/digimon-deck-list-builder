//! Behavioral mechanics tests for DSL constructs required by training-gate cards:
//! - `timing:before_pay_cost_observe`
//! - `verb:substitute_replacement`
//! - `predicate:card_in_zone`
//! - `predicate:rules_text_contains`
//! - `timing:on_add_to_hand` + `predicate:event_add_to_hand_player`
//! - `predicate:can_play_to_breeding_from_trash` + `verb:play_to_breeding_from_trash`
//! - `predicate:self_stack_same_level_count_gte`
//! - `predicate:trashed_hand_card_matching`

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dna_digivolve::DnaRouteWindow;
use digimon_engine::enums::{CardColor, GamePhase, PlaySource};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

// ---------------------------------------------------------------------------
// 1. timing:before_pay_cost_observe
// ---------------------------------------------------------------------------

const BEFORE_PAY_COST_HOST_YAML: &str = r#"
card: TGC-DNA-BASE-BLUE
name: BlueBase
kind: digimon
color: [blue]
level: 4
cost: 5
dp: 5000
effects:
  - when: before_pay_cost_observe
    active_when:
      all_of:
        - your_turn: true
        - dna_origin: true
        - source_is_cost_target_permanent: true
        - cost_target: { color_is: green, kind: digimon }
    process:
      - gain_memory: 1
"#;

const BEFORE_PAY_COST_DNA_TARGET_YAML: &str = r#"
card: TGC-DNA-GREEN-LV5
name: GreenJogress
kind: digimon
color: [green, blue]
level: 5
cost: 7
dp: 8000
alt_paths:
  - kind: dna_digivolve
    materials:
      - { level_eq: 4, color_is: blue }
      - { level_eq: 4, color_is: green }
    cost: 0
effects: []
"#;

#[test]
fn before_pay_cost_observe_fires_on_matching_dna_and_skips_normal_digivolve() {
    let mut green_base = make_test_card("TGC-DNA-BASE-GREEN", "GreenBase");
    green_base.level = Some(4);
    green_base.colors = vec![CardColor::Green];
    green_base.dp = Some(5000);

    let mut filler = make_test_card("TGC-FILLER", "Filler");
    filler.level = Some(3);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(BEFORE_PAY_COST_HOST_YAML)
        .unwrap()
        .from_dsl_yaml(BEFORE_PAY_COST_DNA_TARGET_YAML)
        .unwrap()
        .add_card(green_base)
        .add_card(filler)
        .hand(0, &["TGC-DNA-GREEN-LV5"])
        .deck(0, &["TGC-FILLER", "TGC-FILLER"])
        .start();
    r.skip_mulligan();
    r.game.memory = 2;
    let blue_perm = r.place_on_field(0, "TGC-DNA-BASE-BLUE", Some(0));
    let green_perm = r.place_on_field(0, "TGC-DNA-BASE-GREEN", Some(0));
    let hand_idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "TGC-DNA-GREEN-LV5")
        .unwrap();

    r.game.current_phase = GamePhase::Main;
    r.game.resolve_dna_digivolve_stage2_with_window(
        0,
        blue_perm.index as usize,
        green_perm.index as usize,
        hand_idx,
        DnaRouteWindow::Main,
    );
    r.auto_resolve().unwrap();

    assert_eq!(
        r.game.memory, 3,
        "before_pay_cost_observe must grant +1 memory during matching DNA digivolve"
    );
}

#[test]
fn before_pay_cost_observe_does_not_fire_on_standard_digivolve() {
    let mut std_lv5 = make_test_card("TGC-STD-GREEN-LV5", "StdGreen");
    std_lv5.level = Some(5);
    std_lv5.colors = vec![CardColor::Green];
    std_lv5.dp = Some(7000);
    std_lv5.evo_costs.push(digimon_engine::card_data::EvoCost {
        card_color: CardColor::Blue as u8,
        level: 4,
        memory_cost: 3,
    });

    let mut filler = make_test_card("TGC-FILLER", "Filler");
    filler.level = Some(3);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(BEFORE_PAY_COST_HOST_YAML)
        .unwrap()
        .add_card(std_lv5)
        .add_card(filler)
        .hand(0, &["TGC-STD-GREEN-LV5"])
        .deck(0, &["TGC-FILLER", "TGC-FILLER"])
        .start();
    r.skip_mulligan();
    r.game.memory = 5;
    let blue_perm = r.place_on_field(0, "TGC-DNA-BASE-BLUE", Some(0));
    let hand_idx = r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == "TGC-STD-GREEN-LV5")
        .unwrap();

    assert!(r
        .game
        .digivolve_from_hand(0, hand_idx, blue_perm.index as usize, PlaySource::ByHand));
    r.auto_resolve().unwrap();

    assert_eq!(
        r.game.memory, 2,
        "before_pay_cost_observe gated on dna_origin must not grant memory on standard digivolve"
    );
}

// ---------------------------------------------------------------------------
// 2. verb:substitute_replacement
// ---------------------------------------------------------------------------

const SUBSTITUTE_DECOY_YAML: &str = r#"
card: TGC-DECOY
name: DecoyMon
kind: digimon
color: [white]
level: 3
cost: 3
dp: 3000
effects:
  - kind: replacement
    timing: when_would_be_deleted
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - replacement_cause: opponent_effect
        - kind: digimon
        - other: true
        - color_is: red
    process:
      - substitute_replacement:
          subject: source
"#;

#[test]
fn substitute_replacement_accept_deletes_decoy_and_saves_ally() {
    let mut ally = make_test_card("TGC-RED-ALLY", "RedAlly");
    ally.level = Some(4);
    ally.colors = vec![CardColor::Red];
    ally.dp = Some(5000);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SUBSTITUTE_DECOY_YAML)
        .unwrap()
        .add_card(ally)
        .start();
    r.skip_mulligan();
    let ally_handle = r.place_on_field(0, "TGC-RED-ALLY", Some(0));
    r.place_on_field(0, "TGC-DECOY", Some(0));

    r.game
        .delete_permanent_with_cause(ally_handle, ReplacementCause::OpponentEffect);

    let sel = r
        .pending_selection_view()
        .expect("optional substitute_replacement must prompt player 0");
    assert_eq!(sel.kind, SelectionKind::Replacement);
    let accept = digimon_engine::action::space::REPLACEMENT_ACCEPT;
    assert!(sel.valid_action_ids.contains(&accept));
    r.execute_action(0, accept).unwrap();
    r.auto_resolve().unwrap();

    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "TGC-RED-ALLY",
        "ally must survive when decoy substitutes"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "TGC-DECOY"),
        "decoy must be deleted in place of ally"
    );
}

#[test]
fn substitute_replacement_decline_deletes_original_ally() {
    let mut ally = make_test_card("TGC-RED-ALLY", "RedAlly");
    ally.level = Some(4);
    ally.colors = vec![CardColor::Red];
    ally.dp = Some(5000);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SUBSTITUTE_DECOY_YAML)
        .unwrap()
        .add_card(ally)
        .start();
    r.skip_mulligan();
    let ally_handle = r.place_on_field(0, "TGC-RED-ALLY", Some(0));
    r.place_on_field(0, "TGC-DECOY", Some(0));

    r.game
        .delete_permanent_with_cause(ally_handle, ReplacementCause::OpponentEffect);

    let sel = r
        .pending_selection_view()
        .expect("optional substitute_replacement must prompt player 0");
    assert_eq!(sel.kind, SelectionKind::Replacement);
    assert!(sel.is_optional, "optional replacement must allow declining");
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().unwrap();

    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "TGC-DECOY",
        "decoy must remain on field when replacement is declined"
    );
}

// ---------------------------------------------------------------------------
// 3. predicate:card_in_zone
// ---------------------------------------------------------------------------

const CARD_IN_ZONE_YAML: &str = r#"
card: TGC-ZONE-FILTER
name: ZoneFilterHost
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: on_play
    process:
      - select_zone_cards:
          zones: [battle_area, trash]
          filter:
            all_of:
              - name_is: "TargetMon"
              - card_in_zone: [trash]
          min: 1
          max: 1
          bind_as: picked
          prompt: "Select TargetMon from trash only"
      - place_cards_as_bottom_sources:
          cards: picked
          target: source
"#;

#[test]
fn card_in_zone_restricts_multi_zone_selection_to_specified_zone() {
    let mut target = make_test_card("TGC-TARGET-MON", "TargetMon");
    target.level = Some(3);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(CARD_IN_ZONE_YAML)
        .unwrap()
        .add_card(target)
        .start();
    r.skip_mulligan();
    r.inject_trash(0, "TGC-TARGET-MON");
    r.place_on_field(0, "TGC-TARGET-MON", Some(0));
    let host = r.place_on_field(0, "TGC-ZONE-FILTER", Some(0));

    r.fire_on_play(0, host.index as usize);
    let sel = r
        .pending_selection_view()
        .expect("select_cards should prompt for trash candidate");
    let non_pass: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        non_pass.len(),
        1,
        "only the trash copy should be selectable; battle_area copy must be excluded by card_in_zone"
    );
    r.execute_action(0, non_pass[0]).unwrap();
    r.auto_resolve().unwrap();

    assert_eq!(r.game.players[0].trash.len(), 0, "trash copy moved under host");
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "battle_area copy must remain untouched on field"
    );
    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        2
    );
}

// ---------------------------------------------------------------------------
// 4. predicate:rules_text_contains
// ---------------------------------------------------------------------------

const RULES_TEXT_AURA_YAML: &str = r#"
card: TGC-RULES-AURA
name: RulesAuraHost
kind: digimon
color: [black]
level: 6
cost: 12
dp: 12000
effects:
  - kind: aura
    target:
      of: you
      zone: [battle_area]
      all_of:
        - kind: digimon
        - rules_text_contains: "Knightmon"
    dp_modifier: 3000
"#;

#[test]
fn rules_text_contains_matches_effect_text_only() {
    let mut matching = make_test_card("TGC-HAS-TEXT", "PlainAlly");
    matching.level = Some(4);
    matching.dp = Some(4000);
    matching.effect_text = "[On Play] Search for [Knightmon].".to_string();

    let mut non_matching = make_test_card("TGC-NO-TEXT", "OtherAlly");
    non_matching.level = Some(4);
    non_matching.dp = Some(4000);
    non_matching.effect_text = "[On Play] Draw 1.".to_string();

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(RULES_TEXT_AURA_YAML)
        .unwrap()
        .add_card(matching)
        .add_card(non_matching)
        .start();
    r.skip_mulligan();
    r.place_on_field(0, "TGC-RULES-AURA", Some(0));
    let m_handle = r.place_on_field(0, "TGC-HAS-TEXT", Some(0));
    let n_handle = r.place_on_field(0, "TGC-NO-TEXT", Some(0));
    r.game.tick_declarative_effects();

    assert_eq!(
        r.effective_dp(m_handle),
        Some(7000),
        "Digimon with [Knightmon] in rules text must receive +3000 DP"
    );
    assert_eq!(
        r.effective_dp(n_handle),
        Some(4000),
        "Digimon without [Knightmon] in rules text must not receive aura"
    );
}

// ---------------------------------------------------------------------------
// 5. timing:on_add_to_hand + predicate:event_add_to_hand_player
// ---------------------------------------------------------------------------

const ON_ADD_TO_HAND_YAML: &str = r#"
card: TGC-HAND-WATCHER
name: HandWatcher
kind: tamer
color: [blue]
cost: 4
effects:
  - when: on_add_to_hand
    active_when:
      all_turns: true
    trigger_if:
      all_of:
        - event_add_to_hand_player: opponent
        - event_is_effect_initiated: true
    process:
      - gain_memory: 2
"#;

const OPP_DRAW_EFFECT_YAML: &str = r#"
card: TGC-DRAWER
name: DrawerMon
kind: digimon
color: [blue]
level: 3
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - draw: { of: you, count: 1 }
"#;

#[test]
fn on_add_to_hand_fires_only_for_matching_event_add_to_hand_player() {
    let mut filler = make_test_card("TGC-FILLER", "Filler");
    filler.level = Some(3);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(ON_ADD_TO_HAND_YAML)
        .unwrap()
        .from_dsl_yaml(OPP_DRAW_EFFECT_YAML)
        .unwrap()
        .add_card(filler)
        .deck(0, &["TGC-FILLER", "TGC-FILLER"])
        .deck(1, &["TGC-FILLER", "TGC-FILLER"])
        .start();
    r.skip_mulligan();
    r.game.memory = 0;
    r.place_on_field(0, "TGC-HAND-WATCHER", Some(0));

    // Own effect draw should NOT trigger `event_add_to_hand_player: opponent`.
    let own_drawer = r.place_on_field(0, "TGC-DRAWER", Some(0));
    r.fire_on_play(0, own_drawer.index as usize);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.game.memory, 0,
        "own draw must not trigger opponent-gated on_add_to_hand"
    );

    // Opponent effect draw MUST trigger `event_add_to_hand_player: opponent` and grant P0 +2 memory.
    let opp_drawer = r.place_on_field(1, "TGC-DRAWER", Some(0));
    r.fire_on_play(1, opp_drawer.index as usize);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.game.memory, 2,
        "opponent effect draw must trigger on_add_to_hand and grant controller +2 memory"
    );
}

// ---------------------------------------------------------------------------
// 6. predicate:can_play_to_breeding_from_trash + verb:play_to_breeding_from_trash
// ---------------------------------------------------------------------------

const PLAY_BREEDING_TRASH_YAML: &str = r#"
card: TGC-BREED-REANIMATOR
name: BreedReanimator
kind: digimon
color: [purple]
level: 6
cost: 12
dp: 12000
effects:
  - when: on_play
    condition:
      count_gte:
        filter:
          of: you
          zone: [trash]
          all_of:
            - kind: digimon
            - dp_lte: 5000
            - can_play_to_breeding_from_trash: true
        n: 1
    process:
      - select_trash:
          of: you
          bind_as: pick
          optional: false
          filter:
            all_of:
              - kind: digimon
              - dp_lte: 5000
              - can_play_to_breeding_from_trash: true
          prompt: "Play 1 5000 DP or lower Digimon from trash to empty breeding area"
      - play_to_breeding_from_trash: { of: you, trash_index: pick }
"#;

#[test]
fn play_to_breeding_from_trash_places_card_in_empty_breeding_and_skips_when_occupied() {
    let mut small = make_test_card("TGC-SMALL-TRASH", "SmallTrash");
    small.level = Some(3);
    small.dp = Some(3000);
    small.play_cost = 3;

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(PLAY_BREEDING_TRASH_YAML)
        .unwrap()
        .add_card(small)
        .start();
    r.skip_mulligan();
    r.inject_trash(0, "TGC-SMALL-TRASH");
    r.game.memory = 4;

    // Positive: breeding area is empty.
    let host = r.place_on_field(0, "TGC-BREED-REANIMATOR", Some(0));
    r.fire_on_play(0, host.index as usize);
    let sel = r
        .pending_selection_view()
        .expect("should prompt to select trash card for breeding play");
    let pick = *sel
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("trash card should be selectable");
    r.execute_action(0, pick).unwrap();
    r.auto_resolve().unwrap();

    assert!(r.game.players[0].breeding_area.is_some());
    assert_eq!(
        r.game.players[0]
            .breeding_area
            .as_ref()
            .unwrap()
            .top_card()
            .card_id(&r.game.card_data),
        "TGC-SMALL-TRASH"
    );
    assert_eq!(r.game.memory, 4, "breeding play from trash is free");

    // Negative: breeding area is now occupied; firing again with another trash card must not prompt.
    r.inject_trash(0, "TGC-SMALL-TRASH");
    let host2 = r.place_on_field(0, "TGC-BREED-REANIMATOR", Some(0));
    r.fire_on_play(0, host2.index as usize);
    assert!(
        r.pending_selection_view().is_none(),
        "can_play_to_breeding_from_trash must be false when breeding area is occupied"
    );
}

// ---------------------------------------------------------------------------
// 7. predicate:self_stack_same_level_count_gte
// ---------------------------------------------------------------------------

const SAME_LEVEL_STACK_YAML: &str = r#"
card: TGC-SAME-LVL-HOST
name: SameLevelHost
kind: digimon
color: [black]
level: 4
cost: 5
dp: 5000
effects:
  - when: on_play
    process:
      - if:
          condition:
            self_stack_same_level_count_gte: 2
          then:
            - gain_memory: 2
"#;

#[test]
fn self_stack_same_level_count_gte_distinguishes_same_vs_different_levels() {
    let mut lv4_src = make_test_card("TGC-LV4-SRC", "Lv4Src");
    lv4_src.level = Some(4);
    let mut lv3_src = make_test_card("TGC-LV3-SRC", "Lv3Src");
    lv3_src.level = Some(3);

    let mut r = DebugRunner::builder()
        .from_dsl_yaml(SAME_LEVEL_STACK_YAML)
        .unwrap()
        .add_card(lv4_src)
        .add_card(lv3_src)
        .start();
    r.skip_mulligan();
    r.game.memory = 0;

    // Negative: Lv.3 under Lv.4 -> max same-level count is 1 < 2.
    let diff = r.place_stack(0, &["TGC-LV3-SRC", "TGC-SAME-LVL-HOST"]);
    r.fire_on_play(0, diff.index as usize);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.memory, 0);

    // Positive: Lv.4 under Lv.4 -> same-level count is 2 >= 2.
    let same = r.place_stack(0, &["TGC-LV4-SRC", "TGC-SAME-LVL-HOST"]);
    r.fire_on_play(0, same.index as usize);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.memory, 2);
}

// ---------------------------------------------------------------------------
// 8. predicate:trashed_hand_card_matching
// ---------------------------------------------------------------------------

const TRASHED_HAND_MATCH_YAML: &str = r#"
card: TGC-TRASH-CHECK
name: TrashChecker
kind: digimon
color: [purple]
level: 3
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: discard
          optional: false
          filter: {}
          prompt: "Trash 1 card from hand"
      - trash_from_hand_by_index: { of: you, hand_index: discard }
      - if:
          condition:
            trashed_hand_card_matching:
              trait_has: TS
          then:
            - gain_memory: 2
"#;

#[test]
fn trashed_hand_card_matching_gates_bonus_on_trashed_card_traits() {
    let mut ts_card = make_test_card("TGC-TS-CARD", "TSCard");
    ts_card.traits = vec!["TS".to_string()];
    let mut plain_card = make_test_card("TGC-PLAIN-CARD", "PlainCard");
    plain_card.traits = vec!["Beast".to_string()];

    // Positive case: trashing a [TS] card grants +2 memory.
    {
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(TRASHED_HAND_MATCH_YAML)
            .unwrap()
            .add_card(ts_card.clone())
            .hand(0, &["TGC-TS-CARD"])
            .start();
        r.skip_mulligan();
        r.game.memory = 0;
        let host = r.place_on_field(0, "TGC-TRASH-CHECK", Some(0));
        r.fire_on_play(0, host.index as usize);
        let sel = r.pending_selection_view().expect("hand discard selection");
        assert_eq!(r.game.current_phase, GamePhase::SelectHand);
        let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        r.execute_action(0, pick).unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(r.game.memory, 2, "trashing [TS] card must grant +2 memory");
    }

    // Negative case: trashing a non-[TS] card grants 0 memory.
    {
        let mut r = DebugRunner::builder()
            .from_dsl_yaml(TRASHED_HAND_MATCH_YAML)
            .unwrap()
            .add_card(plain_card)
            .hand(0, &["TGC-PLAIN-CARD"])
            .start();
        r.skip_mulligan();
        r.game.memory = 0;
        let host = r.place_on_field(0, "TGC-TRASH-CHECK", Some(0));
        r.fire_on_play(0, host.index as usize);
        let sel = r.pending_selection_view().expect("hand discard selection");
        let pick = *sel.valid_action_ids.iter().find(|&&a| a != PASS).unwrap();
        r.execute_action(0, pick).unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(
            r.game.memory, 0,
            "trashing non-[TS] card must not grant +2 memory"
        );
    }
}
