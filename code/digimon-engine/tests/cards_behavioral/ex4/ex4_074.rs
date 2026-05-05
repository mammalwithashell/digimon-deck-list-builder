//! EX4-074 ShineGreymon: Ruin Mode - Digimon, Lv.7, Purple/Yellow.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [When Digivolving] [On Deletion] Until the end of your opponent's next turn,
//! all of your opponent's Digimon get -5000DP.
//! [End of Attack] Delete this Digimon and 1 of your opponent's Digimon, and
//! <Recovery +1 (Deck)>. Then, if you have a Tamer in play, hatch 1 Digi-Egg
//! card to an empty space in your breeding area.
//! ```
//!
//! # Coverage
//!
//! - Metadata and printed purple Lv.6 / [ShineGreymon] alternate digivolution paths.
//! - Gap-routed [When Digivolving] [On Deletion] opponent-wide -5000 DP through
//!   the end of the opponent's next turn.
//! - Gap-routed End of Attack self-delete + opponent delete + Recovery + hatch chain.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::TriggerSource;

#[test]
fn ex4_074_metadata_alt_paths_and_supported_clause_match_printed_text() {
    let runner = shine_runner().start();
    let compiled = runner.compiled_card("EX4-074").expect("EX4-074 compiles");

    assert_eq!(compiled.name, "ShineGreymon: Ruin Mode");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(7));
    assert_eq!(compiled.cost, Some(14));
    assert_eq!(compiled.dp, Some(15000));
    assert_eq!(
        compiled.color,
        vec![CompiledColor::Purple, CompiledColor::Yellow]
    );
    assert!(compiled.traits.iter().any(|name| name == "Light Dragon"));
    assert_eq!(compiled.attribute.as_deref(), Some("Vaccine"));

    let digivolve_paths: Vec<_> = compiled
        .alt_paths
        .iter()
        .filter(|path| path.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert!(
        digivolve_paths.iter().any(|path| {
            path.cost == Some(CompiledCost::Literal(5))
                && path.from.as_ref().is_some_and(|from| {
                    from.level_eq == Some(6) && from.color_is == Some(CompiledColor::Purple)
                })
        }),
        "printed purple Lv.6 digivolution path must cost 5; paths={digivolve_paths:?}"
    );
    assert!(
        digivolve_paths.iter().any(|path| {
            path.cost == Some(CompiledCost::Literal(4))
                && path
                    .from
                    .as_ref()
                    .is_some_and(|from| from.name_contains.as_deref() == Some("ShineGreymon"))
        }),
        "printed [ShineGreymon] alternate digivolution path must cost 4; paths={digivolve_paths:?}"
    );

    assert!(
        !triggered_clauses(compiled).iter().any(|clause| {
            clause.when.contains(&CompiledTiming::WhenDigivolving)
                || clause.when.contains(&CompiledTiming::OnDeletion)
        }),
        "When Digivolving / On Deletion DP reduction is intentionally omitted until next-turn modifier expiry exists"
    );
    assert!(
        !triggered_clauses(compiled)
            .iter()
            .any(|clause| clause.when.contains(&CompiledTiming::EndOfAttack)),
        "End of Attack is intentionally omitted until the full chain is fidelity-proven"
    );
}

#[test]
#[ignore = "BLOCKED: EX4-074 needs modifier expiry for opponent's next turn, not current end_of_opponents_turn"]
fn ex4_074_when_digivolving_reduces_all_opponent_digimon_but_not_tamers() {
    let mut runner = shine_runner()
        .add_card(make_digimon("BASE", 3000))
        .add_card(make_digimon("OPP-1", 9000))
        .add_card(make_digimon("OPP-2", 11000))
        .add_card(make_tamer("OPP-TAMER"))
        .start();
    let ruin = runner.place_stack(0, &["BASE", "EX4-074"]);
    let opp_1 = runner.place_on_field(1, "OPP-1", Some(0));
    let opp_2 = runner.place_on_field(1, "OPP-2", Some(0));
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));

    fire_when_digivolving(&mut runner, ruin);

    assert!(
        runner.pending_selection_view().is_none(),
        "opponent-wide DP reduction has no player-visible choice"
    );
    assert_eq!(runner.dp_of(opp_1), Some(4000));
    assert_eq!(runner.dp_of(opp_2), Some(6000));
    assert_eq!(
        runner.dp_of(opp_tamer),
        None,
        "Tamers are not Digimon and must not receive a DP modifier"
    );
}

