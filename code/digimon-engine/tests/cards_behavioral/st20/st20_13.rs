//! ST20-13 Tai Kamiya & Izzy Izumi — Tamer, Black+Green, Cost 3,
//! traits: [ADVENTURE].
//!
//! # Card text (official bundle `data/card_bundles/ST20-13.md`)
//!
//! ```text
//! [Your Turn] When any Digimon cards with the [ADVENTURE] trait would be played
//! from your hand, by suspending this Tamer, reduce the play cost by 1.
//! [Opponent's Turn] All of your Digimon with the [ADVENTURE] trait gain
//! <Blocker>.
//! Security Effect: [Security] Play this card without paying the cost.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST20/Black/ST20_13.cs` — BeforePayCost
//! reducer identical to ST20_12; `BlockerStaticEffect(PermanentCondition =
//! own battle-area Digimon && TopCard.HasAdventureTraits, condition =
//! IsExistOnBattleArea && IsOpponentTurn)`; `PlaySelfTamerSecurityEffect`.
//! The reducer suite is shared with ST20-12 (`st20::st20_12`).

use digimon_dsl::compiled::{CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledTiming};
use digimon_engine::enums::Keyword;

use super::st20_12 as suite;

const CARD_ID: &str = "ST20-13";

#[test]
fn st20_13_identity_and_clauses() {
    suite::assert_adventure_tamer_identity(CARD_ID, [CompiledColor::Black, CompiledColor::Green]);
    let r = suite::builder(&[CARD_ID]).start();
    let card = r.compiled_card(CARD_ID).unwrap();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.when.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(triggered, vec![vec![CompiledTiming::OnSecurity]]);
    assert_eq!(
        card.effects
            .iter()
            .filter(|c| matches!(c, CompiledClause::Declarative(CompiledDeclarativeClause::Aura { .. })))
            .count(),
        1,
        "one [Opponent's Turn] Blocker aura"
    );
}

#[test]
fn st20_13_reduction_accept_pays_one_less_and_suspends() {
    suite::reduction_accept_pays_one_less_and_suspends(CARD_ID);
}

#[test]
fn st20_13_reduction_decline_pays_full() {
    suite::reduction_decline_pays_full(CARD_ID);
}

#[test]
fn st20_13_reduction_only_for_adventure_digimon() {
    suite::reduction_only_for_adventure_digimon(CARD_ID);
}

#[test]
fn st20_13_reduction_not_offered_when_suspended() {
    suite::reduction_not_offered_when_suspended(CARD_ID);
}

#[test]
fn st20_13_reduction_not_on_opponents_turn() {
    suite::reduction_not_on_opponents_turn(CARD_ID);
}

/// Q&A (bundle): with ST20-12 the play cost is reduced by a total of 2.
#[test]
fn st20_13_reduction_stacks_with_st20_12_qa() {
    suite::reduction_stacks_across_tamers(CARD_ID, "ST20-12");
}

#[test]
fn st20_13_reduction_each_tamer_is_its_own_choice() {
    suite::reduction_each_tamer_is_its_own_choice(CARD_ID, "ST20-12");
}

#[test]
fn st20_13_reduction_not_on_digivolve() {
    suite::reduction_not_on_digivolve(CARD_ID);
}

#[test]
fn st20_13_security_plays_self_free() {
    suite::security_plays_self_free(CARD_ID);
}

/// `[Opponent's Turn] All of your Digimon with the [ADVENTURE] trait gain
/// <Blocker>.` — own ADVENTURE Digimon only, only on the opponent's turn, and
/// gone when the Tamer leaves.
#[test]
fn st20_13_opponents_turn_blocker_aura() {
    let mut r = suite::builder(&[CARD_ID])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .memory(0)
        .start();
    let t = r.place_on_field(0, CARD_ID, Some(0));
    let own_adv = r.place_on_field(0, "ADV-DIGI", Some(0));
    let own_plain = r.place_on_field(0, "PLAIN-DIGI", Some(0));
    let own_adv_tamer = r.place_on_field(0, "ADV-TAMER", Some(0));
    let opp_adv = r.place_on_field(1, "ADV-DIGI", Some(0));

    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(own_adv, Keyword::Blocker), "not on your turn");

    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 1);
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(own_adv, Keyword::Blocker));
    assert!(!r.game.has_keyword(own_plain, Keyword::Blocker));
    assert!(!r.game.has_keyword(own_adv_tamer, Keyword::Blocker), "Digimon only");
    assert!(!r.game.has_keyword(opp_adv, Keyword::Blocker), "your Digimon only");
    assert!(!r.game.has_keyword(t, Keyword::Blocker));

    // Source leaves → grant clears on refresh.
    r.game.players[0].battle_area.remove(t.index as usize);
    r.game.tick_declarative_effects();
    let adv_now = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == "ADV-DIGI")
        .map(|i| digimon_engine::permanent::PermanentHandle { player: 0, index: i as u8 })
        .unwrap();
    assert!(!r.game.has_keyword(adv_now, Keyword::Blocker));
}
