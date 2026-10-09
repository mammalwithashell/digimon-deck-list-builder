//! Link cards in production card data (data/cards.json): a link box is not an
//! inherited effect.
//!
//! general_rule.pdf 4-2-4: a Digimon gains the inherited effects of the
//! digivolution cards under it; 4-2-6: it gains the link effects of its link
//! card; 4-7-2: a link card isn't a stacked card; 2-3-12: link requirements,
//! link DP and link effects are card information of their own, and 2-3-4-2:
//! "a card without an inherited effect section can't be referenced" as having
//! one. Until 2026-10-05 cards.json held every link card's link effect in its
//! inherited text, so a Digimon with a link card among its digivolution cards
//! gained that card's link effect.
//!
//! DebugRunner builds DSL cards with empty text fields, so only tests on the
//! production card data (`deck_tools::full_card_data`) can see this.

use digimon_dsl::compiled::CompiledPredicate;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::Keyword;
use digimon_engine::permanent::PermanentHandle;

fn production_card(card_id: &str) -> CardData {
    digimon_engine::deck_tools::full_card_data()
        .get(card_id)
        .unwrap_or_else(|| panic!("{card_id} in production cards.json"))
        .clone()
}

/// A plain Digimon whose only digivolution card is the production `card_id`.
fn digimon_over(card_id: &str) -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .add_card(production_card(card_id))
        .add_card(make_test_card("TOP-DGM", "Top Digimon"))
        .start();
    let stack = runner.place_stack(0, &[card_id, "TOP-DGM"]);
    (runner, stack)
}

/// A plain Digimon with the production `card_id` linked to it.
fn digimon_linked_with(card_id: &str) -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .add_card(production_card(card_id))
        .add_card(make_test_card("HOST-DGM", "Host Digimon"))
        .start();
    let host = runner.place_on_field(0, "HOST-DGM", Some(0));
    runner.push_linked_owned(host, card_id, 0);
    runner.game.tick_declarative_effects();
    (runner, host)
}

// ─── A link card under a Digimon gives it nothing ───────────────────────────

/// BT21-009 Gatchmon prints <Raid> in its link box and has no inherited box.
#[test]
fn gatchmon_as_a_digivolution_card_gives_no_raid() {
    let (runner, stack) = digimon_over("BT21-009");
    assert!(
        !runner.game.has_keyword(stack, Keyword::Raid),
        "a Digimon with BT21-009 Gatchmon under it must not gain its link box's <Raid>"
    );
}

/// BT23-022 Oujamon prints <Security A. +1> in its link box (no DSL YAML, so
/// production reads only the printed text).
#[test]
fn oujamon_as_a_digivolution_card_adds_no_security_check() {
    let (runner, stack) = digimon_over("BT23-022");
    assert_eq!(
        runner.game.security_attack_keyword_bonus(stack),
        0,
        "a Digimon with BT23-022 Oujamon under it must not gain its link box's <Security A. +1>"
    );
}

// ─── A link card linked to a Digimon still gives it its link effect ─────────

/// BT23-007 Musclemon's link box is <Piercing>; it has no DSL YAML, so the
/// host's keyword comes from the printed link text alone.
#[test]
fn musclemon_as_a_link_card_gives_piercing() {
    let (runner, host) = digimon_linked_with("BT23-007");
    assert!(
        runner.game.has_keyword(host, Keyword::Piercing),
        "a Digimon linked with BT23-007 Musclemon gains its link box's <Piercing>"
    );
}

#[test]
fn gatchmon_as_a_link_card_gives_raid() {
    let (runner, host) = digimon_linked_with("BT21-009");
    assert!(
        runner.game.has_keyword(host, Keyword::Raid),
        "a Digimon linked with BT21-009 Gatchmon gains its link box's <Raid>"
    );
}

/// BT24-067 Hackmon's link box is <Retaliation>, which its DSL YAML also grants
/// (`scope: linked`). With the printed link text loaded as well, the Digimon it
/// is linked to retaliates exactly once: a second trigger would open an
/// ordering prompt (general_rule.pdf 16-12).
#[test]
fn hackmon_as_a_link_card_retaliates_once() {
    let mut big = make_test_card("BIG-DGM", "Big Digimon");
    big.dp = Some(8000);
    let mut runner = DebugRunner::builder()
        .add_card(production_card("BT24-067"))
        .add_card(make_test_card("HOST-DGM", "Host Digimon"))
        .add_card(big)
        .start();
    let host = runner.place_on_field(0, "HOST-DGM", Some(0));
    runner.push_linked_owned(host, "BT24-067", 0);
    runner.game.tick_declarative_effects();
    let big = runner.place_on_field(1, "BIG-DGM", Some(0));
    runner.game.players[1].battle_area[big.index as usize].is_suspended = true;

    // 2000 DP + 2000 link DP against 8000: only the host is deleted in battle.
    runner.attack_digimon(host, big, false);

    assert!(
        runner.pending_selection().is_none(),
        "one <Retaliation> trigger, so no ordering prompt: {:?}",
        runner.pending_kind()
    );
    assert!(runner.game.players[0].battle_area.is_empty(), "the host lost the battle");
    assert!(
        runner.game.players[1].battle_area.is_empty(),
        "<Retaliation> deletes the Digimon the host battled"
    );
}

