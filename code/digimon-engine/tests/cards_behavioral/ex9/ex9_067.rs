//! EX9-067 Mirai Kinosaki.
//! Printed text covered here: [On Play] Reveal the top 3 cards of your deck.
//! Add 1 card with the [Puppet] or [LIBERATOR] trait among them to the hand.
//! Return the rest to the bottom of the deck.
//!
//! Security: Play this card without paying the cost.
//!
//! [Your Turn] When any of your Digimon digivolve into a [Puppet] trait Digimon,
//! by returning this Tamer to the bottom of the deck, you may play 1 [Arisa
//! Kinosaki] or 1 [Puppet] trait Digimon card from your hand with the play cost
//! reduced by 3. (DCGO EX9_067.cs, "Your Turn" region; CR 4.3 for the
//! triggered-effect timing.) Covered by the `your_turn_*` tests below.

use digimon_engine::action::space::{PASS, PLAY_HAND_START, SEL_REVEAL_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlaySource};

#[test]
fn ex9_067_on_play_adds_one_puppet_or_liberator_and_bottoms_rest() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-067")
        .expect("EX9-067 YAML loads")
        .add_card(make_trait_card("PUPPET", &["Puppet"]))
        .add_card(make_trait_card("LIBERATOR", &["LIBERATOR"]))
        .add_card(make_test_card("BLANK", "Blank"))
        .deck(0, &["BLANK", "LIBERATOR", "PUPPET"])
        .hand(0, &["EX9-067"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Mirai");

    let view = runner.pending_selection_view().expect("reveal pick");
    assert!(
        !runner.pending_is_optional(),
        "printed add-1 search is mandatory when a match exists"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&digimon_engine::action::space::PASS),
        "PASS is not legal for the mandatory add-1 search"
    );
    assert!(view
        .valid_action_ids
        .contains(&revealed_action_for_id(&runner, "PUPPET")));
    assert!(view
        .valid_action_ids
        .contains(&revealed_action_for_id(&runner, "LIBERATOR")));
    runner
        .execute_action(0, revealed_action_for_id(&runner, "PUPPET"))
        .expect("pick Puppet");
    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"PUPPET".to_string()));
    assert!(!hand_ids.contains(&"LIBERATOR".to_string()));
    assert_eq!(
        runner.game.players[0]
            .deck
            .first()
            .unwrap()
            .card_id(&runner.game.card_data),
        "BLANK",
        "unchosen matching cards return to deck bottom with the rest"
    );
}

#[test]
fn ex9_067_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-067")
        .expect("EX9-067 YAML loads")
        .add_card(attacker)
        .security(1, &["EX9-067"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "EX9-067"),
        "Mirai is played from the defender's security"
    );
}

fn make_trait_card(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .expect("revealed card exists")
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

// ─── [Your Turn] Puppet-digivolve observer ───────────────────────────────────

fn digimon(id: &str, level: u8, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    if level == 4 {
        c.evo_costs = vec![EvoCost {
            card_color: 0,
            level: 3,
            memory_cost: 2,
        }];
    }
    c
}

fn arisa() -> CardData {
    let mut c = make_test_card("ARISA", "Arisa Kinosaki");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c
}

fn puppet_runner(hand: &[&str]) -> DebugRunner {
    let mut b: DebugRunnerBuilder = DebugRunner::builder()
        .dsl_card("EX9-067")
        .expect("EX9-067 YAML loads")
        .add_card(digimon("BASE", 3, 3, &[]))
        .add_card(digimon("PUP-EVO", 4, 4, &["Puppet"]))
        .add_card(digimon("PLAIN-EVO", 4, 4, &[]))
        .add_card(digimon("PUP-HAND", 4, 6, &["Puppet"]))
        .add_card(digimon("PLAIN-HAND", 4, 6, &[]))
        .add_card(arisa())
        .add_card(make_test_card("FILL", "Filler"));
    b = b.hand(0, hand).deck(0, &["FILL"; 6]).deck(1, &["FILL"; 6]).memory(10);
    b.start()
}

/// Board: own turn, Mirai + a Lv3 base. The digivolve is performed by the test.
fn board(r: &mut DebugRunner) {
    r.place_on_field(0, "EX9-067", Some(0));
    r.place_on_field(0, "BASE", Some(0));
}

fn digivolve(r: &mut DebugRunner, hand_idx: usize, how: PlaySource) {
    // slot 0 = Mirai (Tamer), slot 1 = BASE; locate BASE by id.
    let base = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "BASE")
        .expect("base on field");
    assert!(r.game.digivolve_from_hand(0, hand_idx, base, how));
}

fn field_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    zone_ids(&r.game.players[0].hand, &r.game.card_data)
}

/// Hand ids offered by the pending HAND pick (PASS excluded).
fn offered_hand_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("hand pick pending");
    assert_eq!(view.kind, digimon_engine::selection::SelectionKind::Hand);
    let hand = hand_ids(r);
    let mut ids: Vec<String> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .map(|a| hand[(a - PLAY_HAND_START) as usize].clone())
        .collect();
    ids.sort();
    ids
}

