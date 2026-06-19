//! BT22-025 UlforceVeedramon — Digimon, Lv.6, Blue, DP 12000, Cost 7.
//! Traits: Holy Warrior / Royal Knight / CS. ACE (Overflow -4).
//!
//! # Card text (cards.json / card image)
//!
//! ```text
//! [Hand] [Counter] <Blast Digivolve> (Your Digimon may digivolve into this
//!   card without paying the cost.)
//! [On Play] [When Digivolving] Activate 1 of the effects below:
//! ・Return 1 of your opponent's lowest level Digimon to the bottom of the deck.
//! ・You may play 1 play cost 4 or lower blue Tamer card from your hand without
//!   paying the cost.
//! [When Attacking] [Once Per Turn] This Digimon may unsuspend.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT22/Blue/BT22_025.cs
//!
//! # Implemented slice
//! - [Hand][Counter] <Blast Digivolve>: `burst_digivolve` alt-path marker
//!   (cost 0) + `grant_keyword: BlastDigivolve`. PRECEDENT: BT22-052.
//! - [On Play][When Digivolving] modal via `select_effect_choice`:
//!   - branch 0: `select_opponent_permanent` gated to the opponent's
//!     lowest-level Digimon (`level_matches_aggregate { selector: lowest_level,
//!     of: opponent }`) then `return_to_deck { position: bottom }`. PRECEDENT:
//!     BT22-026 (same aggregate, but to hand).
//!   - branch 1: optional `select_hand` filtered to blue Tamer cost <= 4 then
//!     `play_from_hand_free`. PRECEDENT: BT20-091 / BT20-100 free tamer play.
//! - [When Attacking][OPT] this Digimon may unsuspend (pre-existing).

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT22-025";

fn opp_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.colors = vec![CardColor::Purple];
    c
}

fn blue_tamer(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, "BlueTamer");
    c.card_kind = CardKind::Tamer;
    c.colors = vec![CardColor::Blue];
    c.play_cost = cost;
    c
}

// ════════════════════════════════════════════════════════════════════════════
// Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt22_025_loads_when_attacking_unsuspend_slice() {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-025 must load from embedded DSL pack")
        .start();
}

#[test]
fn bt22_025_has_blast_digivolve_marker_and_keyword() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-025 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    assert!(
        card.alt_paths.iter().any(|path| {
            path.kind == CompiledAltPathKind::BurstDigivolve
                && path.marker
                && path.cost == Some(CompiledCost::Literal(0))
        }),
        "BT22-025 must register a <Blast Digivolve> alt-path marker at cost 0"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "BlastDigivolve"
        )),
        "BT22-025 must grant the BlastDigivolve keyword"
    );
}

#[test]
fn bt22_025_has_on_play_when_digivolving_modal_clause() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-025 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
        )),
        "BT22-025 must have an [On Play][When Digivolving] modal clause"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Modal — branch 0: bottom-deck opponent's lowest-level Digimon
// ════════════════════════════════════════════════════════════════════════════

/// The modal first surfaces an EffectChoice with two branches.
#[test]
fn bt22_025_modal_offers_two_branches() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-025 loads")
        .add_card(opp_digimon("OPP-LOW", "OppLow", 3, 3000))
        .memory(5)
        .start();
    let uv = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-LOW", Some(0));

    r.fire_on_play(0, uv.index as usize);

    let kind = r
        .pending_kind()
        .expect("modal EffectChoice prompt must install");
    assert_eq!(kind, SelectionKind::EffectChoice);
    let view = r.pending_selection_view().unwrap();
    assert_eq!(
        view.effect_choices.as_ref().unwrap().len(),
        2,
        "two modal branches"
    );
}

