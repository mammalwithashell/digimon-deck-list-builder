//! Q7335: printed requirement wording is part of a card's text.
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use digimon_engine::effect_context::EffectReadContext;

#[test]
fn loaded_requirement_only_text_matches_without_becoming_effect_text() {
    for wording in [
        "[Digivolve] [Dracomon]: Cost 0",
        "[DNA Digivolve] [Dracomon] + 1 green Digimon: Cost 0",
        "[DigiXros -2] [Dracomon]",
        "[Burst Digivolve] [Dracomon]",
        "[App Fusion] [Dracomon] + [Other]",
        "[Link] [Dracomon]: Cost 1",
        "[Assembly -1] [Dracomon]",
    ] {
        let raw = serde_json::json!({"REQ": {
            "card_id": "REQ", "card_name_eng": "Plain",
            "card_kind": 0, "card_colors": [1], "xros_req": wording
        }});
        let card = CardData::load_from_str(&raw.to_string()).unwrap().remove("REQ").unwrap();
        let r = DebugRunner::builder().add_card(card).hand(0, &["REQ"]).build();
        let handle = r.game.players[0].hand[0].handle();
        let context = EffectReadContext::new(&r.game, handle, None, 0);
        for (needle, expected) in [("Dracomon", true), ("Examon", false)] {
            let pred = digimon_dsl::compiled::CompiledPredicate {
                in_text_contains: Some(needle.into()), ..Default::default()
            };
            assert_eq!(eval_predicate_with_bindings(&pred, &context, PredicateSubject::Card(handle), None), expected, "{wording}: {needle}");
        }
        let narrow = digimon_dsl::compiled::CompiledPredicate {
            effect_text_contains: Some("Dracomon".into()), ..Default::default()
        };
        assert!(!eval_predicate_with_bindings(&narrow, &context, PredicateSubject::Card(handle), None), "requirement must not be reclassified as effect text");
    }
}
