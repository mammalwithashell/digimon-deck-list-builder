//! Zone-aware play / use candidate leaves (G-DSL-PLAY-CANDIDATE-ORIGIN-ZONE).
//!
//! `can_play_from_effect_without_cost` and `can_play_or_use_from_effect` now
//! read the candidate's zone: a card in its owner's trash fails while that
//! player can't play cards from the trash (`CannotPlayFromTrash`, e.g. BT23-014
//! "can't play Digimon or Tamers from the trash"), because the trash-origin
//! play (`Game::trash_play_refusal`) and Option use (`Game::use_option_from`)
//! refuse it. A "from your hand or trash" union pick (`select_union_zone`, ~50
//! cards: ST19-08, EX11-022, BT22-098, ...) shares one filter across both
//! origins, so without this the trash copy would be a dead choice (CR 4.3
//! 1-3-2). The hand copy is unaffected.
use digimon_engine::{
    action::space::{PLAY_HAND_START, TRASH_EFFECT_START},
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, Expiry, GamePhase, ModifierType},
    modifiers::PlayerModifierEntry,
    permanent::PermanentHandle,
};

const UNION_PLAY: &str = r#"
card: ZONE-PLAY-SOURCE
name: Zone play source
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          bind_as: chosen
          optional: true
          cost: false
          filter: {can_play_from_effect_without_cost: true}
          prompt: Play 1 card from your hand or trash without paying the cost
      - if:
          condition: {binding_present: chosen}
          then:
            - play_union_bound_free: {binding: chosen}
"#;

const UNION_USE: &str = r#"
card: ZONE-PLAY-SOURCE
name: Zone use source
kind: tamer
color: [white]
cost: 0
effects:
  - when: on_play
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          bind_as: chosen
          optional: true
          cost: false
          filter:
            all_of:
              - kind: option
              - can_play_or_use_from_effect: true
          prompt: Use 1 Option card from your hand or trash without paying the cost
      - if:
          condition: {binding_present: chosen}
          then:
            - use_option_bound: {binding: chosen}
"#;

fn fixture(yaml: &str, card: &str) -> (DebugRunner, PermanentHandle) {
    let mut digimon = make_test_card("ZP-DIGIMON", "ZP-DIGIMON");
    digimon.card_kind = CardKind::Digimon;
    digimon.level = Some(3);
    digimon.play_cost = 3;
    let mut option = make_test_card("ZP-OPTION", "ZP-OPTION");
    option.card_kind = CardKind::Option;
    option.level = None;
    option.dp = None;
    option.play_cost = 2;
    option.colors = vec![CardColor::White];
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(digimon)
        .add_card(option)
        .hand(0, &[])
        .hand(1, &[])
        .deck(0, &["ZP-DIGIMON"; 20])
        .deck(1, &["ZP-DIGIMON"; 20])
        .memory(3)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.set_phase(GamePhase::Main);
    r.add_to_hand(0, card);
    r.inject_trash(0, card);
    let source = r.place_on_field(0, "ZONE-PLAY-SOURCE", None);
    (r, source)
}

fn lock_trash(r: &mut DebugRunner) {
    r.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayFromTrash, 0, Expiry::Permanent, None, 1),
    );
}

/// Offered candidates in fixture coordinates (hand index / TRASH_EFFECT_START
/// + trash index); empty when no prompt opened.
fn offered(r: &mut DebugRunner, source: PermanentHandle) -> Vec<u16> {
    r.fire_on_play(source.player, source.index as usize);
    if r.game.pending_selection.is_none() {
        return Vec::new();
    }
    let mut ids = crate::union_fixture_candidates(&r.game);
    ids.sort();
    ids
}

#[test]
fn union_play_offers_hand_and_trash_copies_without_the_lock() {
    let (mut r, source) = fixture(UNION_PLAY, "ZP-DIGIMON");
    let trash = TRASH_EFFECT_START + (r.game.players[0].trash.len() as u16 - 1);
    assert_eq!(offered(&mut r, source), vec![PLAY_HAND_START, trash]);
}

#[test]
fn union_play_drops_the_trash_copy_under_cannot_play_from_trash() {
    let (mut r, source) = fixture(UNION_PLAY, "ZP-DIGIMON");
    lock_trash(&mut r);
    assert_eq!(offered(&mut r, source), vec![PLAY_HAND_START], "the hand copy stays playable");
}

#[test]
fn union_play_opens_nothing_when_only_a_locked_trash_copy_qualifies() {
    let (mut r, source) = fixture(UNION_PLAY, "ZP-DIGIMON");
    lock_trash(&mut r);
    r.game.players[0].hand.clear();
    assert!(offered(&mut r, source).is_empty());
}

#[test]
fn the_commit_refuses_the_locked_trash_copy_the_filter_now_drops() {
    // A filter without the leaf still offers the trash copy; choosing it plays
    // nothing, which is the dead choice the leaf removes.
    let yaml = UNION_PLAY.replace(
        "filter: {can_play_from_effect_without_cost: true}",
        "filter: {kind: digimon}",
    );
    let (mut r, source) = fixture(&yaml, "ZP-DIGIMON");
    lock_trash(&mut r);
    let ids = offered(&mut r, source);
    let trash = *ids.iter().find(|&&id| id >= TRASH_EFFECT_START).expect("trash copy offered");
    let action = crate::union_fixture_action(&r.game, trash);
    r.execute_action(0, action).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.battle_area_size(0), 1, "only the source: nothing was played from the trash");
    assert!(r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "ZP-DIGIMON"));
}

#[test]
fn union_use_drops_the_trash_option_under_cannot_play_from_trash() {
    let (mut r, source) = fixture(UNION_USE, "ZP-OPTION");
    let trash = TRASH_EFFECT_START + (r.game.players[0].trash.len() as u16 - 1);
    let mut unlocked = DebugRunner::wrap(r.game.clone());
    assert_eq!(offered(&mut unlocked, source), vec![PLAY_HAND_START, trash]);
    lock_trash(&mut r);
    assert_eq!(offered(&mut r, source), vec![PLAY_HAND_START]);
}