/// POSITIVE branch 0: only the opponent's lowest-LEVEL Digimon is a legal
/// bottom-deck target; the chosen Digimon returns to the bottom of the deck.
#[test]
fn bt22_025_branch0_returns_lowest_level_to_deck_bottom() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-025 loads")
        .add_card(opp_digimon("OPP-LOW", "OppLow", 3, 3000))
        .add_card(opp_digimon("OPP-HIGH", "OppHigh", 6, 12000))
        .deck(1, &["OPP-LOW"])
        .memory(5)
        .start();
    let uv = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(1, "OPP-LOW", Some(0));
    r.place_on_field(1, "OPP-HIGH", Some(0));

    let deck_before = r.game.players[1].deck.len();
    r.fire_on_play(0, uv.index as usize);

    // Branch 0 = return lowest-level to deck bottom.
    r.execute_branch(0).expect("pick branch 0");

    // Only the level-3 Digimon should be a legal target (lowest level).
    let view = r
        .pending_selection_view()
        .expect("opponent-field target prompt for the lowest-level Digimon");
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the single lowest-level Digimon is a legal target"
    );
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("return the lowest-level Digimon");
    let _ = r.auto_resolve();

    assert!(
        r.game.players[1]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&r.game.card_data) != "OPP-LOW"),
        "the lowest-level Digimon left the battle area"
    );
    assert_eq!(
        r.game.players[1].deck.len(),
        deck_before + 1,
        "the returned Digimon goes to the bottom of the deck"
    );
    assert_eq!(
        r.game.players[1]
            .deck
            .last()
            .unwrap()
            .card_id(&r.game.card_data),
        "OPP-LOW",
        "it is placed at the BOTTOM of the deck"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Modal — branch 1: play a blue Tamer cost <= 4 from hand for free
// ════════════════════════════════════════════════════════════════════════════

/// POSITIVE branch 1: a cost-4 blue Tamer in hand is playable for free.
#[test]
fn bt22_025_branch1_plays_blue_tamer_cost_le_4_free() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-025 loads")
        .add_card(blue_tamer("TAMER-4", 4))
        .hand(0, &["TAMER-4"])
        .memory(5)
        .start();
    let uv = r.place_on_field(0, CARD_ID, Some(0));

    let field_before = r.battle_area_size(0);
    r.fire_on_play(0, uv.index as usize);
    r.execute_branch(1).expect("pick branch 1");

    let view = r
        .pending_selection_view()
        .expect("hand prompt for the blue Tamer");
    assert!(r.pending_is_optional(), "the Tamer play is 'you may'");
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("play the blue Tamer");
    let _ = r.auto_resolve();

    assert_eq!(
        r.battle_area_size(0),
        field_before + 1,
        "the blue Tamer is played to the battle area"
    );
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "TAMER-4"),
        "the played card is the blue Tamer"
    );
}

/// NEGATIVE branch 1 filter: a cost-5 blue Tamer and a non-blue Tamer are NOT
/// legal picks; the optional play can be declined.
#[test]
fn bt22_025_branch1_filter_excludes_cost_5_and_non_blue() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT22-025 loads")
        .add_card(blue_tamer("TAMER-5", 5))
        .add_card({
            let mut c = make_test_card("TAMER-RED", "RedTamer");
            c.card_kind = CardKind::Tamer;
            c.colors = vec![CardColor::Red];
            c.play_cost = 3;
            c
        })
        .hand(0, &["TAMER-5", "TAMER-RED"])
        .memory(5)
        .start();
    let uv = r.place_on_field(0, CARD_ID, Some(0));

    r.fire_on_play(0, uv.index as usize);
    r.execute_branch(1).expect("pick branch 1");

    // No legal Tamer to play (cost-5 blue + cost-3 red both excluded). The
    // optional select must still let the player decline / be a no-op.
    if let Some(view) = r.pending_selection_view() {
        assert!(
            !view
                .valid_action_ids
                .iter()
                .any(|&a| a != digimon_engine::action::space::PASS),
            "neither the cost-5 blue Tamer nor the red Tamer is a legal pick"
        );
    }
    let _ = r.auto_resolve();
    assert_eq!(
        r.battle_area_size(0),
        1,
        "only UlforceVeedramon remains; no illegal Tamer was played"
    );
}
