//! ST6 Starter Deck Venomous Violet coverage.
//!
//! These tests start with embedded-pack presence for the full starter-deck card
//! pool, then add per-card behavioral assertions as each YAML spec lands.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCard, CompiledCardKind, CompiledClause, CompiledColor,
    CompiledCost, CompiledCountBound, CompiledDeclarativeClause, CompiledDpConstraint,
    CompiledScope, CompiledStep, CompiledTiming, CompiledZone,
};
use digimon_engine::debug_runner::DebugRunner;

pub(crate) const ST6_CARD_IDS: [&str; 16] = [
    "ST6-01", "ST6-02", "ST6-03", "ST6-04", "ST6-05", "ST6-06", "ST6-07", "ST6-08", "ST6-09",
    "ST6-10", "ST6-11", "ST6-12", "ST6-13", "ST6-14", "ST6-15", "ST6-16",
];

fn st6_runner() -> DebugRunner {
    let mut builder = DebugRunner::builder().memory(10);
    for card_id in ST6_CARD_IDS {
        builder = builder
            .dsl_card(card_id)
            .unwrap_or_else(|err| panic!("{card_id} must be in embedded DSL pack: {err}"));
    }
    builder.start()
}

fn compiled(card_id: &str) -> CompiledCard {
    st6_runner()
        .compiled_card(card_id)
        .unwrap_or_else(|| panic!("{card_id} must be compiled"))
        .clone()
}

fn has_purple_evo_path(card: &CompiledCard, from_level: u8, cost: i32) -> bool {
    card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(cost))
            && path.from.as_ref().is_some_and(|predicate| {
                predicate.level_eq == Some(from_level)
                    && predicate.color_is == Some(CompiledColor::Purple)
            })
    })
}

fn triggered(
    card: &CompiledCard,
    timing: CompiledTiming,
) -> Vec<&digimon_dsl::compiled::CompiledTriggeredClause> {
    card.effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) if triggered.when.contains(&timing) => {
                Some(triggered)
            }
            _ => None,
        })
        .collect()
}

fn any_step(steps: &[CompiledStep], predicate: &impl Fn(&CompiledStep) -> bool) -> bool {
    steps.iter().any(|step| {
        predicate(step)
            || match step {
                CompiledStep::If {
                    then, else_branch, ..
                } => any_step(then, predicate) || any_step(else_branch, predicate),
                CompiledStep::ForEach { body, .. } => any_step(body, predicate),
                CompiledStep::PerSelected { body, .. } => any_step(body, predicate),
                CompiledStep::SelectOwnSources { then, .. } => any_step(then, predicate),
                CompiledStep::SelectOpponentSources { then, .. } => any_step(then, predicate),
                CompiledStep::SelectUnionZone { then, .. } => any_step(then, predicate),
                CompiledStep::Optional(inner) => any_step(inner, predicate),
                _ => false,
            }
    })
}

#[test]
fn st6_full_card_pool_is_in_embedded_dsl_pack() {
    let runner = st6_runner();

    for card_id in ST6_CARD_IDS {
        assert!(
            runner.compiled_card(card_id).is_some(),
            "{card_id} must compile from YAML into the embedded DSL pack"
        );
    }
}

#[test]
fn st6_vanilla_cards_have_printed_metadata_and_evolution_costs() {
    let cases = [
        ("ST6-02", "DemiDevimon", 3, 2, 4000, "Evil", 2, 1),
        ("ST6-05", "Elecmon", 3, 4, 5000, "Mammal", 2, 0),
        ("ST6-07", "Youkomon", 4, 5, 6000, "Mysterious Beast", 3, 2),
        ("ST6-09", "Kyukimon", 5, 6, 9000, "Mysterious Beast", 4, 3),
    ];

    for (card_id, name, level, cost, dp, trait_name, from_level, evo_cost) in cases {
        let card = compiled(card_id);
        assert_eq!(card.name, name);
        assert_eq!(card.kind, CompiledCardKind::Digimon);
        assert_eq!(card.level, Some(level));
        assert_eq!(card.color, vec![CompiledColor::Purple]);
        assert_eq!(card.cost, Some(cost));
        assert_eq!(card.dp, Some(dp));
        assert!(card.traits.contains(&trait_name.to_string()));
        assert!(
            has_purple_evo_path(&card, from_level, evo_cost),
            "{card_id} must retain its printed purple digivolution path"
        );
        assert!(
            card.effects.is_empty(),
            "{card_id} is printed vanilla and must not gain effects"
        );
    }
}

