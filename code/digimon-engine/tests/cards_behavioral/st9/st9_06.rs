//! ST9-06 Imperialdramon Dragon Mode — Digimon, Lv.6, Blue+Green, DP 12000, Cost 13.
//! Traits: Ancient Dragon.
//!
//! # Card text (cards.json — verbatim)
//!
//! [When Digivolving] You may play 1 level 4 or lower blue Digimon card and
//! 1 level 4 or lower green Digimon card from this Digimon's digivolution
//! cards without paying their memory costs.
//!
//! Inherited / Security: (none)
//!
//! Standard digivolve: Lv.5 Blue / Cost 4.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST9/Blue/ST9_06.cs
//!
//! # Patterns this test covers
//!
//! - Single standard digivolve alt-path: Lv.5 Blue / Cost 4.
//! - [When Digivolving] optional source selections from this Digimon's own
//!   digivolution stack (blue Lv<=4 + green Lv<=4), played without cost.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPath, CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope,
    CompiledStep, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

/// Production YAML for ST9-06, loaded at compile time.
const YAML: &str = include_str!("../../../cards/st9/ST9-06.yaml");

// ─── Compile helpers ──────────────────────────────────────────────────────────

fn compiled_st9_06() -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(YAML).expect("ST9-06.yaml parses");
    let registry =
        digimon_dsl::CardRegistry::from_specs("test", &[spec]).expect("ST9-06.yaml compiles");
    registry
        .lookup("ST9-06")
        .expect("ST9-06 in registry")
        .clone()
}

fn source_card(id: &str, color: CardColor, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.level = Some(level);
    card
}

fn st9_06_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("ST9-06 YAML loads")
        .add_card(source_card("BLUE-LV4", CardColor::Blue, 4))
        .add_card(source_card("GREEN-LV4", CardColor::Green, 4))
        .add_card(source_card("RED-LV4", CardColor::Red, 4))
        .add_card(source_card("BLUE-LV5", CardColor::Blue, 5))
        .add_card(source_card("GREEN-LV5", CardColor::Green, 5))
        .add_card(source_card("OTHER-BLUE", CardColor::Blue, 4))
        .add_card(source_card("OTHER-TOP", CardColor::Green, 5))
        .memory(10)
        .start()
}

fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
}

