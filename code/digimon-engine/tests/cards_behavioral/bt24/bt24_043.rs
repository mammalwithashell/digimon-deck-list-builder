//! BT24-043 Tapirmon — Digimon, Lv.3, Green.
//!
//! # Card text
//!
//! [On Play] Reveal the top 3 cards of your deck. Add 1 Digimon card with the
//! [Shaman] trait or with [Beast], [Animal] or [Sovereign] in any of its traits
//! (other than [Sea Animal]) and 1 card with the [TS] trait among them to the
//! hand. Return the rest to the bottom of the deck.
//!
//! Inherited Effect: [When Attacking] [Once Per Turn] Suspend 1 of your
//! opponent's Digimon.
//!
//! # Patterns
//! - Searching rookie: reveal-3, two buckets, no duplicate selected card.
//! - Trait alternatives with an exclusion: Shaman OR Beast/Animal/Sovereign,
//!   but not Sea Animal.
//! - Inherited mandatory When Attacking OPT with an opponent-field suspend prompt.

use std::fs;

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT24-043";

fn yaml() -> String {
    fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/cards/bt24/BT24-043.yaml"
    ))
    .expect("BT24-043 production YAML must exist")
}

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    let yaml = yaml();
    let spec: CardSpec = serde_yml::from_str(&yaml).expect("BT24-043 YAML parses");
    compile(&spec).expect("BT24-043 YAML compiles")
}

fn builder_with_tapirmon() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    let yaml = yaml();
    DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("BT24-043 YAML loads")
}

fn trait_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(4000);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn trait_tamer(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> Option<u16> {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
}

fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = revealed_action_for_id(runner, id)
        .unwrap_or_else(|| panic!("{label}: {id} must be in the reveal"));
    assert!(
        view.valid_action_ids.contains(&action),
        "{label}: {id} action must be legal; valid={:?}",
        view.valid_action_ids
    );
    runner.execute_action(0, action).expect(label);
}

#[test]
fn bt24_043_yaml_has_printed_metadata_alt_path_and_clauses() {
    let card = compiled();
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Tapirmon");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(3));
    assert_eq!(card.color, vec![CompiledColor::Green]);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(1000));
    for trait_name in ["Holy Beast", "Iliad", "TS"] {
        assert!(
            card.traits.iter().any(|t| t == trait_name),
            "missing printed trait {trait_name}; traits={:?}",
            card.traits
        );
    }

    assert!(
        card.alt_paths.iter().any(|path| {
            let Some(from) = path.from.as_ref() else {
                return false;
            };
            path.cost == Some(CompiledCost::Literal(0))
                && from.level_eq == Some(2)
                && from.color_is == Some(CompiledColor::Green)
        }),
        "Tapirmon must digivolve from green level 2 for cost 0"
    );
    assert!(
        card.alt_paths.iter().any(|path| {
            let Some(from) = path.from.as_ref() else {
                return false;
            };
            path.cost == Some(CompiledCost::Literal(0))
                && from.all_of.iter().any(|p| p.level_eq == Some(2))
                && from
                    .all_of
                    .iter()
                    .any(|p| p.trait_has.as_deref() == Some("TS"))
        }),
        "Tapirmon must also digivolve from level 2 with [TS] trait for cost 0"
    );

    let on_play = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnPlay] => Some(t),
            _ => None,
        })
        .expect("On Play reveal clause exists");
    assert!(
        !on_play.optional,
        "printed On Play has no 'may'; the outer trigger is mandatory"
    );
    assert!(!on_play.once_per_turn);

    let inherited = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when == vec![CompiledTiming::WhenAttacking] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("Inherited When Attacking clause exists");
    assert!(inherited.once_per_turn);
    assert!(
        !inherited.optional,
        "inherited suspend is mandatory when a legal target exists"
    );
}