// ─── A link box's replacement keyword (BT22-075 Fakemon: <Scapegoat>) ──────
//
// Fakemon has no DSL YAML, so its link-box <Scapegoat> exists in production
// only as printed link text. general_rule.pdf 16-31: when the Digimon would be
// deleted other than by your effects, by deleting 1 of your other Digimon, it
// isn't deleted (an optional replacement).

fn fakemon_runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(production_card("BT22-075"))
        .add_card(make_test_card("HOST-DGM", "Host Digimon"))
        .add_card(make_test_card("ALLY-DGM", "Ally Digimon"))
        .start()
}

#[test]
fn fakemon_as_a_link_card_gives_scapegoat() {
    use digimon_engine::replacement::ReplacementCause;
    let mut runner = fakemon_runner();
    let host = runner.place_on_field(0, "HOST-DGM", Some(0));
    runner.place_on_field(0, "ALLY-DGM", Some(0));
    runner.push_linked_owned(host, "BT22-075", 0);
    runner.game.tick_declarative_effects();

    runner.game.delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.as_ref().is_some_and(|p| p.is_optional),
        "a Digimon linked with BT22-075 Fakemon may use the link box's <Scapegoat>"
    );
}

#[test]
fn fakemon_as_a_digivolution_card_gives_no_scapegoat() {
    use digimon_engine::replacement::ReplacementCause;
    let mut runner = fakemon_runner();
    let stack = runner.place_stack(0, &["BT22-075", "HOST-DGM"]);
    runner.place_on_field(0, "ALLY-DGM", Some(0));

    runner.game.delete_permanent_with_cause(stack, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "a Digimon with Fakemon under it doesn't gain its link-box <Scapegoat>: {:?}",
        runner.pending_kind()
    );
}

#[test]
fn fakemon_on_its_own_has_no_scapegoat() {
    use digimon_engine::replacement::ReplacementCause;
    let mut runner = fakemon_runner();
    let fakemon = runner.place_on_field(0, "BT22-075", Some(0));
    runner.place_on_field(0, "ALLY-DGM", Some(0));

    runner.game.delete_permanent_with_cause(fakemon, ReplacementCause::OpponentEffect);

    assert!(
        runner.game.pending_selection.is_none(),
        "Fakemon's link-box <Scapegoat> is not its own effect: {:?}",
        runner.pending_kind()
    );
    let left: Vec<&str> = runner.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data))
        .collect();
    assert_eq!(left, vec!["ALLY-DGM"], "Fakemon is deleted");
}

// ─── "with inherited effects" ───────────────────────────────────────────────

/// The DSL's `has_inherited: {}` leaf is "a card with inherited effects"
/// (DCGO `HasInheritedEffect`). A link card has no inherited effect section.
#[test]
fn a_link_card_is_not_a_card_with_inherited_effects() {
    let runner = DebugRunner::builder()
        .add_card(production_card("BT21-009"))
        .add_card(production_card("BT25-100"))
        .add_card(production_card("ST1-03"))
        .hand(0, &["BT21-009", "BT25-100", "ST1-03"])
        .start();
    let with_inherited = CompiledPredicate {
        has_inherited: Some(Box::new(CompiledPredicate::default())),
        ..Default::default()
    };
    let hand = &runner.game.players[0].hand;
    let ctx = EffectReadContext::new(&runner.game, hand[0].handle(), None, 0);
    let matches = |i: usize| eval_predicate(&with_inherited, &ctx, PredicateSubject::Card(hand[i].handle()));

    assert!(matches(2), "control: ST1-03 Agumon prints an inherited effect");
    assert!(!matches(0), "BT21-009 Gatchmon (Digimon) prints a link box, not an inherited effect");
    assert!(!matches(1), "BT25-100 Iron Slash (Option) prints a link box, not an inherited effect");
}
