//! ST23-08 Monarchlizamon — Digimon, Lv.5, Green, DP 7000, Cost 7.
//! Traits: Cyborg, Glowing Dawn, BEATBREAK. Attribute: Data.
//!
//! # Card text (data/cards.json — verbatim, confirmed vs DCGO)
//! <Alliance>.
//! [On Play] [When Digivolving] This Digimon gets +3000 DP until your
//!   opponent's turn ends. Then, if it's your turn, by trashing the bottom
//!   face-down card from under any of your Tamers, you may play or use 1
//!   [Glowing Dawn] trait card from your hand with the cost reduced by 3.
//! Inherited: [End of Attack] [Once Per Turn] By trashing the bottom face-down
//!   card from under any of your Tamers, this [Glowing Dawn] Digimon unsuspends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/Green/ST23_08.cs
//!
//! # Patterns this test covers
//! - H10 Alliance (grant) + Glowing Dawn Lv.4 alt-digivolve
//! - mandatory self +3000 DP until opponent turn end (no target pick)
//! - G-PLAY-OR-USE-FROM-HAND: trash-FD-under-Tamer cost → play a [Glowing Dawn]
//!   Digimon at cost-3, use a [Glowing Dawn] Option at cost-3, optional decline.
//! - inherited End-of-Attack OPT trash-FD → unsuspend self
//!
//! # Verdict — IMPLEMENTED

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "ST23-08";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn gd_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 5;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

fn gd_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Green];
    c.level = None;
    c.dp = None;
    c.play_cost = 5;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

fn gd_host(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(6);
    c.dp = Some(9000);
    c.play_cost = 6;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

fn push_to_hand(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card_id {card_id}"));
    let next_idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .hand
        .push(CardSource::new(data_idx, p, next_idx));
}

fn hand_index_of(runner: &DebugRunner, p: u8, card_id: &str) -> usize {
    runner.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not in hand of p{p}"))
}

fn base() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-08 in embedded DSL pack")
        .add_card(gd_digimon("GD-DIGI"))
        .add_card(gd_option("GD-OPT"))
        .add_card(make_tamer("TAMER"))
        .add_card(gd_host("HOST"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(8)
        .start();
    runner.set_first_player(0);
    runner
}

fn tamer_with_face_down(runner: &mut DebugRunner) -> PermanentHandle {
    let tamer = runner.place_stack(0, &["FILLER", "TAMER"]);
    runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down = true;
    tamer
}

/// Drive every pending prompt for the [On Play] resolution, choosing the
/// `target_card` at the `Hand` prompt (the play-or-use pick) and the first
/// legal action for every other prompt (TriggerOrder clause ordering, the
/// outer-optional Replacement accept gate, the +3000 buff, and the Tamer
/// trash-FD pick). Asserts the GD card was actually offered at the Hand prompt.
fn drive_choosing(runner: &mut DebugRunner, target_card: &str) {
    use digimon_engine::selection::SelectionKind;
    let mut guard = 0;
    while let Some(v) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 16, "prompt loop did not terminate");
        let action = if v.kind == SelectionKind::Hand {
            let idx = hand_index_of(runner, 0, target_card);
            let want = digimon_engine::action::space::PLAY_HAND_START + idx as u16;
            assert!(
                v.valid_action_ids.contains(&want),
                "{target_card} must be a legal play-or-use pick (ids={:?}, want={want})",
                v.valid_action_ids
            );
            want
        } else {
            v.valid_action_ids[0]
        };
        runner.execute_action(v.selecting_player, action).unwrap();
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st23_08_metadata_alliance_alt_path_and_main_clause() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Monarchlizamon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));

    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "Alliance"
        )),
        "<Alliance> grant present"
    );
    assert!(
        card.alt_paths.iter().any(|p| p
            .from
            .as_ref()
            .and_then(|f| f.trait_has.as_deref())
            .map(|t| t == "Glowing Dawn")
            .unwrap_or(false)),
        "Lv.4 [Glowing Dawn] alt-path present"
    );
    let op_wd = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Triggered(t)
                    if t.when.contains(&CompiledTiming::OnPlay)
                        && t.when.contains(&CompiledTiming::WhenDigivolving)
                        && t.scope == CompiledScope::FaceUp
            )
        })
        .count();
    assert_eq!(op_wd, 2, "DP-buff clause + play-or-use clause both present");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play] self +3000 DP until opponent turn end (mandatory)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st23_08_on_play_self_plus_3000_dp() {
    let mut runner = base();
    let monarch = runner.place_on_field(0, CARD_ID, Some(0));
    let dp_before = runner.effective_dp(monarch).expect("self dp");

    runner.fire_on_play(0, monarch.index as usize);
    // No Tamer/FD and no GD card → play-or-use half is inert.
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.effective_dp(monarch).expect("self dp"),
        dp_before + 3000,
        "Monarchlizamon gains +3000 DP"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — play-or-use half (G-PLAY-OR-USE-FROM-HAND)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st23_08_plays_glowing_dawn_digimon_at_cost_minus_3() {
    let mut runner = base();
    let _tamer = tamer_with_face_down(&mut runner);
    push_to_hand(&mut runner, 0, "GD-DIGI");

    let monarch = runner.place_on_field(0, CARD_ID, Some(0));
    let trash_before = runner.trash_size(0);
    let mem_before = runner.memory();
    let field_before = runner.game.players[0].battle_area.len();

    runner.fire_on_play(0, monarch.index as usize);

    // TriggerOrder (+3000 vs play-or-use) → outer-optional accept → Tamer pick
    // → hand pick (the GD Digimon). The driver accepts the optional gate.
    drive_choosing(&mut runner, "GD-DIGI");

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "face-down trashed as cost"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "GD-DIGI"),
        "the [Glowing Dawn] Digimon was played"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before + 1,
        "exactly one new permanent entered"
    );
    assert_eq!(runner.memory(), mem_before - 2, "cost 5 - 3 = 2 memory");
}

