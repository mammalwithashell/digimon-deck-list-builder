//! A DSL card's traits in DebugRunner are its whole printed trait line — form,
//! attribute and type (general_rule.pdf 2-3-2-1: "multiple traits including
//! form, attribute, and type") — as they are in production, where
//! `CardData::load_from_str` builds `traits` from cards.json's
//! `form_eng + attribute_eng + type_eng`. A "[Vaccine] trait" effect
//! (`trait_has: Vaccine`) must therefore see a card's YAML `attribute:` and
//! `form:` lines, not only its `traits:` line.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::permanent::PermanentHandle;

/// [On Play] Select 1 of your Digimon matching `filter`.
fn seeker(filter: &str) -> String {
    format!(
        r#"
card: TRAIT-SEEKER
name: Trait Seeker
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
effects:
  - when: on_play
    summary: "Select 1 of your Digimon"
    process:
      - select_own_permanent:
          bind_as: target
          filter: {{ {filter} }}
          prompt: "Select 1 of your Digimon"
"#
    )
}

/// A Lv.4 Digimon whose trait lines (`form:` / `attribute:` / `traits:`) are
/// `trait_lines`.
fn digimon(card_id: &str, trait_lines: &str) -> String {
    format!(
        "card: {card_id}\nname: {card_id}\nkind: digimon\nlevel: 4\ncolor: [white]\n\
         cost: 4\ndp: 4000\n{trait_lines}\n"
    )
}

/// Plays the seeker for `filter` with a card printing `matching` and one
/// printing `other` on the field, and reports whether each is offered.
fn offered(filter: &str, matching: &str, other: &str) -> (bool, bool) {
    offered_after(filter, matching, other, |_, _| {})
}

/// `offered`, with `prepare` run on the runner and the matching card's
/// permanent before the seeker is played.
fn offered_after(
    filter: &str,
    matching: &str,
    other: &str,
    prepare: impl FnOnce(&mut DebugRunner, PermanentHandle),
) -> (bool, bool) {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(&seeker(filter))
        .expect("seeker compiles")
        .from_dsl_yaml(&digimon("LINE-MATCH", matching))
        .expect("matching card compiles")
        .from_dsl_yaml(&digimon("LINE-OTHER", other))
        .expect("other card compiles")
        .hand(0, &["TRAIT-SEEKER"])
        .memory(10)
        .start();
    let matching = runner.place_on_field(0, "LINE-MATCH", Some(0));
    let other = runner.place_on_field(0, "LINE-OTHER", Some(0));
    prepare(&mut runner, matching);

    runner.play(0, 0).expect("play Trait Seeker");

    let targets = runner
        .pending_selection_view()
        .map(|view| view.valid_action_ids)
        .unwrap_or_default();
    (
        targets.contains(&encode_permanent(matching)),
        targets.contains(&encode_permanent(other)),
    )
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    digimon_engine::action::space::encode_attack(0, handle.index as u16)
}

#[test]
fn trait_has_matches_a_dsl_card_by_its_yaml_attribute() {
    let (vaccine, virus) = offered(
        "trait_has: Vaccine",
        "traits: [Dragon]\nattribute: Vaccine",
        "traits: [Dragon]\nattribute: Virus",
    );
    assert!(
        vaccine,
        "a card printing the Vaccine attribute has the [Vaccine] trait"
    );
    assert!(!virus, "a Virus card does not have the [Vaccine] trait");
}

#[test]
fn trait_has_matches_a_dsl_card_by_its_yaml_form() {
    let (hybrid, champion) = offered(
        "trait_has: Hybrid",
        "traits: [Warrior]\nform: Hybrid",
        "traits: [Warrior]\nform: Champion",
    );
    assert!(
        hybrid,
        "a card printing the Hybrid form has the [Hybrid] trait"
    );
    assert!(
        !champion,
        "a Champion card does not have the [Hybrid] trait"
    );
}

/// A rule box can give a card a second attribute — EX13-066 prints Virus and
/// "(Rule) Also treated as ... Trait: [Data] Attribute." — so `attribute:`
/// takes a list, like cards.json's `attribute_eng`.
#[test]
fn trait_has_matches_every_attribute_of_a_dsl_card_listing_two() {
    let (both, virus) = offered(
        "trait_has: Data",
        "traits: [Puppet]\nattribute: [Virus, Data]",
        "traits: [Puppet]\nattribute: Virus",
    );
    assert!(
        both,
        "a card with the Virus and Data attributes has the [Data] trait"
    );
    assert!(!virus, "a Virus-only card does not have the [Data] trait");
}

/// `attribute_is` / `form_is` name a trait by the line it is printed on, and
/// printed text reaches either only as "the [X] trait" (general_rule.pdf
/// 2-3-2-3) — BT16-077 and EX3-008 filter "[Free] trait" cards with
/// `attribute_is: Free` — so they match the trait line like `trait_has`.
#[test]
fn attribute_is_matches_a_dsl_card_by_its_attribute() {
    let (free, vaccine) = offered(
        "attribute_is: Free",
        "traits: [Mutant]\nattribute: Free",
        "traits: [Mutant]\nattribute: Vaccine",
    );
    assert!(
        free,
        "a card printing the Free attribute matches `attribute_is: Free`"
    );
    assert!(!vaccine, "a Vaccine card does not");
}

#[test]
fn form_is_matches_a_dsl_card_by_its_form() {
    let (hybrid, champion) = offered(
        "form_is: Hybrid",
        "traits: [Warrior]\nform: Hybrid",
        "traits: [Warrior]\nform: Champion",
    );
    assert!(
        hybrid,
        "a card printing the Hybrid form matches `form_is: Hybrid`"
    );
    assert!(!champion, "a Champion card does not");
}

/// Like `trait_has`, `attribute_is` sees a trait an effect gives a Digimon
/// ("gains the [Free] trait" — a `ChangeTraits` modifier).
#[test]
fn attribute_is_matches_a_trait_an_effect_grants() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};

    let (granted, vaccine) = offered_after(
        "attribute_is: Free",
        "traits: [Mutant]\nattribute: Vaccine",
        "traits: [Mutant]\nattribute: Vaccine",
        |runner, matching| {
            runner.game.modifiers.add(
                matching,
                ModifierEntry::simple(ModifierType::ChangeTraits, 0, Expiry::Permanent, 0)
                    .with_payload(ModifierPayload::Traits {
                        add: vec!["Free".to_string()],
                        replace: false,
                    }),
            );
        },
    );
    assert!(
        granted,
        "a Digimon given the [Free] trait matches `attribute_is: Free`"
    );
    assert!(!vaccine, "a Vaccine Digimon without it does not");
}
