//! `can_play_or_use_from_effect` (G-DSL-PLAY-OR-USE-CANDIDATE-LEGALITY).
//!
//! A "play or use 1 ... card from your hand" pick (ST24-06 RizeGreymon) offers
//! only cards the effect controller can actually play or use right now. DCGO
//! ST24_06.cs: `(IsOption && !CanNotPlayThisOption) || (HasPlayCost &&
//! CanPlayAsNewPermanent(card, false, activateClass))`. A Digi-Egg has no play
//! cost and is never played (CR 4.3 7-1-1; it is hatched, 4-18); a Digimon /
//! Tamer needs a free battle-area slot and no effect-play prohibition; an
//! Option needs no "can't use Option cards" lock.
use digimon_engine::{
    action::space::{MAX_FIELD_SLOTS, PASS, PLAY_HAND_START, TRASH_EFFECT_START},
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, CostDelta, Expiry, GamePhase, ModifierType},
    selection::OptionPlayResult,
    modifiers::PlayerModifierEntry,
    permanent::PermanentHandle,
};

const PLAY_OR_USE: &str = r#"
card: PLAY-OR-USE-SOURCE
name: Play or use source
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: chosen
          optional: true
          filter: {can_play_or_use_from_effect: true}
          prompt: Play or use 1 card from your hand without paying the cost
      - if:
          condition: {binding_present: chosen}
          then:
            - play_or_use_from_hand: {of: you, hand_index: chosen, cost_delta: free}
"#;

/// Hand order: 0 Digimon, 1 Digi-Egg, 2 Option, 3 Tamer.
const HAND: [&str; 4] = ["PU-DIGIMON", "PU-EGG", "PU-OPTION", "PU-TAMER"];

/// A green Option: nothing on the field is green, so its colour requirement is
/// unmet (CR 4.3 4-22-2). Not in `HAND`; tests add it explicitly.
const OFF_COLOUR: &str = "PU-GREEN-OPTION";

fn fixture(yaml: &str) -> (DebugRunner, PermanentHandle) {
    let mut builder = DebugRunner::builder().from_dsl_yaml(yaml).unwrap();
    for (id, kind) in [
        ("PU-DIGIMON", CardKind::Digimon),
        ("PU-EGG", CardKind::DigiEgg),
        ("PU-OPTION", CardKind::Option),
        ("PU-TAMER", CardKind::Tamer),
        (OFF_COLOUR, CardKind::Option),
    ] {
        let mut card = make_test_card(id, id);
        card.card_kind = kind;
        // The source Tamer is white, so a white Option meets its colour
        // requirement (CR 4.3 4-22-2); the off-colour Option is green.
        if id == "PU-OPTION" {
            card.colors = vec![CardColor::White];
        } else if id == OFF_COLOUR {
            card.colors = vec![CardColor::Green];
        }
        match kind {
            CardKind::Digimon => {
                card.level = Some(3);
                card.dp = Some(2000);
                card.play_cost = 4;
            }
            CardKind::DigiEgg => {
                card.level = Some(2);
                card.dp = None;
                card.play_cost = 0;
            }
            _ => {
                card.level = None;
                card.dp = None;
                card.play_cost = 4;
            }
        }
        builder = builder.add_card(card);
    }
    let mut r = builder
        .hand(0, &[])
        .hand(1, &[])
        .deck(0, &["PU-DIGIMON"; 20])
        .deck(1, &["PU-DIGIMON"; 20])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.set_phase(GamePhase::Main);
    for id in HAND {
        r.add_to_hand(0, id);
    }
    let source = r.place_on_field(0, "PLAY-OR-USE-SOURCE", None);
    (r, source)
}

fn forbid(r: &mut DebugRunner, kind: ModifierType) {
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(kind, 0, Expiry::Permanent, None, 1),
    );
}

/// Fire the source and return the hand indices offered (PASS excluded).
fn offered(r: &mut DebugRunner, source: PermanentHandle) -> Vec<usize> {
    r.fire_on_play(source.player, source.index as usize);
    let Some(pending) = r.game.pending_selection.as_ref() else {
        return Vec::new();
    };
    assert!(pending.is_optional, "'you may play or use' is declinable");
    pending
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| (id - PLAY_HAND_START) as usize)
        .collect()
}

