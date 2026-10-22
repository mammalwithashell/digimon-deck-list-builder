//! BT23-006: image and official bundle verified; DCGO BT23_006.cs cross-checked.

use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "BT23-006";

fn txt(mut c: CardData, name: &str) -> CardData {
    c.card_name = name.to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT23-006 YAML loads")
        .add_card(named(
            "SISTER",
            "Sistermon Blanc",
            CardColor::White,
            3,
            3000,
        ))
        .add_card(named("HUCK-DIGI", "BaoHuckmon", CardColor::Red, 4, 5000))
        .add_card(with_text(
            named("SIS-DIGI", "Angewomon", CardColor::White, 5, 7000),
            "Play 1 [Sistermon Blanc] from your hand.",
        ))
        .add_card(txt(
            tamer("HUCK-TAMER", CardColor::White, 4),
            "Huckmon Tamer",
        ))
        .add_card({
            let mut t = tamer("SIS-TAMER-TXT", CardColor::White, 3);
            t.effect_text = "Search for 1 [Sistermon Ciel].".into();
            t
        })
        .add_card(txt(option("HUCK-OPT", CardColor::Red, 2), "Huckmon Strike"))
        .add_card(txt(tamer("PLAIN-TAMER", CardColor::White, 3), "Tai"))
        .add_card(digimon(
            "KNIGHT",
            CardColor::Red,
            6,
            11000,
            &["Royal Knight"],
        ))
        .add_card(named("PLAIN", "Greymon", CardColor::Red, 4, 5000))
        .add_card(named("WHITE-1", "WhiteOne", CardColor::White, 3, 3000))
        .add_card(named("WHITE-2", "WhiteTwo", CardColor::White, 3, 3000))
        .add_card(named("RED-1", "RedOne", CardColor::Red, 3, 3000))
        .add_card(named("CARRIER", "Carrier", CardColor::Red, 4, 5000))
        .add_card(named("FILL", "Filler", CardColor::Red, 3, 3000))
}

fn on_play_setup(top: &[&str]) -> DebugRunner {
    let deck = deck_with_top(top, "FILL", 4);
    builder()
        .hand(0, &[CARD_ID])
        .deck(0, &deck)
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start()
}

#[test]
fn bt23_006_search_name_filter_excludes_effect_text() {
    let mut r = on_play_setup(&["HUCK-DIGI", "SIS-DIGI", "PLAIN"]);
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(offered_reveal_ids(&r), vec!["HUCK-DIGI"]);
}
#[test]
fn bt23_006_search_two_buckets_and_bottom() {
    let mut r = on_play_setup(&["HUCK-DIGI", "KNIGHT", "PLAIN"]);
    play_card(&mut r, 0, CARD_ID);
    pick_revealed(&mut r, "HUCK-DIGI");
    assert_eq!(offered_reveal_ids(&r), vec!["KNIGHT"]);
    pick_revealed(&mut r, "KNIGHT");
    r.auto_resolve().unwrap();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["HUCK-DIGI", "KNIGHT"]);
    assert_eq!(deck_bottom_ids(&r, 0, 1), vec!["PLAIN"]);
}
#[test]
fn bt23_006_search_accepts_named_non_digimon() {
    let mut r = on_play_setup(&["HUCK-TAMER", "HUCK-OPT", "PLAIN"]);
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(offered_reveal_ids(&r), vec!["HUCK-OPT", "HUCK-TAMER"]);
    pick_revealed(&mut r, "HUCK-OPT");
    r.auto_resolve().unwrap();
    assert_eq!(hand_ids(&r, 0), vec!["HUCK-OPT"]);
    assert_eq!(r.deck_size(0), 6);
}
#[test]
fn bt23_006_search_missing_name_still_offers_knight() {
    let mut r = on_play_setup(&["PLAIN", "KNIGHT", "SIS-DIGI"]);
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(offered_reveal_ids(&r), vec!["KNIGHT"]);
    pick_revealed(&mut r, "KNIGHT");
    r.auto_resolve().unwrap();
    assert_eq!(hand_ids(&r, 0), vec!["KNIGHT"]);
}
#[test]
fn bt23_006_no_match_returns_all_three() {
    let mut r = on_play_setup(&["PLAIN", "PLAIN-TAMER", "FILL"]);
    play_card(&mut r, 0, CARD_ID);
    r.auto_resolve().unwrap();
    assert!(hand_ids(&r, 0).is_empty());
    assert_eq!(r.deck_size(0), 7);
}

