//! ST23-12 Chiropmon — Digimon, Lv.3, Purple, DP 2000, Cost 3.
//! Traits: Mammal, Glowing Dawn, BEATBREAK. Attribute: Virus.
//!
//! # Card text (data/cards.json — verbatim)
//! [On Play] By trashing the bottom face-down card from under any of your
//! Tamers, you may return 1 Digimon card with the [Glowing Dawn] trait from your
//! trash to the hand.
//! Inherited: ＜Retaliation＞.
//!
//! Printed digivolve box: [Digivolve] Lv.2 w/[Glowing Dawn] trait: Cost 0.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/Purple/ST23_12.cs
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - A4 trash → hand recursion (gated on a paid cost)
//! - cost verb trash_bottom_face_down_source_under_tamer (tail runs iff trashed)
//! - inherited <Retaliation> keyword grant

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const CARD_ID: &str = "ST23-12";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Purple];
    c
}

/// A Glowing-Dawn Digimon we seed into the trash to be returned.
fn gd_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

fn non_gd_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Mammal".to_string()];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn seed_trash(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .trash
        .push(CardSource::new(data_idx, p, next_idx));
}

fn base() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-12 in embedded DSL pack")
        .add_card(tamer("TAMER"))
        .add_card(gd_digimon("GD-DIGI"))
        .add_card(non_gd_digimon("NON-GD"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(8)
        .start()
}

/// Place a Tamer with one face-down source beneath it (the cost source).
fn tamer_with_face_down(r: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    let t = r.place_stack(0, &["FILLER", "TAMER"]);
    r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    t
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st23_12_metadata_and_alt_path() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Chiropmon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    let gd = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .and_then(|f| f.trait_has.as_deref())
            .map(|t| t == "Glowing Dawn")
            .unwrap_or(false)
    });
    assert!(gd, "Lv.2 [Glowing Dawn] alt-path present");
}

#[test]
fn st23_12_has_on_play_clause_and_inherited_retaliation() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let has_op = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
    ));
    assert!(has_op, "[On Play] clause present");
    let has_ret = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
            if keyword == "Retaliation" && *scope == CompiledScope::Inherited
    ));
    assert!(has_ret, "inherited <Retaliation> present");
}

// ─── Section 2 — Behavioral ──────────────────────────────────────────────────

/// POSITIVE: with a Tamer holding a face-down source and a [Glowing Dawn]
/// Digimon in trash, the On Play trashes the source and returns the trash
/// Digimon to hand.
#[test]
fn st23_12_trashes_face_down_and_returns_glowing_dawn_from_trash() {
    let mut runner = base();
    runner.set_first_player(0);
    let tamer = tamer_with_face_down(&mut runner);
    seed_trash(&mut runner, 0, "GD-DIGI");
    let arma = runner.place_on_field(0, CARD_ID, Some(0));

    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    runner.fire_on_play(0, arma.index as usize);

    // The clause is optional ("you may"): accept the activation prompt first.
    runner
        .accept_optional_trigger()
        .expect("accept the optional clause");
    // The Tamer pick (cost): pick the only eligible Tamer.
    let v = runner.pending_selection_view().expect("Tamer pick installs");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    // The [Glowing Dawn] Digimon in trash to return.
    let v2 = runner
        .pending_selection_view()
        .expect("trash return pick installs after the cost is paid");
    runner.execute_action(0, v2.valid_action_ids[0]).unwrap();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        tamer_sources_before - 1,
        "the Tamer's bottom face-down source was trashed (cost)"
    );
    // Trash: -1 returned GD Digimon, +1 the trashed face-down source = net 0.
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "1 GD Digimon left trash, 1 face-down source entered (net unchanged)"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "the [Glowing Dawn] Digimon returned to hand"
    );
}

/// DECLINE the return: paying the cost still trashes the face-down source, but
/// declining the optional return leaves the trash Digimon where it is.
#[test]
fn st23_12_return_is_optional() {
    let mut runner = base();
    runner.set_first_player(0);
    let tamer = tamer_with_face_down(&mut runner);
    seed_trash(&mut runner, 0, "GD-DIGI");
    let arma = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);

    runner.fire_on_play(0, arma.index as usize);
    // The clause is optional ("you may"): accept the activation prompt first.
    runner
        .accept_optional_trigger()
        .expect("accept the optional clause");
    // Tamer pick (the cost): pick the only eligible Tamer.
    let tamer_pick = runner.pending_selection_view().expect("Tamer pick installs");
    runner
        .execute_action(0, tamer_pick.valid_action_ids[0])
        .unwrap();
    // Return pick — optional ("you may return"): decline it.
    let v2 = runner.pending_selection_view().expect("return pick installs");
    assert!(v2.is_optional, "the return is optional ('you may return')");
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the return");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declined the return ⇒ nothing added to hand"
    );
    assert_eq!(
        runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        1,
        "the face-down source was still trashed (cost paid)"
    );
}

/// NEGATIVE (cost gate): no Tamer with a face-down source ⇒ the cost is
/// unpayable, the whole effect is a clean no-op, and the trash Digimon stays.
#[test]
fn st23_12_no_face_down_source_clean_noop() {
    let mut runner = base();
    runner.set_first_player(0);
    // A Tamer is on the field but with NO face-down source beneath it.
    runner.place_on_field(0, "TAMER", Some(0));
    seed_trash(&mut runner, 0, "GD-DIGI");
    let arma = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);

    runner.fire_on_play(0, arma.index as usize);
    let _ = runner.auto_resolve();

    assert_eq!(runner.hand_size(0), hand_before, "no cost ⇒ no return");
    assert_eq!(runner.trash_size(0), trash_before, "no cost ⇒ trash unchanged");
}

/// NEGATIVE (trash filter): with the cost payable but only a NON-Glowing-Dawn
/// Digimon in trash, the return finds no legal target — the cost is still paid
/// (DCGO trashes first) but nothing returns.
#[test]
fn st23_12_no_glowing_dawn_in_trash_returns_nothing() {
    let mut runner = base();
    runner.set_first_player(0);
    let _tamer = tamer_with_face_down(&mut runner);
    seed_trash(&mut runner, 0, "NON-GD");
    let arma = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);

    runner.fire_on_play(0, arma.index as usize);
    let v = runner.pending_selection_view().expect("Tamer pick installs");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "no [Glowing Dawn] Digimon in trash ⇒ nothing returned"
    );
}
