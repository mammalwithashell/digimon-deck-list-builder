//! `link_requirement_met_by_self`: the subject card's [Link] requirement is
//! met by the effect's own permanent, link cost ignored
//! (G-DSL-LINK-REQUIREMENT-MET-BY-SELF; DCGO `CanLinkToTargetPermanent(
//! permanent, payCost: false)`, the Logimon (BT25-052) [Main] offer gate).
use digimon_dsl::compiled::CompiledPredicate;
use digimon_engine::{
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::predicate::{eval_predicate, PredicateSubject},
    effect_context::EffectReadContext,
    enums::CardKind,
    permanent::PermanentHandle,
};

fn digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

const NEEDS_BEAST: &str = r#"
card: TEST-LINK-NEEDS-BEAST
name: Beast Linker
kind: digimon
level: 3
color: [green]
cost: 3
dp: 2000
effects:
  - kind: link_condition
    cost: 9
    filter: { trait_has: Beast }
"#;

fn runner() -> (DebugRunner, PermanentHandle) {
    let mut beast = digimon("BEAST-HOST");
    beast.traits = vec!["Beast".into()];
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(NEEDS_BEAST)
        .expect("fixture compiles")
        .link_requirement("ANY-LINK", 7)
        .add_card(digimon("ANY-LINK"))
        .add_card(digimon("NO-LINK"))
        .add_card(digimon("PLAIN-HOST"))
        .add_card(beast)
        .hand(0, &["ANY-LINK", "NO-LINK", "TEST-LINK-NEEDS-BEAST"])
        .deck(0, &["NO-LINK"; 6])
        .deck(1, &["NO-LINK"; 6])
        .memory(0)
        .start();
    r.skip_mulligan();
    let host = r.place_on_field(0, "PLAIN-HOST", Some(0));
    (r, host)
}

fn hand(r: &DebugRunner, id: &str) -> CardHandle {
    r.game.players[0]
        .hand
        .iter()
        .find(|c| c.card_id(&r.game.card_data) == id)
        .expect("in hand")
        .handle()
}

fn meets(r: &DebugRunner, host: PermanentHandle, card: CardHandle) -> bool {
    let top = r.game.players[0].battle_area[host.index as usize].top_card().handle();
    eval_predicate(
        &CompiledPredicate { link_requirement_met_by_self: Some(true), ..Default::default() },
        &EffectReadContext::new(&r.game, top, Some(host), 0),
        PredicateSubject::Card(card),
    )
}

#[test]
fn link_requirement_met_by_self_reads_the_requirement_against_the_effect_permanent() {
    let (mut r, plain) = runner();
    assert!(meets(&r, plain, hand(&r, "ANY-LINK")), "an unfiltered [Link] requirement is met");
    assert!(!meets(&r, plain, hand(&r, "NO-LINK")), "no [Link] at all (Q4533)");
    assert!(!meets(&r, plain, hand(&r, "TEST-LINK-NEEDS-BEAST")), "the host is not [Beast]");
    let beast = r.place_on_field(0, "BEAST-HOST", Some(0));
    assert!(meets(&r, beast, hand(&r, "TEST-LINK-NEEDS-BEAST")), "a [Beast] host meets it");
}

#[test]
fn link_requirement_met_by_self_ignores_the_link_cost() {
    // Link cost 7 from memory 0; the leaf never looks at the payment.
    let (r, plain) = runner();
    assert!(meets(&r, plain, hand(&r, "ANY-LINK")));
}

#[test]
fn link_requirement_met_by_self_is_false_without_a_source_permanent() {
    let (r, _plain) = runner();
    let card = hand(&r, "ANY-LINK");
    assert!(!eval_predicate(
        &CompiledPredicate { link_requirement_met_by_self: Some(true), ..Default::default() },
        &EffectReadContext::new(&r.game, card, None, 0),
        PredicateSubject::Card(card),
    ));
}
