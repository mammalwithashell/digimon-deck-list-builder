//! LM-067 Gundramon (dual) — card-text tests.
//!
//! Digimon face: <Reboot> <Blocker> <Fragment (3)>. [When Digivolving] [Counter]
//! [Once Per Turn] Reveal the top 6 cards of your deck. You may play or use 1
//! play or use cost 6 or lower [Three Musketeers] text card among them without
//! paying the cost. Return the rest to the bottom of the deck. [When
//! Digivolving] [When Attacking] By trashing up to 3 [Three Musketeers] text
//! cards from any of your Digimon's digivolution cards, for each one trashed,
//! delete 1 of your opponent's play cost 7 or lower Digimon.
//! Option face (use 6): <Use Req. ([Three Musketeers] in text)> [Main]
//! <De-Digivolve 3> 3 of your opponent's Digimon. Then, you may place 1 [Three
//! Musketeers] trait card from your hand or trash as any of your Digimon's
//! bottom digivolution card. Arts Digivolve.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

const CARD_ID: &str = "LM-067";

fn filler() -> CardData {
    let mut c = make_test_card("SEC-OPT", "Security Option Filler");
    c.card_kind = CardKind::Option;
    c
}

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("LM-067 YAML parses and compiles")
        .add_card(filler())
        .deck(0, &["SEC-OPT"; 10])
        .deck(1, &["SEC-OPT"; 10])
        .security(0, &["SEC-OPT"; 3])
        .security(1, &["SEC-OPT"; 3])
        .memory(5)
        .start()
}

fn triggered_with(
    effects: &[CompiledClause],
    a: CompiledTiming,
    b: CompiledTiming,
) -> Option<&digimon_dsl::compiled::CompiledTriggeredClause> {
    effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.when.contains(&a) && t.when.contains(&b) => Some(t),
        _ => None,
    })
}

#[test]
fn lm_067_metadata() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Gundramon");
    assert_eq!(card.kind, CompiledCardKind::Dual);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(13000));
    for t in ["Machine", "Three Musketeers", "Titan", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
    let dual = card.dual.as_ref().expect("dual");
    assert_eq!(dual.option.use_cost, 6);
    assert!(dual.option.keywords.iter().any(|k| k == "ArtsDigivolve"));
}

#[test]
fn lm_067_wd_counter_reveal_six_is_once_per_turn() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let dual = card.dual.as_ref().expect("dual");
    let t = triggered_with(
        &dual.digimon.effects,
        CompiledTiming::WhenDigivolving,
        CompiledTiming::Counter,
    )
    .expect("WD/Counter clause");
    assert!(t.once_per_turn);
    let body = format!("{:?}", t.process);
    assert!(body.contains("RevealTopDeck"), "reveals the top cards");
    assert!(body.contains("UseOptionFromRevealed"), "may use an Option");
    assert!(body.contains("PlayFromRevealedFree"), "may play a card");
}

#[test]
fn lm_067_wd_wa_source_trash_deletes_per_card() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let dual = card.dual.as_ref().expect("dual");
    let t = triggered_with(
        &dual.digimon.effects,
        CompiledTiming::WhenDigivolving,
        CompiledTiming::WhenAttacking,
    )
    .expect("WD/WA clause");
    assert!(!t.once_per_turn);
    let body = format!("{:?}", t.process);
    assert!(body.contains("SelectOwnSources"), "chooses up to 3 sources");
    assert!(body.contains("DeletePermanent"), "deletes per trashed card");
}

#[test]
fn lm_067_option_main_de_digivolves_three() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let dual = card.dual.as_ref().expect("dual");
    let main = dual
        .option
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::Main) => Some(t),
            _ => None,
        })
        .expect("option [Main]");
    let body = format!("{:?}", main.process);
    assert!(body.contains("DeDigivolve"), "<De-Digivolve 3>");
    assert!(body.contains("PlaceAsBottomSource"), "may place a [Three Musketeers] card");
}