fn inherited_setup(hand: &[&str]) -> DebugRunner {
    let mut r = builder()
        .hand(0, hand)
        .hand(1, &["WHITE-1"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    r.place_stack(0, &[CARD_ID, "CARRIER"]);
    r
}

#[test]
fn bt23_006_inherited_white_digimon_played_gains_one_memory() {
    let mut r = inherited_setup(&["WHITE-1"]);
    let mem0 = r.memory();
    play_card(&mut r, 0, "WHITE-1");
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), mem0 - 3 + 1, "paid 3, gained 1");
}

#[test]
fn bt23_006_inherited_non_white_digimon_does_not_gain() {
    let mut r = inherited_setup(&["RED-1"]);
    let mem0 = r.memory();
    play_card(&mut r, 0, "RED-1");
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), mem0 - 3);
}

#[test]
fn bt23_006_inherited_white_tamer_does_not_gain() {
    let mut r = inherited_setup(&["HUCK-TAMER"]);
    let mem0 = r.memory();
    play_card(&mut r, 0, "HUCK-TAMER");
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), mem0 - 4, "Tamers are not Digimon");
}

#[test]
fn bt23_006_inherited_opponents_white_digimon_on_their_turn_does_not_gain() {
    let mut r = inherited_setup(&[]);
    r.end_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.turn_player(), 1);
    r.game.memory = 5;
    play_card(&mut r, 1, "WHITE-1");
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 2, "opponent paid 3 and gained nothing");
}

#[test]
fn bt23_006_inherited_once_per_turn() {
    let mut r = inherited_setup(&["WHITE-1", "WHITE-2"]);
    let mem0 = r.memory();
    play_card(&mut r, 0, "WHITE-1");
    r.auto_resolve().unwrap();
    play_card(&mut r, 0, "WHITE-2");
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), mem0 - 6 + 1, "only the first white play gains");
}

fn digimon(id: &str, color: CardColor, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.colors = vec![color];
    c.dp = Some(dp);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn named(id: &str, name: &str, color: CardColor, level: u8, dp: i32) -> CardData {
    let mut c = digimon(id, color, level, dp, &[]);
    c.card_name = name.to_string();
    c
}

fn with_text(mut c: CardData, text: &str) -> CardData {
    c.effect_text = text.to_string();
    c
}

fn tamer(id: &str, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c.play_cost = cost;
    c
}

fn option(id: &str, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c.play_cost = cost;
    c
}

fn ids(r: &DebugRunner, cards: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].hand)
}

fn hand_index(r: &DebugRunner, p: u8, id: &str) -> usize {
    hand_ids(r, p)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} in player {p}'s hand: {:?}", hand_ids(r, p)))
}

fn play_card(r: &mut DebugRunner, p: u8, id: &str) {
    let slot = hand_index(r, p, id);
    assert!(r.play(p, slot).is_some(), "{id} should be playable");
}

fn offered_reveal_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let mask = digimon_engine::action::mask::build_action_mask(&r.game, view.selecting_player);
    assert_eq!(
        mask[PASS as usize], 0.0,
        "available search picks are mandatory"
    );
    assert!(!view.valid_action_ids.contains(&PASS));
    for &action in &view.valid_action_ids {
        assert!(mask[action as usize] > 0.5);
    }
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| a.checked_sub(SEL_REVEAL_START))
        .filter_map(|i| r.game.revealed_cards.get(i as usize))
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    out.sort();
    out
}

fn pick_revealed(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("reveal prompt pending");
    let want = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .find(|&a| {
            a.checked_sub(SEL_REVEAL_START)
                .and_then(|i| r.game.revealed_cards.get(i as usize))
                .is_some_and(|c| c.card_id(&r.game.card_data) == id)
        })
        .unwrap_or_else(|| panic!("{id} must be a legal reveal pick: {view:?}"));
    r.execute_action(view.selecting_player, want)
        .expect("pick revealed card");
}

fn deck_with_top<'a>(top: &[&'a str], filler: &'a str, n_filler: usize) -> Vec<&'a str> {
    let mut deck: Vec<&str> = vec![filler; n_filler];
    for c in top.iter().rev() {
        deck.push(c);
    }
    deck
}

fn deck_bottom_ids(r: &DebugRunner, p: u8, n: usize) -> Vec<String> {
    ids(r, &r.game.players[p as usize].deck[..n])
}

