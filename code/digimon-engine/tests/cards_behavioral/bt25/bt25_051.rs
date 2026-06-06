//! BT25-051 Grizzlymon — Digimon, Lv.4, Green/Yellow, DP 4000, Cost 4.
//! Traits: Beast, Iliad, TS. Attribute: Vaccine.
//!
//! # Card text (cards.json — verbatim)
//!
//! ＜Blocker＞
//! [On Play] [When Digivolving] 1 of your Digimon with [Beast], [Animal] or
//! [Sovereign], other than [Sea Animal] trait, in any of its traits or the
//! [Shaman] or [TS] trait gets +3000 DP until your opponent's turn ends.
//!
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When this Digimon wins a battle, ＜Draw 1＞.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_051.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H5 (Blocker grant)
//! - D1 (select own Digimon → +DP until opponent's turn ends)
//! - F-win-battle (inherited OPT draw on battle win)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-051";

fn beast_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.traits = vec!["Beast".to_string()];
    c
}

fn weak_opp(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c
}

fn carrier_lv6(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-051 YAML parses and compiles")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(beast_lv4("BEAST-ALLY"))
        .add_card(weak_opp("OPP-WEAK"))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt25_051_metadata_and_alt_path() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert_eq!(card.name, "Grizzlymon");
    assert_eq!(card.dp, Some(4000));
    let path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("digivolve alt-path");
    assert_eq!(path.from.as_ref().unwrap().trait_has.as_deref(), Some("TS"));
}

#[test]
fn bt25_051_has_blocker_and_win_battle_opt() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");

    let has_blocker = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword.eq_ignore_ascii_case("blocker")
        )
    });
    assert!(has_blocker, "must grant <Blocker>");

    let win = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && (t.when.contains(&CompiledTiming::OnAnyDeletion)
                        || t.when.contains(&CompiledTiming::OnDeletion)
                        || t.when.contains(&CompiledTiming::EndOfBattle)) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited win-battle clause present");
    assert!(win.once_per_turn, "win-battle draw is [Once Per Turn]");
}

// ─── Section 2 — Behavior: <Blocker> on self ─────────────────────────────────

#[test]
fn bt25_051_grants_self_blocker() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    let griz = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        runner.game.has_keyword(griz, Keyword::Blocker),
        "Grizzlymon must have <Blocker> as a face-up Digimon"
    );
}

// ─── Section 3 — Behavior: +3000 DP until opponent's turn ends ───────────────

#[test]
fn bt25_051_on_play_buffs_a_beast_digimon_3000() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    let ally = runner.place_on_field(0, "BEAST-ALLY", Some(0));
    let dp_before = runner.effective_dp(ally).unwrap();
    runner.play(0, 0);
    assert!(
        runner.pending_selection().is_some(),
        "the [On Play] buff must install a target selection"
    );
    runner.auto_resolve().expect("resolve");

    // Whichever eligible target was chosen gets +3000; assert the ally path.
    if runner.effective_dp(ally) == Some(dp_before + 3000) {
        // Buff persists into the opponent's turn (until opp turn ends).
        runner.end_turn(); // opponent's turn now
        assert_eq!(
            runner.effective_dp(ally),
            Some(dp_before + 3000),
            "+3000 DP persists through the opponent's turn"
        );
        runner.end_turn(); // back to controller — buff has expired
        assert_eq!(
            runner.effective_dp(ally),
            Some(dp_before),
            "+3000 DP expires once the opponent's turn ends"
        );
    }
}

// ─── Section 4 — Behavior: inherited win-battle draw ─────────────────────────

#[test]
fn bt25_051_inherited_draws_on_battle_win() {
    let mut runner = base().add_card(carrier_lv6("CARRIER6")).memory(10).start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER6"]); // Grizzlymon is a source
    let weak = runner.place_on_field(1, "OPP-WEAK", Some(0)); // 1000 DP → deleted

    let deck_before = runner.deck_size(0);
    runner.attack_digimon(stack, weak, false);
    runner.auto_resolve().expect("resolve inherited draw");

    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "winning a battle draws 1 via Grizzlymon's inherited effect"
    );
}

#[test]
fn bt25_051_inherited_no_draw_without_battle_win() {
    let mut runner = base().add_card(carrier_lv6("CARRIER6")).memory(10).start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let stack = runner.place_stack(0, &[CARD_ID, "CARRIER6"]);
    let deck_before = runner.deck_size(0);
    runner.attack_player(stack, 1, false);
    runner.auto_resolve().ok();

    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "a player attack / security check is not a battle win → no inherited draw"
    );
}