#[test]
#[ignore = "BLOCKED: EX4-074 needs modifier expiry for opponent's next turn, not current end_of_opponents_turn"]
fn ex4_074_on_deletion_reduces_all_opponent_digimon() {
    let mut runner = shine_runner()
        .add_card(make_digimon("OPP-1", 9000))
        .add_card(make_digimon("OPP-2", 10000))
        .start();
    let ruin = runner.place_on_field(0, "EX4-074", Some(0));
    let opp_1 = runner.place_on_field(1, "OPP-1", Some(0));
    let opp_2 = runner.place_on_field(1, "OPP-2", Some(0));

    runner
        .game
        .delete_permanent_with_cause(ruin, ReplacementCause::OpponentEffect);
    runner
        .auto_resolve()
        .expect("finish On Deletion DP reduction");

    assert_eq!(runner.dp_of(opp_1), Some(4000));
    assert_eq!(runner.dp_of(opp_2), Some(5000));
}

#[test]
#[ignore = "BLOCKED: EX4-074 needs modifier expiry for opponent's next turn, not current end_of_opponents_turn"]
fn ex4_074_on_deletion_during_opponents_turn_expires_at_end_of_their_next_turn() {
    let mut runner = shine_runner()
        .add_card(make_digimon("OPP", 9000))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .start();
    let ruin = runner.place_on_field(0, "EX4-074", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.end_turn();
    runner
        .game
        .delete_permanent_with_cause(ruin, ReplacementCause::OpponentEffect);
    runner
        .auto_resolve()
        .expect("finish On Deletion DP reduction");

    assert_eq!(runner.dp_of(opp), Some(4000));
    runner.end_turn();
    assert_eq!(
        runner.dp_of(opp),
        Some(4000),
        "modifier persists through the opponent's current turn ending"
    );
    runner.end_turn();
    assert_eq!(
        runner.dp_of(opp),
        Some(4000),
        "modifier persists through ShineGreymon's controller's next turn"
    );
    runner.end_turn();
    assert_eq!(
        runner.dp_of(opp),
        Some(9000),
        "modifier expires when the opponent's next turn ends"
    );
}

#[test]
#[ignore = "BLOCKED: PUPPETS-G031 — End of Attack chain needs mandatory self-delete + opponent target delete + Recovery + conditional hatch continuation"]
fn ex4_074_end_of_attack_self_deletes_opponent_delete_recovers_and_hatches_with_tamer() {
    let mut runner = shine_runner()
        .add_card(make_digimon("OPP", 9000))
        .add_card(make_tamer("TAMER"))
        .add_card(make_test_card("SECURITY", "Security"))
        .hand(0, &["TAMER"])
        .deck(0, &["SECURITY"])
        .security(1, &["SECURITY"])
        .start();
    let ruin = runner.place_on_field(0, "EX4-074", Some(0));
    runner.place_on_field(0, "TAMER", Some(0));
    runner.place_on_field(1, "OPP", Some(0));

    runner.attack_player(ruin, 1, false);
    runner.auto_resolve().expect("resolve End of Attack chain");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|perm| { perm.top_card().card_id(&runner.game.card_data) != "EX4-074" }),
        "Ruin Mode deletes itself"
    );
    assert_eq!(runner.security_count(0), 1, "Recovery +1 (Deck)");
    assert!(
        runner.game.players[0].breeding_area.is_some(),
        "Tamer condition hatches an egg into an empty breeding area"
    );
}

fn shine_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("EX4-074")
        .expect("EX4-074 YAML loads")
}

fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(handle),
    );
    runner.game.drain_effect_queue();
    runner
        .auto_resolve()
        .expect("finish When Digivolving effect");
}

fn triggered_clauses(
    compiled: &digimon_dsl::compiled::CompiledCard,
) -> Vec<&digimon_dsl::compiled::CompiledTriggeredClause> {
    compiled
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .collect()
}

fn make_digimon(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

fn make_tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.dp = None;
    card
}
