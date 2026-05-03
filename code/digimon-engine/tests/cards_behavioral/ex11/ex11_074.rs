//! EX11-074 Vortexdramon — focused readiness slice.
//!
//! This test intentionally covers only the currently supported static keywords
//! and the effect-battle pathway. Remaining Zephagamon/Vortexdramon card text
//! gaps are tracked in docs and QA gap files.

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::{EffectTiming, TriggerSource};

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("EX11-074")
}

fn fighter(id: &str, dp: i32) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card
}

#[test]
fn ex11_074_compiles_static_keywords_and_on_suspend_battle_slice() {
    let card = compiled();
    assert_eq!(card.level, Some(7));
    assert_eq!(card.dp, Some(14000));

    let keywords: Vec<_> = card
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();
    assert!(keywords.contains(&"Piercing"));
    assert!(keywords.contains(&"Vortex"));
    assert!(keywords.contains(&"Blocker"));

    let has_on_suspend_battle = card.effects.iter().any(|clause| match clause {
        CompiledClause::Triggered(triggered) => {
            triggered.when.contains(&CompiledTiming::OnSuspend)
                && triggered
                    .process
                    .iter()
                    .any(|step| matches!(step, CompiledStep::Battle { .. }))
        }
        _ => false,
    });
    assert!(
        has_on_suspend_battle,
        "EX11-074 readiness slice must include an OnSuspend triggered battle step"
    );
}

#[test]
fn ex11_074_effect_battle_deletes_defender_without_piercing_security_check() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-074")
        .expect("EX11-074 in embedded pack")
        .add_card(fighter("EX11-074-DEF", 3000))
        .add_card(make_test_card("EX11-074-SEC", "Security"))
        .security(1, &["EX11-074-SEC", "EX11-074-SEC", "EX11-074-SEC"])
        .start();

    let attacker = runner.place_on_field(0, "EX11-074", Some(0));
    runner.place_on_field(1, "EX11-074-DEF", Some(0));
    assert!(
        runner.game.has_keyword(attacker, Keyword::Piercing),
        "readiness fixture relies on EX11-074's printed Piercing keyword"
    );

    let security_before = runner.security_count(1);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnSuspend, TriggerSource::Permanent(attacker));
    runner.game.drain_effect_queue();

    let (selecting_player, action_id) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("battle target selection");
        assert_eq!(pending.valid_action_ids.len(), 1);
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .execute_action(selecting_player, action_id)
        .expect("battle selection resolves");

    assert_eq!(runner.battle_area_size(1), 0, "defender should be deleted");
    assert_eq!(
        runner.security_count(1),
        security_before,
        "effect battle must not continue into Piercing security checks"
    );
    assert!(
        runner.game.pending_attack.is_none(),
        "effect battle must not leave an attack pending"
    );
}
