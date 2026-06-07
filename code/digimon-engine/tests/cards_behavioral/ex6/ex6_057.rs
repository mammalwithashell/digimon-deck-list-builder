//! EX6-057 Lilithmon — Digimon, Purple, Lv6, cost 11, DP 11000.
//!
//! # Card text (cards.json)
//! - **[On Play] [When Digivolving]** Until the end of your opponent's turn, 1
//!   of your opponent's Digimon gains "[End of Your Turn] Delete this Digimon."
//! - **[All Turns] [Once Per Turn]** When this Digimon would leave the battle
//!   area other than in battle, by deleting 1 level 5 or lower Digimon, prevent
//!   it from leaving.
//! - **[Opponent's Turn] [Once Per Turn]** When another Digimon is deleted,
//!   trash the top card of your opponent's security stack.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/EX6/Purple/EX6_057.cs`
//!
//! The full grant-and-delete behavior (and the `<Partition>`-skip interaction
//! that makes the granted self-delete the carrier's OWN effect) is pinned by
//! judge-quiz Q16 in `tests/judge_quiz/e_partition_digixros.rs`. These tests
//! cover the card's structure + the [On Play] grant install.

#![allow(unused_imports)]

use digimon_dsl::compiled::{CompiledCard, CompiledCardKind, CompiledClause, CompiledColor};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};

const CARD_ID: &str = "EX6-057";

fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.dp = Some(5000);
    c
}

/// Structure: Purple Lv6 Digimon, cost 11, with all three printed clauses.
#[test]
fn ex6_057_is_purple_lv6_digimon_with_three_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX6-057 compiles from the embedded pack")
        .memory(10)
        .start();
    let card = runner.compiled_card(CARD_ID).expect("EX6-057 registered");

    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert!(
        card.color.iter().any(|c| matches!(c, CompiledColor::Purple)),
        "EX6-057 is a Purple card; got {:?}",
        card.color
    );
    assert_eq!(
        card.effects.len(),
        3,
        "EX6-057 has 3 clauses (grant / leave-prevention replacement / on-deletion \
         security trash); no-approximations forbids omitting any; got {}",
        card.effects.len()
    );
}

/// [On Play] grant: selecting 1 opponent Digimon installs an
/// `EndOfYourTurn` granted-trigger on it (the "[EoT] Delete this" clause).
#[test]
fn ex6_057_on_play_grants_eot_delete_to_selected_opponent_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX6-057 compiles")
        .add_card(opp_digimon("OPPMON"))
        .memory(10)
        .start();
    r.skip_mulligan();

    let opp = r.place_stack(1, &["OPPMON"]);
    let lilithmon = r.place_on_field(0, CARD_ID, None);
    r.fire_on_play(0, lilithmon.index as usize);

    // Resolve the [On Play] target select — OPPMON is the only opponent Digimon.
    if let Some(sel) = r.game.pending_selection.as_ref() {
        let who = sel.selecting_player;
        let aid = sel.valid_action_ids[0];
        let _ = r.game.resolve_selection(who, aid);
    }

    assert!(
        !r.game
            .modifiers
            .granted_triggered_for_timing(opp, EffectTiming::EndOfYourTurn)
            .is_empty(),
        "EX6-057 [On Play] must grant the selected opponent Digimon an \
         [End of Your Turn] effect"
    );
}