#[test]
fn bt23_006_digivolution_paths_mask_and_cost() {
    for (color, level, traits, expected) in [
        (CardColor::Red, 2, vec![], true),
        (CardColor::Blue, 2, vec!["CS"], true),
        (CardColor::Blue, 2, vec![], false),
        (CardColor::Blue, 3, vec!["CS"], false),
    ] {
        let mut r = builder()
            .add_card(digimon("BASE", color, level, 1000, &traits))
            .hand(0, &[CARD_ID])
            .deck(0, &["FILL"; 6])
            .deck(1, &["FILL"; 6])
            .memory(5)
            .start();
        r.place_stack(0, &["BASE"]);
        let action = digimon_engine::action::space::encode_digivolve(0, 0);
        let mask = digimon_engine::action::mask::build_action_mask(&r.game, 0);
        assert_eq!(
            mask[action as usize] > 0.5,
            expected,
            "color={color:?} level={level} traits={traits:?}"
        );
        if expected {
            r.game.decode_action(action, 0);
            r.auto_resolve().unwrap();
            assert_eq!(r.memory(), 5, "digivolution costs zero");
            assert_eq!(
                r.game.players[0].battle_area[0]
                    .top_card()
                    .card_id(&r.game.card_data),
                CARD_ID
            );
        }
    }
}

#[test]
fn bt23_006_search_offers_both_name_branches() {
    let mut r = on_play_setup(&["SISTER", "HUCK-DIGI", "KNIGHT"]);
    play_card(&mut r, 0, CARD_ID);
    assert_eq!(offered_reveal_ids(&r), vec!["HUCK-DIGI", "SISTER"]);
    pick_revealed(&mut r, "SISTER");
    pick_revealed(&mut r, "KNIGHT");
    r.auto_resolve().unwrap();
    assert_eq!(deck_bottom_ids(&r, 0, 1), vec!["HUCK-DIGI"]);
}

#[test]
fn bt23_006_inherited_own_white_play_on_opponent_turn_does_not_gain() {
    let mut r = inherited_setup(&[]);
    r.end_turn();
    r.auto_resolve().unwrap();
    assert_eq!(r.turn_player(), 1);
    let h = r.place_stack(0, &["WHITE-1"]);
    let before = r.memory();
    r.fire_play_event_triggers(0, h.index as usize, true, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), before);
}
#[test]
fn bt23_006_inherited_opponent_white_play_on_own_turn_does_not_gain() {
    let mut r = inherited_setup(&[]);
    let h = r.place_stack(1, &["WHITE-1"]);
    let before = r.memory();
    r.fire_play_event_triggers(1, h.index as usize, true, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), before);
}

#[test]
fn bt23_006_parked_white_play_uses_captured_card_and_consumes_both_opt() {
    use digimon_engine::selection::SelectionKind;
    let mut r = inherited_setup(&["WHITE-1", "WHITE-2"]);
    r.place_stack(0, &[CARD_ID, "CARRIER"]);
    play_card(&mut r, 0, "WHITE-1");
    assert_eq!(r.pending_kind(), Some(SelectionKind::TriggerOrder));
    let p = r.pending_selection().unwrap();
    assert_eq!(p.valid_action_ids.len(), 2);
    let mask = digimon_engine::action::mask::build_action_mask(&r.game, 0);
    for &a in &p.valid_action_ids {
        assert_eq!(mask[a as usize], 1.0);
    }
    // Isolate the captured event from the played Digimon's later lifetime.
    // Both inherited sources remain in play while the played Digimon leaves.
    let played = r.game.players[0].battle_area.remove(2);
    assert_eq!(played.top_card().card_id(&r.game.card_data), "WHITE-1");
    r.game.players[0].trash.extend(played.card_sources);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.memory(),
        4,
        "both sources remember the white Digimon play"
    );
    play_card(&mut r, 0, "WHITE-2");
    r.auto_resolve().unwrap();
    assert_eq!(r.memory(), 1, "both once-per-turn triggers were consumed");
}

#[test]
fn bt23_006_inherited_white_token_play_gains_memory_once() {
    let mut r = inherited_setup(&[]);
    for expected in [6, 6] {
        let token = r.place_stack(0, &["TOKEN_ATHO_RENE_POR"]);
        r.fire_play_event_triggers(0, token.index as usize, true, false);
        r.auto_resolve().unwrap();
        assert_eq!(
            r.memory(),
            expected,
            "white Digimon token play respects OPT"
        );
    }
}