fn on_field(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

fn in_hand(r: &DebugRunner, id: &str) -> bool {
    r.game.players[0]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

#[test]
fn play_or_use_candidates_exclude_the_digi_egg() {
    let (mut r, source) = fixture(PLAY_OR_USE);
    assert_eq!(offered(&mut r, source), vec![0, 2, 3], "Digimon, Option and Tamer; never the Digi-Egg");
}

#[test]
fn play_or_use_commits_a_played_digimon_and_a_used_option_for_free() {
    for (pick, id) in [(0usize, "PU-DIGIMON"), (2, "PU-OPTION")] {
        let (mut r, source) = fixture(PLAY_OR_USE);
        assert!(offered(&mut r, source).contains(&pick));
        let memory = r.memory();
        r.execute_action(0, PLAY_HAND_START + pick as u16).unwrap();
        let _ = r.auto_resolve();
        assert!(!in_hand(&r, id), "{id} left the hand");
        assert_eq!(on_field(&r, id), id == "PU-DIGIMON", "{id}: played vs used");
        assert_eq!(r.memory(), memory, "free: no memory paid");
    }
}

#[test]
fn play_or_use_candidates_respect_each_prohibition() {
    for (kind, expected) in [
        (ModifierType::CannotUseOptionCards, vec![0, 3]),
        (ModifierType::CannotPlayDigimonByEffect, vec![2, 3]),
        (ModifierType::CannotPlayTamerByEffect, vec![0, 2]),
    ] {
        let (mut r, source) = fixture(PLAY_OR_USE);
        forbid(&mut r, kind);
        assert_eq!(offered(&mut r, source), expected, "{kind:?}");
    }
}

#[test]
fn play_or_use_full_battle_area_offers_only_the_option() {
    let (mut r, source) = fixture(PLAY_OR_USE);
    while r.battle_area_size(0) < MAX_FIELD_SLOTS as usize {
        r.place_on_field(0, "PU-DIGIMON", None);
    }
    assert_eq!(offered(&mut r, source), vec![2], "no free slot: only the Option is usable");
}

#[test]
fn play_or_use_false_polarity_selects_only_the_egg_and_commit_still_refuses_it() {
    let yaml = PLAY_OR_USE.replace(
        "can_play_or_use_from_effect: true",
        "can_play_or_use_from_effect: false",
    );
    let (mut r, source) = fixture(&yaml);
    assert_eq!(offered(&mut r, source), vec![1]);
    let field = r.battle_area_size(0);
    r.execute_action(0, PLAY_HAND_START + 1).unwrap();
    let _ = r.auto_resolve();
    assert!(in_hand(&r, "PU-EGG"), "a Digi-Egg is never played from hand");
    assert_eq!(r.battle_area_size(0), field);
}

// ─── Option colour requirement (CR 4.3 4-22-1..3) ───────────────────────────
// An effect-driven use is a use: the colour requirement still applies (only
// 4-22-5, activating an Option's effect WITHOUT using it, is exempt). Official
// rulings Q1500 (BT6-112) and Q2785 (BT17-035). DCGO: the pick offers
// `IsOption && !CanNotPlayThisOption`, and `CanNotPlayThisOption`
// (CardSource.cs:184-246) includes `!MatchColorRequirement`
// (CardSource.cs:253-309); `PlayOptionCards` re-filters on it for every use,
// paid or free (CardEffectCommons.cs:67-69).

#[test]
fn play_or_use_candidates_exclude_an_option_whose_colour_requirement_is_unmet() {
    let (mut r, source) = fixture(PLAY_OR_USE);
    r.add_to_hand(0, OFF_COLOUR);
    assert_eq!(
        offered(&mut r, source),
        vec![0, 2, 3],
        "the green Option (index 4) can't be used with only a white Tamer on the field"
    );
}

/// "Ignore colour requirements" (DCGO `IIgnoreColorConditionEffect`, e.g. the
/// temporary one Taomon BT10_039.cs:77-80 installs) is the engine's
/// player-scoped `IgnoreColorRequirement`: with it the off-colour Option is
/// offered again.
#[test]
fn play_or_use_candidates_include_the_off_colour_option_when_colour_is_ignored() {
    let (mut r, source) = fixture(PLAY_OR_USE);
    r.add_to_hand(0, OFF_COLOUR);
    forbid(&mut r, ModifierType::IgnoreColorRequirement);
    assert_eq!(offered(&mut r, source), vec![0, 2, 3, 4]);
}

/// A pick that does not filter on legality (any Option) still can't commit an
/// off-colour use: the card stays in hand and no memory moves.
const PLAY_OR_USE_ANY_OPTION: &str = r#"
card: PLAY-OR-USE-SOURCE
name: Play or use source
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: chosen
          optional: true
          filter: {kind: option}
          prompt: Use 1 Option card from your hand without paying the cost
      - if:
          condition: {binding_present: chosen}
          then:
            - play_or_use_from_hand: {of: you, hand_index: chosen, cost_delta: free}
"#;

#[test]
fn play_or_use_commit_refuses_an_off_colour_option() {
    let (mut r, source) = fixture(PLAY_OR_USE_ANY_OPTION);
    r.add_to_hand(0, OFF_COLOUR);
    assert_eq!(offered(&mut r, source), vec![2, 4], "the filter admits both Options");
    let memory = r.memory();
    r.execute_action(0, PLAY_HAND_START + 4).unwrap();
    let _ = r.auto_resolve();
    assert!(in_hand(&r, OFF_COLOUR), "an off-colour Option can't be used, even for free");
    assert_eq!(r.memory(), memory);
}

/// The engine's effect-driven use entry point refuses an off-colour Option
/// under every cost policy (free, reduced, fixed), and allows a matching one.
#[test]
fn effect_driven_option_use_enforces_the_colour_requirement_for_every_cost_policy() {
    for delta in [CostDelta::Free, CostDelta::Reduce(2), CostDelta::Fixed(0)] {
        let (mut r, _source) = fixture(PLAY_OR_USE);
        r.add_to_hand(0, OFF_COLOUR);
        let idx = r.game.players[0]
            .hand
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == OFF_COLOUR)
            .unwrap();
        let result = r.game.use_option_from_hand_with_cost(0, idx, delta);
        assert_eq!(result, OptionPlayResult::Invalid, "{delta:?}: off-colour use refused");
        assert!(in_hand(&r, OFF_COLOUR), "{delta:?}: the card stays in hand");

        let idx = r.game.players[0]
            .hand
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == "PU-OPTION")
            .unwrap();
        let result = r.game.use_option_from_hand_with_cost(0, idx, delta);
        assert_ne!(result, OptionPlayResult::Invalid, "{delta:?}: the white Option is usable");
        assert!(!in_hand(&r, "PU-OPTION"), "{delta:?}: the white Option was used");
    }
}

