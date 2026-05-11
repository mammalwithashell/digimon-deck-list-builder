//! BT24-046 Garurumon — Digimon, Lv.4, Green, DP 4000, Cost 4.
//!
//! # Card text
//!
//! Effect:
//!   <Jamming>
//!   [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon.
//!
//! Inherited Effect:
//!   [When Attacking] [Once Per Turn] Suspend 1 of your opponent's Digimon.
//!
//! # Patterns
//! - H2 printed Jamming via `kind: grant_keyword`
//! - Mandatory `select_opponent_permanent` Digimon-only target selection
//! - Inherited [When Attacking] [Once Per Turn] target selection

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::PermanentHandle;

fn yaml() -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join("bt24")
        .join("BT24-046.yaml");
    std::fs::read_to_string(&path).expect("BT24-046 production YAML exists")
}

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    let yaml = yaml();
    let spec: CardSpec = serde_yml::from_str(&yaml).expect("BT24-046 YAML parses");
    compile(&spec).expect("BT24-046 YAML compiles")
}

fn garurumon_builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(&yaml())
        .expect("BT24-046 YAML loads")
        .add_card(digimon("CARRIER"))
        .add_card(digimon("OWN-DIGIMON"))
        .add_card(digimon("OPP-DIGIMON-A"))
        .add_card(digimon("OPP-DIGIMON-B"))
        .add_card(tamer("OPP-TAMER"))
}

fn digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card
}

fn tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}

fn target_action(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

fn fire_when_digivolving(runner: &mut DebugRunner, source: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn fire_when_attacking(runner: &mut DebugRunner, source: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

#[test]
fn bt24_046_has_printed_metadata_jamming_and_triggered_clauses() {
    let card = compiled();

    assert_eq!(card.card, "BT24-046");
    assert_eq!(card.name, "Garurumon");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.color, vec![CompiledColor::Green, CompiledColor::Blue]);
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(4000));
    for trait_name in ["Beast", "Iliad", "TS"] {
        assert!(
            card.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    assert!(card.alt_paths.iter().any(|path| {
        path.kind == CompiledAltPathKind::Digivolve
            && path.cost == Some(CompiledCost::Literal(3))
            && path.from.as_ref().is_some_and(|from| {
                (from.level_eq == Some(3) || from.all_of.iter().any(|p| p.level_eq == Some(3)))
                    && (from.color_is == Some(CompiledColor::Green)
                        || from
                            .all_of
                            .iter()
                            .any(|p| p.color_is == Some(CompiledColor::Green)))
            })
    }));
    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(2))
                && path.from.as_ref().is_some_and(|from| {
                    let has_level = from.all_of.iter().any(|p| p.level_eq == Some(3));
                    let has_gabumon_or_ts = from.all_of.iter().any(|p| {
                        p.any_of.iter().any(|branch| {
                            branch.name_contains.as_deref() == Some("Gabumon")
                                || branch.trait_has.as_deref() == Some("TS")
                        })
                    });
                    has_level && has_gabumon_or_ts
                })
        }),
        "BT24-046 must expose the printed cost-2 Lv.3 Gabumon-name or TS-trait digivolution path"
    );

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope: CompiledScope::FaceUp,
            ..
        }) if keyword == "Jamming"
    )));

    let shared = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::FaceUp
                    && triggered.when.contains(&CompiledTiming::OnPlay)
                    && triggered.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("shared On Play / When Digivolving clause");
    assert!(!shared.optional, "printed suspend effect is mandatory");
    assert!(
        !shared.once_per_turn,
        "face-up suspend effect has no [Once Per Turn]"
    );

    let inherited = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.scope == CompiledScope::Inherited
                    && triggered.when == vec![CompiledTiming::WhenAttacking] =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("inherited When Attacking clause");
    assert!(inherited.once_per_turn);
    assert!(!inherited.optional);
}

#[test]
fn bt24_046_on_play_prompts_only_opponent_digimon_and_suspends_selected_target() {
    let mut runner = garurumon_builder()
        .hand(0, &["BT24-046"])
        .memory(10)
        .start();
    runner.place_on_field(0, "OWN-DIGIMON", Some(0));
    let opp_digimon = runner.place_on_field(1, "OPP-DIGIMON-A", Some(0));
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));

    runner.play(0, 0).expect("play Garurumon");

    let view = runner
        .pending_selection_view()
        .expect("On Play must prompt for an opponent Digimon");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!view.is_optional);
    assert!(view.valid_action_ids.contains(&target_action(opp_digimon)));
    assert!(!view.valid_action_ids.contains(&target_action(opp_tamer)));

    runner
        .execute_action(0, target_action(opp_digimon))
        .expect("select opponent Digimon");
    assert!(
        runner.game.players[1].battle_area[opp_digimon.index as usize].is_suspended,
        "selected opponent Digimon must be suspended"
    );
}

#[test]
fn bt24_046_when_digivolving_uses_same_opponent_digimon_suspend_prompt() {
    let mut runner = garurumon_builder().memory(10).start();
    let garurumon = runner.place_on_field(0, "BT24-046", Some(0));
    let opp_digimon = runner.place_on_field(1, "OPP-DIGIMON-A", Some(0));
    runner.place_on_field(1, "OPP-TAMER", Some(0));

    fire_when_digivolving(&mut runner, garurumon);

    let view = runner
        .pending_selection_view()
        .expect("When Digivolving must prompt for an opponent Digimon");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(view.valid_action_ids, vec![target_action(opp_digimon)]);
    runner
        .execute_action(0, target_action(opp_digimon))
        .expect("select opponent Digimon");
    assert!(
        runner.game.players[1].battle_area[opp_digimon.index as usize].is_suspended,
        "selected opponent Digimon must be suspended"
    );
}

#[test]
fn bt24_046_inherited_when_attacking_is_opt_and_suspends_selected_target() {
    let mut runner = garurumon_builder().memory(10).start();
    let carrier = runner.place_stack(0, &["BT24-046", "CARRIER"]);
    let first = runner.place_on_field(1, "OPP-DIGIMON-A", Some(0));
    let second = runner.place_on_field(1, "OPP-DIGIMON-B", Some(0));

    fire_when_attacking(&mut runner, carrier);

    let view = runner
        .pending_selection_view()
        .expect("inherited When Attacking must prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!view.is_optional);
    assert_eq!(view.valid_action_ids.len(), 2);
    runner
        .execute_action(0, target_action(first))
        .expect("select first opponent Digimon");
    assert!(runner.game.players[1].battle_area[first.index as usize].is_suspended);

    fire_when_attacking(&mut runner, carrier);
    assert!(
        runner.pending_selection().is_none(),
        "OPT must block the second inherited activation in the same turn"
    );
    assert!(
        !runner.game.players[1].battle_area[second.index as usize].is_suspended,
        "second target must remain unsuspended after OPT lockout"
    );
}

#[test]
fn bt24_046_no_target_behavior_does_not_install_prompt() {
    let mut runner = garurumon_builder()
        .hand(0, &["BT24-046"])
        .memory(10)
        .start();
    runner.place_on_field(0, "OWN-DIGIMON", Some(0));
    runner.place_on_field(1, "OPP-TAMER", Some(0));

    runner.play(0, 0).expect("play Garurumon");

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon means the mandatory suspend effect quietly has no target"
    );
}