fn top_card_ids(runner: &DebugRunner, player: PlayerId) -> Vec<String> {
    runner
        .game
        .player(player)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn source_card_ids(runner: &DebugRunner, host: PermanentHandle) -> Vec<String> {
    runner.game.player(host.player).battle_area[host.index as usize]
        .card_sources
        .iter()
        .map(|source| source.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn assert_mask_matches_pending(runner: &DebugRunner, player: PlayerId) {
    let pending = runner
        .pending_selection_view()
        .expect("pending selection view exists");
    let mask = build_action_mask(&runner.game, player);
    for action in &pending.valid_action_ids {
        assert_eq!(
            mask[*action as usize], 1.0,
            "pending action {action} must be exposed through the action mask"
        );
    }
    if pending.is_optional {
        assert_eq!(
            mask[PASS as usize], 1.0,
            "optional pending selection must expose PASS through the action mask"
        );
    }
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

/// The YAML must parse and compile without errors.
#[test]
fn st9_06_compiles() {
    let _ = compiled_st9_06();
}

/// Card metadata must match the printed card (Lv.6, DP 12000, Cost 13).
#[test]
fn st9_06_card_metadata_matches_print() {
    let compiled = compiled_st9_06();
    assert_eq!(
        compiled.level,
        Some(6),
        "Imperialdramon Dragon Mode is Lv.6"
    );
    assert_eq!(compiled.dp, Some(12000), "DP is 12000");
    assert_eq!(compiled.cost, Some(13), "play cost is 13");
}

/// Standard digivolve alt-path: Lv.5 Blue / Cost 4 (from evo_costs in cards.json).
#[test]
fn st9_06_has_standard_digivolve_lv5_blue_cost4() {
    let compiled = compiled_st9_06();
    let path = compiled
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && matches!(p.cost, Some(CompiledCost::Literal(4)))
                && p.from
                    .as_ref()
                    .and_then(|f| f.level_eq)
                    .map_or(false, |l| l == 5)
        })
        .expect("ST9-06 must have a Lv.5 digivolve path with cost 4");
    assert!(!path.ignore_requirements);
}

/// Exactly one alt-path (the standard digivolve; no DNA / DigiXros paths).
#[test]
fn st9_06_has_exactly_one_alt_path() {
    let compiled = compiled_st9_06();
    assert_eq!(
        compiled.alt_paths.len(),
        1,
        "ST9-06 has exactly 1 alt-path (standard digivolve Lv.5 Blue / Cost 4)"
    );
}

/// The printed [When Digivolving] source-play clause is now represented as one
/// optional face-up triggered clause.
#[test]
fn st9_06_has_optional_when_digivolving_clause() {
    let compiled = compiled_st9_06();
    let clauses: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if matches!(t.scope, CompiledScope::FaceUp)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        clauses.len(),
        1,
        "ST9-06 must have exactly one face-up [When Digivolving] clause"
    );
    assert!(
        clauses[0].optional,
        "printed 'You may' makes the clause optional"
    );
}

#[test]
fn st9_06_when_digivolving_clause_selects_blue_then_green_sources() {
    let compiled = compiled_st9_06();
    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .expect("when digivolving clause exists");

    assert_eq!(clause.process.len(), 2, "blue and green source picks");
    for (idx, expected_binding) in [(0, "blue_source"), (1, "green_source")] {
        let CompiledStep::SelectOwnSources {
            target,
            min,
            max,
            bind_as,
            then,
            ..
        } = &clause.process[idx]
        else {
            panic!("step {idx} must be select_own_sources");
        };
        assert_eq!((*min, *max), (0, 1), "each source pick is optional up to 1");
        assert!(
            target.is_some(),
            "source picks must be scoped to this Digimon"
        );
        assert_eq!(bind_as.as_deref(), Some(expected_binding));
        assert!(
            matches!(
                then.as_slice(),
                [CompiledStep::PlaySelectedSourcesFree { .. }]
            ),
            "each source pick must immediately play the selected source free"
        );
    }
}

#[test]
fn st9_06_when_digivolving_plays_blue_and_green_from_stack_free() {
    let mut runner = st9_06_runner();
    let carrier = runner.place_stack(
        0,
        &["BLUE-LV4", "GREEN-LV4", "RED-LV4", "BLUE-LV5", "ST9-06"],
    );
    let other = runner.place_stack(0, &["OTHER-BLUE", "OTHER-TOP"]);

    fire_when_digivolving(&mut runner, carrier);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 0,
            max: 1,
            picked: 0,
        }),
        "blue source selection must be player-visible"
    );
    assert!(
        runner.pending_is_optional(),
        "blue source selection must be declinable"
    );
    assert_mask_matches_pending(&runner, 0);

    let blue = encode_source_select(carrier.index as u16, 0).expect("blue source action");
    let green = encode_source_select(carrier.index as u16, 1).expect("green source action");
    let red = encode_source_select(carrier.index as u16, 2).expect("red source action");
    let blue_lv5 = encode_source_select(carrier.index as u16, 3).expect("blue Lv5 source action");
    let other_blue = encode_source_select(other.index as u16, 0).expect("other blue action");
    let view = runner
        .pending_selection_view()
        .expect("blue source pending");
    assert!(view.valid_action_ids.contains(&blue));
    assert!(
        !view.valid_action_ids.contains(&green),
        "green source is not a blue pick"
    );
    assert!(
        !view.valid_action_ids.contains(&red),
        "red source is filtered out"
    );
    assert!(
        !view.valid_action_ids.contains(&blue_lv5),
        "Lv5 blue source is filtered out"
    );
    assert!(
        !view.valid_action_ids.contains(&other_blue),
        "selection must be restricted to this Digimon's sources"
    );

    runner.execute_action(0, blue).expect("choose blue source");

    assert!(
        top_card_ids(&runner, 0).contains(&"BLUE-LV4".to_string()),
        "blue selected source should be played as its own permanent"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 0,
            max: 1,
            picked: 0,
        }),
        "green source selection must follow the blue source play"
    );
    assert_mask_matches_pending(&runner, 0);
    let green_view = runner
        .pending_selection_view()
        .expect("green source pending");
    let shifted_green =
        encode_source_select(carrier.index as u16, 0).expect("green source shifts after blue play");
    assert!(green_view.valid_action_ids.contains(&shifted_green));
    assert_eq!(
        green_view
            .valid_action_ids
            .iter()
            .filter(|&&action| action != PASS)
            .count(),
        1,
        "only the Lv4 green source should be legal after the blue source is played"
    );

    runner
        .execute_action(0, shifted_green)
        .expect("choose green source");

    assert!(runner.pending_selection().is_none());
    let tops = top_card_ids(&runner, 0);
    assert!(tops.contains(&"BLUE-LV4".to_string()));
    assert!(tops.contains(&"GREEN-LV4".to_string()));
    assert_eq!(
        source_card_ids(&runner, carrier),
        vec!["RED-LV4", "BLUE-LV5", "ST9-06"],
        "played sources are removed from the original stack; ineligible sources remain"
    );
}