// The dedicated Option-use verbs offer only usable Options too: an off-colour
// candidate would be a dead pick (the commit refuses it). DCGO filters both the
// hand pick (e.g. EX7_073.cs:62-70, BT10_039.cs:42-52) and `PlayOptionCards`
// on `!CanNotPlayThisOption`.

const USE_FROM_HAND: &str = r#"
card: PLAY-OR-USE-SOURCE
name: Use option source
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - use_option_from_hand:
          of: you
          optional: true
          filter: {kind: option}
          prompt: Use 1 Option card from your hand without paying the cost
"#;

const USE_FROM_TRASH: &str = r#"
card: PLAY-OR-USE-SOURCE
name: Use option source
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - use_option_from_trash:
          of: you
          optional: true
          filter: {kind: option}
          cost: { reduce: 4 }
          prompt: Use 1 Option card from your trash with the cost reduced by 4
"#;

fn offered_ids(r: &mut DebugRunner, source: PermanentHandle, zone_start: u16, trash: bool) -> Vec<String> {
    r.fire_on_play(source.player, source.index as usize);
    let Some(pending) = r.game.pending_selection.as_ref() else {
        return Vec::new();
    };
    let zone = if trash { &r.game.players[0].trash } else { &r.game.players[0].hand };
    let ids: Vec<String> = zone
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    let mut out: Vec<String> = pending
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&id| id != PASS)
        .map(|id| ids[(id - zone_start) as usize].clone())
        .collect();
    out.sort();
    out
}

#[test]
fn use_option_from_hand_does_not_offer_an_off_colour_option() {
    let (mut r, source) = fixture(USE_FROM_HAND);
    r.add_to_hand(0, OFF_COLOUR);
    assert_eq!(
        offered_ids(&mut r, source, PLAY_HAND_START, false),
        vec!["PU-OPTION".to_string()],
        "the green Option is not usable with only a white Tamer on the field"
    );
}

#[test]
fn use_option_from_trash_does_not_offer_an_off_colour_option() {
    let (mut r, source) = fixture(USE_FROM_TRASH);
    r.add_to_hand(0, OFF_COLOUR);
    for id in ["PU-OPTION", OFF_COLOUR] {
        let i = r.game.players[0]
            .hand
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == id)
            .unwrap();
        let card = r.game.players[0].hand.remove(i);
        r.game.players[0].trash.push(card);
    }
    assert_eq!(
        offered_ids(&mut r, source, TRASH_EFFECT_START, true),
        vec!["PU-OPTION".to_string()],
        "the green Option is not usable with only a white Tamer on the field"
    );
}
