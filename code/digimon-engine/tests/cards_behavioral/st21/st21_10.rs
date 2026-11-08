//! ST21-10 Gabumon — Digimon, Lv.3, Purple, DP 1000, Cost 3.
//! Traits: Reptile / ADVENTURE. Form: Rookie. Attribute: Data.
//!
//! # Card text (official bundle `data/card_bundles/ST21-10.md`)
//!
//! ```text
//! Digivolve: Purple Lv.2 / cost 0; Blue Lv.2 / cost 0.
//! [Digivolve] Lv.2 w/[ADVENTURE] trait: Cost 0
//!
//! [Your Turn] While your opponent has a Digimon with 10000 DP or more, or
//! your Tamers have 3 or more total colors, this Digimon can digivolve into
//! [MetalGarurumon] in the hand for a digivolution cost of 4, ignoring
//! digivolution requirements.
//!
//! Inherited: [When Attacking] [Once Per Turn] <Draw 1>. Then, trash 1 card
//! in your hand.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/Purple/ST21_10.cs` — warp
//! `AddSelfDigivolutionRequirementStaticEffect(self, name MetalGarurumon,
//! cost 4, ignore requirements)` gated on owner turn && (opp DP >= 10000 ||
//! Tamer colours >= 3); inherited OnAllyAttack OPT draw 1 then a mandatory
//! 1-card hand discard. (DCGO's alt requirement also admits [Hero]; the printed
//! box says [ADVENTURE] only, and the print governs.)
//!
//! Note: the generated per-card JSON `cards/st21/ST21-10.json` lists only the
//! Purple circle; the official DB (and `data/cards.json`) also has Blue Lv.2/0.

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{encode_digivolve, HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "ST21-10";

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c
}

fn lv2(id: &str, color: CardColor, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(2);
    c.colors = vec![color];
    if adventure {
        c.traits = vec!["ADVENTURE".to_string()];
    }
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut metal = make_test_card("METALGARURUMON", "MetalGarurumon");
    metal.level = Some(6);
    metal.dp = Some(12000);
    metal.play_cost = 12;
    let mut other6 = make_test_card("OTHER-LV6", "WarGreymon");
    other6.level = Some(6);
    other6.dp = Some(12000);
    let mut big = make_test_card("BIG-DP", "Big");
    big.level = Some(6);
    big.dp = Some(10000);
    let mut small = make_test_card("SMALL-DP", "Small");
    small.level = Some(5);
    small.dp = Some(9000);
    let mut carrier = make_test_card("CARRIER", "Carrier");
    carrier.level = Some(4);
    carrier.dp = Some(4000);
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-10 YAML parses, compiles and is in the embedded pack")
        .add_card(metal)
        .add_card(other6)
        .add_card(big)
        .add_card(small)
        .add_card(tamer("T-RED", CardColor::Red))
        .add_card(tamer("T-BLUE", CardColor::Blue))
        .add_card(tamer("T-GREEN", CardColor::Green))
        .add_card(lv2("BLUE-LV2", CardColor::Blue, false))
        .add_card(lv2("YELLOW-LV2", CardColor::Yellow, false))
        .add_card(lv2("ADV-YELLOW-LV2", CardColor::Yellow, true))
        .add_card(carrier)
        .add_card(make_test_card("FILL", "Fill"))
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.players[h.player as usize].battle_area[h.index as usize].top_card().card_id(&runner.game.card_data).to_string()
}