#[test]
fn st23_08_uses_glowing_dawn_option_at_cost_minus_3() {
    let mut runner = base();
    let _tamer = tamer_with_face_down(&mut runner);
    push_to_hand(&mut runner, 0, "GD-OPT");

    let monarch = runner.place_on_field(0, CARD_ID, Some(0));
    let trash_before = runner.trash_size(0);
    let mem_before = runner.memory();

    runner.fire_on_play(0, monarch.index as usize);

    drive_choosing(&mut runner, "GD-OPT");

    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "the trashed face-down (cost) AND the used Option both went to trash"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "GD-OPT"),
        "the [Glowing Dawn] Option was USED (left hand)"
    );
    assert_eq!(runner.memory(), mem_before - 2, "use cost 5 - 3 = 2 memory");
}

#[test]
fn st23_08_play_or_use_is_declinable() {
    use digimon_engine::selection::SelectionKind;
    let mut runner = base();
    let _tamer = tamer_with_face_down(&mut runner);
    push_to_hand(&mut runner, 0, "GD-DIGI");

    let monarch = runner.place_on_field(0, CARD_ID, Some(0));
    let trash_before = runner.trash_size(0);
    let field_before = runner.game.players[0].battle_area.len();

    runner.fire_on_play(0, monarch.index as usize);

    // Walk prompts; DECLINE the optional Replacement engagement gate (PASS).
    let mut declined = false;
    let mut guard = 0;
    while let Some(v) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 16);
        if v.kind == SelectionKind::Replacement && v.is_optional {
            runner
                .execute_action(v.selecting_player, digimon_engine::action::space::PASS)
                .unwrap();
            declined = true;
        } else {
            runner
                .execute_action(v.selecting_player, v.valid_action_ids[0])
                .unwrap();
        }
    }
    assert!(
        declined,
        "the optional engagement gate was offered and declined"
    );

    assert_eq!(runner.trash_size(0), trash_before, "no trash on decline");
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before,
        "no play on decline"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [End of Attack][OPT]: trash FD → unsuspend self
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st23_08_inherited_unsuspends_glowing_dawn_host() {
    let mut runner = base();
    let host = runner.place_stack(0, &[CARD_ID, "HOST"]);
    runner.game.players[0].battle_area[host.index as usize].is_suspended = true;
    let tamer = runner.place_stack(0, &["FILLER", "TAMER"]);
    runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down = true;

    let trash_before = runner.trash_size(0);
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(host));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "inherited end_of_attack OPT installs a prompt"
    );
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[host.index as usize].is_suspended,
        "the Glowing Dawn host unsuspends after the face-down trash cost"
    );
    assert_eq!(runner.trash_size(0), trash_before + 1, "face-down trashed");
}
