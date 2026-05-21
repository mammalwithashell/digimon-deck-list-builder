//! EX7-024 Shoemon.
//!
//! Printed text from `data/cards.json`:
//! - [Your Turn] When this Digimon would digivolve into a Digimon card with the
//!   [Puppet] trait, reduce the digivolution cost by 1.
//! - Inherited: [Your Turn] All of your opponent's Security Digimon get -3000 DP.
//!
//! Current verdict: PARTIAL. The printed metadata, yellow Lv2 digivolution
//! path, and the inherited opponent-security -3000 DP aura are implemented in
//! production DSL (PUPPETS-G008 / G-OPPONENT-SECURITY-DP-AURA, resolved Track I
//! 2026-05-17 — lowers via `applies_to_opponent_security_dp: true`). The
//! source-scoped digivolve-into-trait cost reduction remains blocked by a
//! reusable DSL/engine vocabulary gap:
//! - no `when_this_digivolves_into` + target-trait cost-reduction hook.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause,
};
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

#[test]
fn ex7_024_compiles_with_printed_stats_and_lv2_yellow_path() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 found in embedded DSL pack")
        .start();

    let compiled = runner
        .compiled_card("EX7-024")
        .expect("EX7-024 in compiled cards");

    assert_eq!(compiled.card, "EX7-024");
    assert_eq!(compiled.name, "Shoemon");
    assert_eq!(compiled.kind, CompiledCardKind::Digimon);
    assert_eq!(compiled.level, Some(3));
    assert_eq!(compiled.color, vec![CompiledColor::Yellow]);
    assert_eq!(compiled.cost, Some(3));
    assert_eq!(compiled.dp, Some(1000));
    for trait_name in ["Puppet", "LIBERATOR"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }

    assert!(
        compiled.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::Digivolve
                && path.cost == Some(CompiledCost::Literal(0))
                && path.from.as_ref().is_some_and(|from| {
                    (from.level_eq == Some(2)
                        || from.all_of.iter().any(|pred| pred.level_eq == Some(2)))
                        && (from.color_is == Some(CompiledColor::Yellow)
                            || from
                                .all_of
                                .iter()
                                .any(|pred| pred.color_is == Some(CompiledColor::Yellow)))
                })
        }),
        "Shoemon must digivolve from a yellow Lv2 for cost 0"
    );

    assert_eq!(
        compiled.effects.len(),
        1,
        "EX7-024 should only encode the inherited opponent-security DP aura; \
         the source-scoped digivolve-into-trait cost reducer remains blocked"
    );
}

#[test]
#[ignore = "pending: DSL/engine gap - no when_this_digivolves_into + target_trait_has cost-reduction hook"]
fn ex7_024_cost_reduction_reduces_digivolving_into_puppet_by_one() {
    todo!("unignore when the DSL can express source-scoped digivolve-into-trait cost reduction")
}

#[test]
#[ignore = "pending: DSL/engine gap - same as ex7_024_cost_reduction_reduces_digivolving_into_puppet_by_one"]
fn ex7_024_cost_reduction_does_not_apply_to_non_puppet_target() {
    todo!("unignore when target-trait filtering is available to digivolution cost reduction")
}

#[test]
#[ignore = "pending: DSL/engine gap - same as ex7_024_cost_reduction_reduces_digivolving_into_puppet_by_one"]
fn ex7_024_cost_reduction_only_applies_on_your_turn() {
    todo!("unignore when source-scoped [Your Turn] digivolution cost reduction is available")
}

/// Behavioral: EX7-024 carried as an inherited card under an 8000-DP attacker
/// drops a 9000-DP opponent Security Digimon to 6000, so the attacker survives.
#[test]
fn ex7_024_inherited_security_dp_aura_drops_opponent_security_digimon_by_3000() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 YAML loads")
        .add_card(attacker_lv4("ATK", 8000))
        .add_card(digimon_security_card("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "EX7-024")
            .expect("EX7-024 registered");
        let next = game.next_card_index();
        let perm = &mut game.players[atk.player as usize].battle_area[atk.index as usize];
        let mut src = CardSource::new(data_idx, atk.player, next);
        src.card_index = next;
        perm.card_sources.insert(0, src);
    }

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "8000 DP attacker survives after Shoemon's -3000 drops 9000 security Digimon to 6000"
    );
    assert_eq!(runner.battle_area_size(0), 1);
}

/// Structural: the consult site (`attacker_security_dp_adjustment`) is
/// turn-agnostic and security battles only happen on the attacker's turn, so
/// `[Your Turn]` is behaviorally redundant. This test asserts the printed
/// qualifier is still faithfully encoded as an `active_when: your_turn` gate.
#[test]
fn ex7_024_inherited_security_dp_aura_only_applies_on_your_turn() {
    let runner = DebugRunner::builder()
        .dsl_card("EX7-024")
        .expect("EX7-024 YAML loads")
        .start();
    let compiled = runner
        .compiled_card("EX7-024")
        .expect("EX7-024 must be compiled");

    let active_when = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                active_when,
                applies_to_opponent_security_dp,
                ..
            }) if *applies_to_opponent_security_dp => Some(active_when.clone()),
            _ => None,
        })
        .expect("EX7-024 must compile an opponent-security DP aura")
        .expect("the aura must carry an active_when gate");

    assert_eq!(
        active_when.your_turn,
        Some(true),
        "[Your Turn] printed qualifier must be encoded as a your_turn gate"
    );
}

fn attacker_lv4(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card.play_cost = 5;
    card
}

fn digimon_security_card(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card
}