#[test]
fn st9_06_when_digivolving_no_blue_candidate_still_offers_green_candidate() {
    let mut runner = st9_06_runner();
    let carrier = runner.place_stack(0, &["GREEN-LV4", "RED-LV4", "ST9-06"]);

    fire_when_digivolving(&mut runner, carrier);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 0,
            max: 1,
            picked: 0,
        }),
        "empty blue bucket should skip into the green source selection"
    );
    let green = encode_source_select(carrier.index as u16, 0).expect("green source action");
    let red = encode_source_select(carrier.index as u16, 1).expect("red source action");
    let view = runner
        .pending_selection_view()
        .expect("green source pending");
    assert!(view.valid_action_ids.contains(&green));
    assert!(!view.valid_action_ids.contains(&red));

    runner
        .execute_action(0, green)
        .expect("choose green source");

    assert!(runner.pending_selection().is_none());
    assert!(top_card_ids(&runner, 0).contains(&"GREEN-LV4".to_string()));
    assert_eq!(source_card_ids(&runner, carrier), vec!["RED-LV4", "ST9-06"]);
}

#[test]
fn st9_06_when_digivolving_no_green_candidate_skips_green_pick() {
    let mut runner = st9_06_runner();
    let carrier = runner.place_stack(0, &["BLUE-LV4", "RED-LV4", "ST9-06"]);

    fire_when_digivolving(&mut runner, carrier);

    let blue = encode_source_select(carrier.index as u16, 0).expect("blue source action");
    runner.execute_action(0, blue).expect("choose blue source");

    assert!(
        runner.pending_selection().is_none(),
        "missing green candidates should not leave a dead pending selection"
    );
    assert!(top_card_ids(&runner, 0).contains(&"BLUE-LV4".to_string()));
    assert_eq!(source_card_ids(&runner, carrier), vec!["RED-LV4", "ST9-06"]);
}

#[test]
fn st9_06_when_digivolving_declining_does_nothing() {
    let mut runner = st9_06_runner();
    let carrier = runner.place_stack(0, &["BLUE-LV4", "GREEN-LV4", "ST9-06"]);

    fire_when_digivolving(&mut runner, carrier);

    assert!(
        runner.pending_is_optional(),
        "printed 'You may' must expose PASS"
    );
    assert_mask_matches_pending(&runner, 0);
    runner
        .execute_action(0, PASS)
        .expect("decline blue source selection");

    assert!(
        runner.pending_is_optional(),
        "green source selection is also optional"
    );
    runner
        .execute_action(0, PASS)
        .expect("decline green source selection");

    assert!(runner.pending_selection().is_none());
    assert_eq!(
        top_card_ids(&runner, 0),
        vec!["ST9-06"],
        "declining both selections should play no sources"
    );
    assert_eq!(
        source_card_ids(&runner, carrier),
        vec!["BLUE-LV4", "GREEN-LV4", "ST9-06"],
        "declining leaves the stack unchanged"
    );
}