#[test]
fn bt24_043_on_play_adds_trait_bucket_and_ts_without_duplicate_selection() {
    let mut runner = builder_with_tapirmon()
        .add_card(trait_digimon("DUAL-SHAMAN-TS", &["Shaman", "TS"]))
        .add_card(trait_tamer("TS-ONLY", &["TS"]))
        .add_card(trait_digimon("REMAINDER", &["Machine"]))
        .add_card(make_test_card("BOTTOM-SENTINEL", "Bottom Sentinel"))
        .deck(
            0,
            &["BOTTOM-SENTINEL", "REMAINDER", "TS-ONLY", "DUAL-SHAMAN-TS"],
        )
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Tapirmon");

    assert!(
        !runner.pending_is_optional(),
        "first matching reveal bucket is mandatory"
    );
    assert!(
        !runner
            .pending_selection_view()
            .unwrap()
            .valid_action_ids
            .contains(&PASS),
        "PASS must not be legal for mandatory bucket selection"
    );
    pick_revealed_by_id(&mut runner, "DUAL-SHAMAN-TS", "pick Shaman/TS dual card");

    let second = runner
        .pending_selection_view()
        .expect("TS bucket must be offered");
    if let Some(dual_action) = revealed_action_for_id(&runner, "DUAL-SHAMAN-TS") {
        assert!(
            !second.valid_action_ids.contains(&dual_action),
            "no_duplicate_cards prevents the same card from satisfying both buckets"
        );
    }
    pick_revealed_by_id(&mut runner, "TS-ONLY", "pick TS-only card");
    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"DUAL-SHAMAN-TS".to_string()));
    assert!(hand_ids.contains(&"TS-ONLY".to_string()));
    assert!(!hand_ids.contains(&"REMAINDER".to_string()));

    let deck_ids = zone_ids(&runner.game.players[0].deck, &runner.game.card_data);
    assert_eq!(
        deck_ids.first().map(String::as_str),
        Some("REMAINDER"),
        "unchosen reveal card must be returned to deck bottom; deck={deck_ids:?}"
    );
}

#[test]
fn bt24_043_on_play_excludes_sea_animal_from_beast_animal_sovereign_bucket() {
    let mut runner = builder_with_tapirmon()
        .add_card(trait_digimon("SEA-BEAST", &["Beast", "Sea Animal"]))
        .add_card(trait_digimon("BEAST-OK", &["Beast"]))
        .add_card(trait_tamer("TS-ONLY", &["TS"]))
        .deck(0, &["TS-ONLY", "BEAST-OK", "SEA-BEAST"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Tapirmon");
    let first = runner
        .pending_selection_view()
        .expect("trait bucket must be offered");
    let sea_action = revealed_action_for_id(&runner, "SEA-BEAST").expect("SEA-BEAST revealed");
    let beast_action = revealed_action_for_id(&runner, "BEAST-OK").expect("BEAST-OK revealed");
    assert!(
        !first.valid_action_ids.contains(&sea_action),
        "Sea Animal must be excluded even when it has Beast"
    );
    assert!(
        first.valid_action_ids.contains(&beast_action),
        "non-Sea-Animal Beast Digimon must be selectable"
    );
}

#[test]
fn bt24_043_on_play_with_no_matching_search_targets_adds_nothing_and_bottoms_reveal() {
    let mut runner = builder_with_tapirmon()
        .add_card(trait_digimon("SEA-BEAST", &["Beast", "Sea Animal"]))
        .add_card(trait_digimon("NO-TRAIT", &["Machine"]))
        .add_card(trait_tamer("TAMER-NO-TS", &["Iliad"]))
        .deck(0, &["TAMER-NO-TS", "NO-TRAIT", "SEA-BEAST"])
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Tapirmon");
    runner
        .auto_resolve()
        .expect("empty buckets should flow to bottom placement");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.is_empty(),
        "no matching search targets should be added to hand; hand={hand_ids:?}"
    );
    assert_eq!(
        runner.game.players[0].deck.len(),
        3,
        "all revealed nonmatches return to deck"
    );
}

#[test]
fn bt24_043_inherited_when_attacking_suspends_selected_opponent_digimon() {
    let mut runner = builder_with_tapirmon()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(trait_digimon("OPPONENT", &["Machine"]))
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opponent = runner.place_on_field(1, "OPPONENT", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("inherited suspend must install opponent selection");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!runner.pending_is_optional());
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select opponent Digimon to suspend");

    assert!(
        runner.game.players[opponent.player as usize].battle_area[opponent.index as usize]
            .is_suspended,
        "selected opponent Digimon must be suspended"
    );
}

#[test]
fn bt24_043_inherited_when_attacking_no_target_installs_no_prompt() {
    let mut runner = builder_with_tapirmon()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "mandatory suspend has no legal choice when opponent controls no Digimon"
    );
}

#[test]
fn bt24_043_inherited_when_attacking_is_once_per_turn() {
    let mut runner = builder_with_tapirmon()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(trait_digimon("OPPONENT", &["Machine"]))
        .memory(0)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opponent = runner.place_on_field(1, "OPPONENT", None);

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    let first = runner
        .pending_selection_view()
        .expect("first attack should install suspend prompt");
    runner
        .execute_action(0, first.valid_action_ids[0])
        .expect("resolve first suspend");

    runner.game.players[opponent.player as usize].battle_area[opponent.index as usize]
        .is_suspended = false;

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "OPT must block the second inherited When Attacking trigger in the same turn"
    );
    assert!(
        !runner.game.players[opponent.player as usize].battle_area[opponent.index as usize]
            .is_suspended,
        "second same-turn trigger must not suspend again"
    );
}
