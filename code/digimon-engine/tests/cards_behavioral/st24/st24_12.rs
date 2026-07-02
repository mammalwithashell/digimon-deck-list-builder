//! ST24-12 Falcomon — Digimon, Lv.3, Purple, DP 2000, Cost 3.
//! Traits: Avian, DATA SQUAD.
//!
//! # Card text (data/cards.json — verbatim, matches DCGO)
//! [On Play] By trashing the bottom face-down card from under any of your
//!   Tamers, you may return 1 Digimon card with the [DATA SQUAD] trait from your
//!   trash to the hand.
//! Inherited [When Attacking] [Once Per Turn] Delete 1 of your opponent's level 3
//!   Digimon.
//!
//! Printed digivolve box: [Digivolve] Lv.2 w/[DATA SQUAD] trait: Cost 0.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST24/Purple/ST24_12.cs
//!
//! # Patterns — On Play sister to ST23-12 (trash-return, trait-swapped) +
//! inherited [When Attacking][OPT] MANDATORY delete of an opponent Lv.3 Digimon.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "ST24-12";

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

fn ds_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["DATA SQUAD".to_string()];
    c
}

fn non_ds_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Mammal".to_string()];
    c
}

fn opp_lv3(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

fn opp_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
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
        .expect("ST24-12 in embedded DSL pack")
        .add_card(tamer("TAMER"))
        .add_card(ds_digimon("DS-DIGI"))
        .add_card(non_ds_digimon("NON-DS"))
        .add_card(opp_lv3("OPP-LV3"))
        .add_card(opp_lv4("OPP-LV4"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(8)
        .start()
}

fn tamer_with_face_down(r: &mut DebugRunner) -> PermanentHandle {
    let t = r.place_stack(0, &["FILLER", "TAMER"]);
    r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    t
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, handle: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(handle));
    r.game.drain_effect_queue();
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn st24_12_metadata_and_alt_path() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Falcomon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    let ds = card.alt_paths.iter().any(|p| {
        p.from
            .as_ref()
            .and_then(|f| f.trait_has.as_deref())
            .map(|t| t == "DATA SQUAD")
            .unwrap_or(false)
    });
    assert!(ds, "Lv.2 [DATA SQUAD] alt-path present");
}

#[test]
fn st24_12_has_on_play_and_inherited_when_attacking() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    let has_op = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnPlay) && t.scope != CompiledScope::Inherited)
    });
    assert!(has_op, "[On Play] clause present");
    let has_inherited_wa = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
            && t.when.contains(&CompiledTiming::WhenAttacking)
            && t.once_per_turn)
    });
    assert!(has_inherited_wa, "inherited [When Attacking][OPT] present");
}

// ─── Section 2 — On Play: trash FD under Tamer → return DATA SQUAD from trash ─

#[test]
fn st24_12_trashes_face_down_and_returns_ds_from_trash() {
    let mut runner = base();
    runner.set_first_player(0);
    let tamer = tamer_with_face_down(&mut runner);
    seed_trash(&mut runner, 0, "DS-DIGI");
    let arma = runner.place_on_field(0, CARD_ID, Some(0));

    let hand_before = runner.hand_size(0);
    let trash_before = runner.trash_size(0);
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    runner.fire_on_play(0, arma.index as usize);

    runner
        .accept_optional_trigger()
        .expect("accept the optional clause");
    let v = runner
        .pending_selection_view()
        .expect("Tamer pick installs");
    runner.execute_action(0, v.valid_action_ids[0]).unwrap();
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
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "1 DATA SQUAD Digimon left trash, 1 face-down source entered (net unchanged)"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "the [DATA SQUAD] Digimon returned to hand"
    );
}

#[test]
fn st24_12_no_activation_without_face_down_source() {
    let mut runner = base();
    runner.set_first_player(0);
    runner.place_on_field(0, "TAMER", Some(0)); // no FD source beneath
    seed_trash(&mut runner, 0, "DS-DIGI");
    let arma = runner.place_on_field(0, CARD_ID, Some(0));
    let hand_before = runner.hand_size(0);

    runner.fire_on_play(0, arma.index as usize);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.pending_selection.is_none(),
        "no face-down source ⇒ the activation cost is unpayable, no prompt"
    );
    assert_eq!(runner.hand_size(0), hand_before, "nothing returned");
}

// ─── Section 3 — Inherited [When Attacking][OPT] delete 1 opp Lv.3 Digimon ────

/// POSITIVE: a [When Attacking] deletes the opponent's Lv.3 Digimon.
#[test]
fn st24_12_inherited_deletes_opponent_lv3() {
    let mut r = base();
    r.set_first_player(0);
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    r.place_on_field(1, "OPP-LV3", Some(0));
    let opp_field_before = r.game.players[1].battle_area.len();

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    // DCGO canNoSelect:false → mandatory pick when a Lv.3 exists.
    let _ = r.auto_resolve();

    assert_eq!(
        r.game.players[1].battle_area.len(),
        opp_field_before - 1,
        "the opponent's Lv.3 Digimon was deleted"
    );
}

/// FILTER: a Lv.4 opponent Digimon is not a legal delete target ⇒ clean no-op.
#[test]
fn st24_12_inherited_does_not_delete_lv4() {
    let mut r = base();
    r.set_first_player(0);
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    r.place_on_field(1, "OPP-LV4", Some(0));
    let opp_field_before = r.game.players[1].battle_area.len();

    fire(&mut r, EffectTiming::WhenAttacking, carrier);
    let _ = r.auto_resolve();

    assert!(
        r.game.pending_selection.is_none(),
        "no Lv.3 target ⇒ no stuck selection"
    );
    assert_eq!(
        r.game.players[1].battle_area.len(),
        opp_field_before,
        "the Lv.4 Digimon is not a legal target — nothing deleted"
    );
}