#[test]
fn st6_inherited_trash_draw_and_dp_effects_are_authored() {
    let pagumon = compiled("ST6-01");
    let pagumon_on_deletion = triggered(&pagumon, CompiledTiming::OnDeletion)
        .into_iter()
        .find(|clause| clause.scope == CompiledScope::Inherited)
        .expect("ST6-01 inherited On Deletion clause");
    assert!(
        any_step(&pagumon_on_deletion.process, &|step| matches!(
            step,
            CompiledStep::TrashFromTop { count: 2, .. }
        )),
        "ST6-01 must trash top 2 cards from deck"
    );

    for card_id in ["ST6-03", "ST6-06"] {
        let card = compiled(card_id);
        let when_attacking = triggered(&card, CompiledTiming::WhenAttacking)
            .into_iter()
            .find(|clause| clause.scope == CompiledScope::Inherited)
            .unwrap_or_else(|| panic!("{card_id} inherited When Attacking clause"));
        assert!(
            any_step(&when_attacking.process, &|step| matches!(
                step,
                CompiledStep::Draw { count: 1, .. }
            )),
            "{card_id} must draw 1"
        );
        assert!(
            any_step(&when_attacking.process, &|step| matches!(
                step,
                CompiledStep::SelectHand {
                    optional: false,
                    ..
                }
            )),
            "{card_id} must require a hand card to trash after drawing"
        );
        assert!(
            any_step(&when_attacking.process, &|step| matches!(
                step,
                CompiledStep::TrashFromHandByIndex { .. }
            )),
            "{card_id} must trash the selected hand card"
        );
    }

    let weregarurumon = compiled("ST6-11");
    let aura = weregarurumon
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                active_when,
                dp_modifier,
                ..
            }) if *scope == CompiledScope::Inherited => Some((active_when, dp_modifier)),
            _ => None,
        })
        .expect("ST6-11 inherited DP aura");
    assert_eq!(*aura.1, Some(2000));
    let condition = aura.0.as_ref().expect("ST6-11 must require trash count");
    assert!(
        condition
            .all_of
            .iter()
            .any(|part| part.your_turn == Some(true)),
        "ST6-11 aura must apply only on your turn"
    );
    assert!(
        condition.all_of.iter().any(|part| {
            part.count_gte.as_ref().is_some_and(|count| {
                count.n == CompiledDpConstraint::Literal(5)
                    && count.filter.zone == vec![CompiledZone::Trash]
            })
        }),
        "ST6-11 aura must require 5 or more cards in your trash"
    );
}

#[test]
fn st6_keyword_recursion_and_digiburst_shapes_are_authored() {
    let devimon = compiled("ST6-08");
    assert!(devimon.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) if keyword == "Blocker"
        )
    }));
    let attack = triggered(&devimon, CompiledTiming::WhenAttacking)
        .pop()
        .expect("ST6-08 When Attacking clause");
    assert!(any_step(&attack.process, &|step| matches!(
        step,
        CompiledStep::LoseMemory(2)
    )));

    for card_id in ["ST6-04", "ST6-10"] {
        let card = compiled(card_id);
        let timing = if card_id == "ST6-04" {
            CompiledTiming::OnPlay
        } else {
            CompiledTiming::WhenDigivolving
        };
        let clause = triggered(&card, timing)
            .pop()
            .unwrap_or_else(|| panic!("{card_id} recursion clause"));
        assert!(clause.optional, "{card_id} printed recursion is optional");
        assert!(any_step(&clause.process, &|step| matches!(
            step,
            CompiledStep::SelectTrash { optional: true, .. }
        )));
        assert!(any_step(&clause.process, &|step| matches!(
            step,
            CompiledStep::AddToHandFromTrash { .. }
        )));
    }

    let venom = compiled("ST6-12");
    let wd = triggered(&venom, CompiledTiming::WhenDigivolving)
        .pop()
        .expect("ST6-12 When Digivolving clause");
    assert!(any_step(&wd.process, &|step| matches!(
        step,
        CompiledStep::SelectCountCappedMulti {
            max: CompiledCountBound::Literal(2),
            optional_zero: true,
            ..
        }
    )));
    assert!(any_step(&wd.process, &|step| matches!(
        step,
        CompiledStep::GrantKeyword {
            keyword,
            expiry,
            ..
        } if keyword == "Retaliation" && expiry == "end_of_opponents_next_turn"
    )));

    let cres = compiled("ST6-13");
    assert!(cres.effects.iter().any(|clause| {
        matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                value,
                ..
            }) if keyword == "SecurityAttackPlus" && *value == Some(1)
        )
    }));
    let main = triggered(&cres, CompiledTiming::MainOnField)
        .pop()
        .expect("ST6-13 Main Digi-Burst clause");
    assert!(any_step(&main.process, &|step| matches!(
        step,
        CompiledStep::SelectOwnSources { min: 2, max: 2, .. }
    )));
    assert!(any_step(&main.process, &|step| matches!(
        step,
        CompiledStep::TrashSelectedSources { .. }
    )));
    assert!(any_step(&main.process, &|step| matches!(
        step,
        CompiledStep::PlayFromTrashFree {
            suppress_on_play: false,
            ..
        }
    )));

    // The action bar derives its short effect tag from the [Main] clause's
    // authored `summary` (lowered to `Effect.name` at `lower_triggered.rs`,
    // then surfaced as `ActionExplanation.effect_name`). Assert the headline
    // CresGarurumon Digi-Burst clause carries that name source so the button
    // can read "CresGarurumon: Digi-Burst 2".
    let summary = main
        .summary
        .as_deref()
        .expect("ST6-13 [Main] clause carries a summary (the effect-name source)");
    assert!(
        summary.contains("Digi-Burst 2"),
        "summary feeds the action-bar effect name: {summary}"
    );
}