fn warp_runner(opp: Option<&str>, tamers: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .hand(0, &["METALGARURUMON", "OTHER-LV6"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .memory(20)
        .start();
    runner.skip_mulligan();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    if let Some(o) = opp {
        runner.place_on_field(1, o, Some(0));
    }
    for t in tamers {
        runner.place_on_field(0, t, None);
    }
    (runner, me)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st21_10_identity_and_paths() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((card.level, card.dp, card.cost), (Some(3), Some(1000), Some(3)));
    let paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(paths, 4, "Purple + Blue Lv.2/0 circles, Lv.2 w/[ADVENTURE]/0 box, warp into [MetalGarurumon]");
    let inherited: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(inherited.len(), 1);
    assert_eq!(inherited[0].when, vec![CompiledTiming::WhenAttacking]);
    assert!(inherited[0].once_per_turn, "[Once Per Turn]");
    assert!(!inherited[0].optional);
}

#[test]
fn st21_10_digivolve_circles_and_box() {
    for (base, ok) in [("BLUE-LV2", true), ("ADV-YELLOW-LV2", true), ("YELLOW-LV2", false)] {
        let mut runner = builder().hand(0, &[CARD_ID]).deck(0, &["FILL", "FILL"]).memory(5).start();
        runner.skip_mulligan();
        let b = runner.place_on_field(0, base, Some(0));
        runner.game.decode_action(encode_digivolve(0, b.index as u16), 0);
        assert_eq!(top_id(&runner, b) == CARD_ID, ok, "digivolve from {base}");
    }
}

// ─── Section 2 — warp into [MetalGarurumon] ──────────────────────────────────

#[test]
fn st21_10_warp_via_opponent_10000_dp() {
    let (mut runner, me) = warp_runner(Some("BIG-DP"), &[]);
    let before = runner.memory();
    assert!(runner.game.digivolve_from_hand(0, 0, me.index as usize, PlaySource::ByHand));
    assert_eq!(top_id(&runner, me), "METALGARURUMON");
    assert_eq!(before - runner.memory(), 4);
}

#[test]
fn st21_10_warp_via_three_tamer_colors() {
    let (mut runner, me) = warp_runner(None, &["T-RED", "T-BLUE", "T-GREEN"]);
    let before = runner.memory();
    assert!(runner.game.digivolve_from_hand(0, 0, me.index as usize, PlaySource::ByHand));
    assert_eq!(top_id(&runner, me), "METALGARURUMON");
    assert_eq!(before - runner.memory(), 4);
}

#[test]
fn st21_10_warp_blocked_without_either_condition() {
    let (mut runner, me) = warp_runner(Some("SMALL-DP"), &["T-RED", "T-BLUE"]);
    assert!(!runner.game.digivolve_from_hand(0, 0, me.index as usize, PlaySource::ByHand));
    assert_eq!(top_id(&runner, me), CARD_ID);
}

#[test]
fn st21_10_warp_only_into_metalgarurumon() {
    let (mut runner, me) = warp_runner(Some("BIG-DP"), &[]);
    assert!(!runner.game.digivolve_from_hand(0, 1, me.index as usize, PlaySource::ByHand));
    assert_eq!(top_id(&runner, me), CARD_ID);
}

#[test]
fn st21_10_warp_blocked_on_opponents_turn() {
    let (mut runner, me) = warp_runner(Some("BIG-DP"), &[]);
    runner.game.turn_player_idx = 1;
    assert!(!runner.game.digivolve_from_hand(0, 0, me.index as usize, PlaySource::ByHand));
}

// ─── Section 3 — inherited [When Attacking][OPT] draw 1, trash 1 ─────────────

fn hand_ids(runner: &DebugRunner) -> Vec<String> {
    runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

#[test]
fn st21_10_inherited_when_attacking_draws_then_trashes_once_per_turn() {
    let mut runner = builder()
        .hand(0, &["OTHER-LV6"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.attack_player(carrier, 1, false);
    // Draw 1 happened; now the mandatory trash prompt is pending.
    let view = runner.pending_selection_view().expect("trash prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(!view.is_optional, "the trash is mandatory");
    assert_eq!(hand_ids(&runner).len(), 2, "drew 1");
    let slot = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "OTHER-LV6")
        .unwrap() as u16;
    let action = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .expect("any hand card may be trashed");
    runner.execute_action(view.selecting_player, action).expect("trash");
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner), vec!["FILL"]);
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "OTHER-LV6"));

    // Second attack in the same turn: [Once Per Turn] — no draw/trash.
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    let hand_before = runner.hand_size(0);
    runner.attack_player(carrier, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), hand_before, "OPT: no second draw");
}

#[test]
fn st21_10_inherited_does_not_apply_to_gabumon_itself() {
    let mut runner = builder()
        .hand(0, &["OTHER-LV6"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    runner.skip_mulligan();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.attack_player(me, 1, false);
    let _ = runner.auto_resolve();
    assert_eq!(hand_ids(&runner), vec!["OTHER-LV6"], "inherited text is inactive on the top card");
}

#[allow(dead_code)]
fn _silence() {
    let _ = PASS;
}
