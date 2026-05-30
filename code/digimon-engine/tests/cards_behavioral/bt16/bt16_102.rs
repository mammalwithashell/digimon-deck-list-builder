//! BT16-102 Magnamon (X Antibody) — Digimon, Yellow/Blue/White, Lv6, cost 12, DP 12000.
//!
//! # Card text (cards.json)
//! - `<Blocker>`, `<Armor Purge>`
//! - **[When Digivolving]** If [Magnamon (X Antibody)] or an [Armor Form] trait
//!   card is in this Digimon's digivolution cards, this Digimon isn't affected by
//!   your opponent's effects and gains +3000 DP until the end of your opponent's
//!   turn. Then, unsuspend this Digimon.
//! - **[All Turns] [Once Per Turn]** When a card is removed from a security
//!   stack, you may activate 1 of this Digimon's [When Digivolving] effects.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/BT16/Yellow/BT16_102.cs`
//!
//! The opponent-effect immunity is exercised behaviorally by judge-quiz Q17
//! (`tests/judge_quiz/a_immunity_scope.rs`) — it suppresses a Lilithmon-granted
//! `[End of Your Turn] Delete this`. These tests cover the card's structure.

#![allow(unused_imports)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledColor, CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

const CARD_ID: &str = "BT16-102";

/// Structure: tri-color Lv6 Digimon, cost 12, with all four printed clauses.
#[test]
fn bt16_102_is_tricolor_lv6_with_four_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT16-102 compiles from the embedded pack")
        .memory(10)
        .start();
    let card = runner.compiled_card(CARD_ID).expect("BT16-102 registered");

    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    for c in [CompiledColor::Yellow, CompiledColor::Blue, CompiledColor::White] {
        assert!(card.color.contains(&c), "BT16-102 is Yellow/Blue/White; got {:?}", card.color);
    }
    assert_eq!(
        card.effects.len(),
        4,
        "BT16-102 has 4 clauses (<Blocker>, <Armor Purge>, [When Digivolving], \
         [All Turns][OPT] security re-activation); got {}",
        card.effects.len()
    );
}

/// The [When Digivolving] clause grants opponent-effect immunity (the Q17
/// load-bearing clause) — at least one `GrantEffectImmunity` step is present.
#[test]
fn bt16_102_when_digivolving_grants_opponent_effect_immunity() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT16-102 compiles")
        .memory(10)
        .start();
    let card = runner.compiled_card(CARD_ID).expect("BT16-102 registered");

    let wd = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::WhenDigivolving) => {
                Some(t)
            }
            _ => None,
        })
        .expect("BT16-102 must carry a [When Digivolving] clause");

    let immunity_grants = wd
        .process
        .iter()
        .filter(|s| matches!(s, CompiledStep::GrantEffectImmunity { .. }))
        .count();
    assert!(
        immunity_grants >= 1,
        "[When Digivolving] must grant opponent-effect immunity; got {immunity_grants} \
         GrantEffectImmunity steps in {:?}",
        wd.process
    );
}
