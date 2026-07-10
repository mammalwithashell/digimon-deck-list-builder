//! Generated rules-FAQ conformance scenarios.
//!
//! Source data: `qa/rules/faq_conformance_cases.json`.
//! Regenerate with:
//! `python code/tools/faq_conformance_generator.py --check` or without
//! `--check` to rewrite this file.

#![allow(clippy::needless_update)]

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind};

struct TextScopeCase<'a> {
    id: &'a str,
    source: &'a str,
    ruling_text: &'a str,
    card_id: &'a str,
    card_name: &'a str,
    card_kind: CardKind,
    level: Option<u8>,
    dp: Option<i32>,
    play_cost: u16,
    colors: &'a [CardColor],
    traits: &'a [&'a str],
    effect_text: &'a str,
    inherited_text: &'a str,
    security_text: &'a str,
    rule_text: &'a str,
    requirement_text: &'a str,
    positive_needles: &'a [&'a str],
    negative_needles: &'a [&'a str],
}

fn synthetic_card(case: &TextScopeCase<'_>) -> CardData {
    CardData {
        card_id: case.card_id.to_string(),
        card_name: case.card_name.to_string(),
        card_kind: case.card_kind,
        level: case.level,
        dp: case.dp,
        play_cost: case.play_cost,
        colors: case.colors.to_vec(),
        traits: case.traits.iter().map(|trait_name| (*trait_name).to_string()).collect(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: case.effect_text.to_string(),
        inherited_text: case.inherited_text.to_string(),
        security_text: case.security_text.to_string(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: case.card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn text_scope_contains(case: &TextScopeCase<'_>, card: &CardData, needle: &str) -> bool {
    let needle = needle.to_lowercase();
    let haystacks = [
        card.card_name.as_str(),
        card.effect_text.as_str(),
        card.inherited_text.as_str(),
        card.security_text.as_str(),
        case.rule_text,
        case.requirement_text,
    ];
    haystacks
        .iter()
        .any(|field| field.to_lowercase().contains(&needle))
        || card
            .traits
            .iter()
            .any(|trait_name| trait_name.to_lowercase().contains(&needle))
}

fn run_text_scope_case(case: &TextScopeCase<'_>) {
    let mut runner = DebugRunner::builder()
        .add_card(synthetic_card(case))
        .start();
    let _handle = runner.place_on_field(0, case.card_id, Some(0));
    let card = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == case.card_id)
        .expect("generated FAQ card should be present");

    for needle in case.positive_needles {
        assert!(
            text_scope_contains(case, card, needle),
            "FAQ conformance {id} from {source}: expected {card_id} to match              `{needle}` under ruling: {ruling}",
            id = case.id,
            source = case.source,
            card_id = case.card_id,
            needle = needle,
            ruling = case.ruling_text
        );
    }

    for needle in case.negative_needles {
        assert!(
            !text_scope_contains(case, card, needle),
            "FAQ conformance {id} from {source}: did not expect {card_id} to              match `{needle}` under ruling: {ruling}",
            id = case.id,
            source = case.source,
            card_id = case.card_id,
            needle = needle,
            ruling = case.ruling_text
        );
    }
}

#[test]
fn bt25_082_in_text_scope_store_champs() {
    let case = TextScopeCase {
        id: "bt25_082_in_text_scope_store_champs",
        source: "data/card_bundles/BT25-082.md",
        ruling_text: "It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolution requirements, DNA digivolution, DigiXros requirements, burst digivolve, App Fusion, Link, or Assembly requirements. For example, a card with [Knightmon] in its text would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.",
        card_id: "BT25-082",
        card_name: "BlackGatomon",
        card_kind: CardKind::Digimon,
        level: Some(4u8),
        dp: Some(6000i32),
        play_cost: 6u16,
        colors: &[CardColor::Purple, CardColor::Black],
        traits: &["Dark Animal", "Iliad", "TS"],
        effect_text: "[On Play] [When Digivolving] If you have 1 or fewer Tamers, you may play 1 Tamer card with [Three Musketeers] in its text from your hand without paying the cost. [All Turns] While you have a Tamer with [Three Musketeers] in its text, this Digimon may digivolve into a [Three Musketeers] trait Digimon card for a digivolution cost of 4, ignoring digivolution requirements.",
        inherited_text: "[When Attacking] [Once Per Turn] By placing 1 [Three Musketeers] trait card from your hand or trash as this Digimon's bottom digivolution card, <Draw 1>.",
        security_text: "",
        rule_text: "",
        requirement_text: "[Digivolve] Lv.3 w/[Three Musketeers] in text or w/[TS] trait: Cost 2",
        positive_needles: &["BlackGatomon", "Three Musketeers", "TS"],
        negative_needles: &["Gammamon"],
    };
    run_text_scope_case(&case);
}

#[test]
fn ex12_018_in_text_scope_mirrored() {
    let case = TextScopeCase {
        id: "ex12_018_in_text_scope_mirrored",
        source: "data/card_bundles/EX12-018.md",
        ruling_text: "It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolve requirements, DNA Digivolve, DigiXros requirements, Burst Digivolve, App Fusion, Link, or Assembly requirements. For example, \"a card with [Knightmon] in its text\" would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.",
        card_id: "EX12-018",
        card_name: "Siriusmon",
        card_kind: CardKind::Digimon,
        level: Some(6u8),
        dp: Some(12000i32),
        play_cost: 12u16,
        colors: &[CardColor::Red, CardColor::Yellow],
        traits: &["Light Dragon", "VB"],
        effect_text: "<Progress> <Piercing> <Security A. +1> [When Digivolving] [When Attacking] [Once Per Turn] You may place up to 2 Digimon cards with [Gammamon] in their texts or the [VB] trait from your hand or trash as this Digimon's top or bottom digivolution cards.",
        inherited_text: "",
        security_text: "",
        rule_text: "<Arts Digivolve> (Instead of trashing after use, your cards may digivolve into this card without paying the cost.)",
        requirement_text: "[Digivolve] Lv.5 w/[Gammamon] in text or w/[VB] trait: Cost 3",
        positive_needles: &["Siriusmon", "Gammamon", "VB", "Arts Digivolve"],
        negative_needles: &["Three Musketeers"],
    };
    run_text_scope_case(&case);
}

#[test]
fn ex12_048_in_text_scope_mirrored() {
    let case = TextScopeCase {
        id: "ex12_048_in_text_scope_mirrored",
        source: "data/card_bundles/EX12-048.md",
        ruling_text: "It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolution requirements, DNA digivolution, DigiXros requirements, burst digivolve, App Fusion, Link, or Assembly requirements. For example, a card with [Knightmon] in its text would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.",
        card_id: "EX12-048",
        card_name: "SeitenGokuumon",
        card_kind: CardKind::Digimon,
        level: Some(6u8),
        dp: Some(13000i32),
        play_cost: 13u16,
        colors: &[CardColor::Yellow, CardColor::Red, CardColor::Blue],
        traits: &["Tathagata", "Shambala", "SW"],
        effect_text: "<Rush> <Raid> <Piercing> <Security A. +1> [On Play] [When Digivolving] 1 of your opponent's Digimon gets -8000 DP until their turn ends. [All Turns] When this Digimon would leave the battle area other than by your effects, you may play 2 level 5 cards with [Gokuumon] in their texts or the [SW] trait from its digivolution cards without paying the costs.",
        inherited_text: "",
        security_text: "",
        rule_text: "Assembly -6: 3 [Gokuumon]/[Sagomon]/[Cho-Hakkaimon]/[Sanzomon] w/different names",
        requirement_text: "[Digivolve] Lv.5 w/[Gokuumon] in text or w/[Shambala] trait: Cost 4",
        positive_needles: &["SeitenGokuumon", "Gokuumon", "Shambala", "Assembly"],
        negative_needles: &["Gammamon"],
    };
    run_text_scope_case(&case);
}