#[test]
fn st6_tamer_and_option_security_shapes_are_authored() {
    let matt = compiled("ST6-14");
    let deletion = triggered(&matt, CompiledTiming::OnAnyDeletion)
        .pop()
        .expect("ST6-14 deletion observer");
    assert!(deletion.optional);
    assert!(
        deletion
            .active_when
            .as_ref()
            .is_some_and(|predicate| predicate.your_turn == Some(true)),
        "ST6-14 must be scoped to your turn"
    );
    assert!(any_step(&deletion.process, &|step| matches!(
        step,
        CompiledStep::ActivationCost { .. }
    )));
    assert!(any_step(&deletion.process, &|step| matches!(
        step,
        CompiledStep::GainMemory(1)
    )));
    assert!(any_step(
        &triggered(&matt, CompiledTiming::OnSecurity)
            .pop()
            .expect("ST6-14 Security clause")
            .process,
        &|step| matches!(step, CompiledStep::PlayFromSecurity)
    ));

    let death_claw = compiled("ST6-15");
    let main = triggered(&death_claw, CompiledTiming::MainFromHand)
        .pop()
        .expect("ST6-15 Main clause");
    assert!(any_step(&main.process, &|step| matches!(
        step,
        CompiledStep::SelectOwnPermanent { optional: true, .. }
    )));
    assert!(any_step(&main.process, &|step| matches!(
        step,
        CompiledStep::SelectOpponentPermanent {
            optional: false,
            ..
        }
    )));
    assert!(any_step(&main.process, &|step| matches!(
        step,
        CompiledStep::DeletePermanent { .. }
    )));
    let security = triggered(&death_claw, CompiledTiming::OnSecurity)
        .pop()
        .expect("ST6-15 Security clause");
    assert_eq!(security.scope, CompiledScope::Inherited);
    assert!(any_step(&security.process, &|step| matches!(
        step,
        CompiledStep::SelectOpponentPermanent {
            optional: false,
            ..
        }
    )));

    let nail_bone = compiled("ST6-16");
    let main = triggered(&nail_bone, CompiledTiming::MainFromHand)
        .pop()
        .expect("ST6-16 Main clause");
    let free_plays = main
        .process
        .iter()
        .filter(|step| {
            matches!(
                step,
                CompiledStep::PlayFromTrashFree {
                    suppress_on_play: true,
                    ..
                }
            )
        })
        .count();
    assert_eq!(
        free_plays, 2,
        "ST6-16 Main must free-play Lv3 and Lv4 candidates with On Play suppression"
    );
    let security = triggered(&nail_bone, CompiledTiming::OnSecurity)
        .pop()
        .expect("ST6-16 Security clause");
    assert!(security.optional);
    assert!(any_step(&security.process, &|step| matches!(
        step,
        CompiledStep::PlayFromTrashFree {
            suppress_on_play: true,
            ..
        }
    )));
}

/// ST-6 Venomous Violet starter-deck fixture: exact official 54-card composition
/// (4 Digi-Eggs + 50 main). Mirrors `st4_deck_library_recipe_has_canonical_counts`.
#[test]
fn st6_deck_library_recipe_has_canonical_counts() {
    let library: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../data/deck_library.json"))
            .expect("deck_library.json parses");
    let entry = &library["archetypes"]["ST-6 Venomous Violet"]["decklists"][0];
    assert_eq!(entry["deck_id"], "starter_st6_venomous_violet");

    let cards: Vec<String> = serde_json::from_str(
        entry["decklist"]
            .as_str()
            .expect("decklist is encoded JSON"),
    )
    .expect("decklist parses");
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for card in &cards {
        *counts.entry(card.clone()).or_default() += 1;
    }

    assert_eq!(cards.len(), 54);
    assert_eq!(counts.get("ST6-01"), Some(&4));
    let main_count: usize = cards.iter().filter(|id| id.as_str() != "ST6-01").count();
    assert_eq!(main_count, 50);

    let expected = [
        ("ST6-01", 4),
        ("ST6-02", 4),
        ("ST6-03", 4),
        ("ST6-04", 4),
        ("ST6-05", 2),
        ("ST6-06", 4),
        ("ST6-07", 4),
        ("ST6-08", 4),
        ("ST6-09", 4),
        ("ST6-10", 2),
        ("ST6-11", 4),
        ("ST6-12", 2),
        ("ST6-13", 2),
        ("ST6-14", 4),
        ("ST6-15", 4),
        ("ST6-16", 2),
    ];
    for (card_id, count) in expected {
        assert_eq!(counts.get(card_id), Some(&count), "{card_id} count");
    }
}