fn pick_from_hand(r: &mut DebugRunner, id: &str) {
    let idx = hand_ids(r)
        .iter()
        .position(|h| h == id)
        .expect("card in hand");
    r.execute_action(0, PLAY_HAND_START + idx as u16)
        .expect("pick hand card");
}

#[test]
fn your_turn_digivolve_into_puppet_returns_tamer_and_plays_puppet_at_minus_3() {
    let mut r = puppet_runner(&["PUP-EVO", "PUP-HAND", "PLAIN-HAND"]);
    board(&mut r);
    digivolve(&mut r, 0, PlaySource::ByDigivolve);
    // Optional trigger: accept, then the bounce + hand pick.
    r.accept_optional_trigger().expect("accept Mirai's optional clause");
    assert!(
        !field_ids(&r).contains(&"EX9-067".to_string()),
        "the Tamer is returned to the deck as the cost"
    );
    assert_eq!(
        r.game.players[0].deck.first().unwrap().card_id(&r.game.card_data),
        "EX9-067",
        "returned to the BOTTOM of the deck"
    );
    // Only the Puppet Digimon is a legal pick (not PLAIN-HAND).
    let view = r.pending_selection_view().expect("hand pick");
    assert!(view.is_optional, "DCGO canNoSelect: true");
    assert_eq!(
        offered_hand_ids(&r),
        vec!["PUP-HAND".to_string()],
        "a non-Puppet Digimon is not eligible"
    );
    let before = r.memory();
    pick_from_hand(&mut r, "PUP-HAND");
    r.auto_resolve().ok();
    assert!(field_ids(&r).contains(&"PUP-HAND".to_string()), "played");
    assert_eq!(r.memory(), before - 3, "play cost 6 reduced by 3");
}

#[test]
fn your_turn_effect_initiated_digivolve_into_puppet_also_triggers() {
    let mut r = puppet_runner(&["PUP-EVO", "PUP-HAND"]);
    board(&mut r);
    digivolve(&mut r, 0, PlaySource::ByEffect);
    r.accept_optional_trigger()
        .expect("effect-initiated digivolves trigger Mirai too");
    pick_from_hand(&mut r, "PUP-HAND");
    r.auto_resolve().ok();
    assert!(field_ids(&r).contains(&"PUP-HAND".to_string()));
}

#[test]
fn your_turn_arisa_kinosaki_tamer_is_a_legal_pick_at_minus_3() {
    let mut r = puppet_runner(&["PUP-EVO", "ARISA"]);
    board(&mut r);
    digivolve(&mut r, 0, PlaySource::ByDigivolve);
    r.accept_optional_trigger().unwrap();
    let before = r.memory();
    pick_from_hand(&mut r, "ARISA");
    r.auto_resolve().ok();
    assert!(field_ids(&r).contains(&"ARISA".to_string()));
    assert_eq!(r.memory(), before, "Arisa cost 3 reduced by 3 = free");
}

#[test]
fn your_turn_decline_keeps_tamer_and_plays_nothing() {
    let mut r = puppet_runner(&["PUP-EVO", "PUP-HAND"]);
    board(&mut r);
    digivolve(&mut r, 0, PlaySource::ByDigivolve);
    r.decline_optional_trigger().expect("decline");
    r.auto_resolve().ok();
    assert!(field_ids(&r).contains(&"EX9-067".to_string()));
    assert!(!field_ids(&r).contains(&"PUP-HAND".to_string()));
}

#[test]
fn your_turn_accept_then_decline_pick_still_pays_the_return_cost() {
    let mut r = puppet_runner(&["PUP-EVO", "PUP-HAND"]);
    board(&mut r);
    digivolve(&mut r, 0, PlaySource::ByDigivolve);
    r.accept_optional_trigger().unwrap();
    r.execute_action(0, PASS).expect("no card selected (canNoSelect)");
    r.auto_resolve().ok();
    assert!(!field_ids(&r).contains(&"EX9-067".to_string()));
    assert!(!field_ids(&r).contains(&"PUP-HAND".to_string()));
}

#[test]
fn your_turn_digivolve_into_non_puppet_does_not_trigger() {
    let mut r = puppet_runner(&["PLAIN-EVO", "PUP-HAND"]);
    board(&mut r);
    digivolve(&mut r, 0, PlaySource::ByDigivolve);
    r.auto_resolve().ok();
    assert!(r.pending_selection().is_none());
    assert!(field_ids(&r).contains(&"EX9-067".to_string()));
}

#[test]
fn your_turn_only_not_on_opponents_turn() {
    let mut r = puppet_runner(&["PUP-EVO", "PUP-HAND"]);
    board(&mut r);
    r.set_first_player(1);
    digivolve(&mut r, 0, PlaySource::ByEffect);
    r.auto_resolve().ok();
    assert!(r.pending_selection().is_none(), "[Your Turn] only");
    assert!(field_ids(&r).contains(&"EX9-067".to_string()));
}
