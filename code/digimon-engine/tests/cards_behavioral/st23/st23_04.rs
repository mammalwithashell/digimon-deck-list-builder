//! ST23-04 Murasamemon — Digimon, Lv.5, Yellow, DP 7000, Cost 7.
//! Traits: Beastkin, Glowing Dawn, BEATBREAK. Attribute: Virus.
//!
//! # Card text (data/cards.json — verbatim, confirmed vs DCGO)
//! <Alliance>.
//! [On Play] [When Digivolving] 1 of your opponent's Digimon gets -5000 DP for
//!   the turn. Then, if it's your turn, by trashing the bottom face-down card
//!   from under any of your Tamers, you may play or use 1 [Glowing Dawn] trait
//!   card from your hand with the cost reduced by 3.
//! Inherited: [End of Attack] [Once Per Turn] By trashing the bottom face-down
//!   card from under any of your Tamers, this [Glowing Dawn] Digimon unsuspends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST23/Yellow/ST23_04.cs
//!
//! # Patterns this test covers
//! - H10 Alliance (grant) + Glowing Dawn Lv.4 alt-digivolve
//! - mandatory DP-minus (-5000 for the turn) to an opponent Digimon
//! - G-PLAY-OR-USE-FROM-HAND: by trashing a face-down card under a Tamer (the
//!   pay cost), may PLAY a [Glowing Dawn] Digimon at cost-3, USE a [Glowing
//!   Dawn] Option at cost-3, optional decline, and the no-target negative.
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

const CARD_ID: &str = "ST23-04";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A high-DP opponent Digimon (so -5000 DP never deletes it — rule 17-1-3-1).
fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(5);
    c.dp = Some(9000);
    c.play_cost = 6;
    c
}

/// A [Glowing Dawn] Digimon to PLAY from hand (cost 5 → cost-3 = pay 2).
fn gd_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 5;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// A [Glowing Dawn] Option to USE from hand (use cost 5 → cost-3 = pay 2).
fn gd_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Yellow];
    c.level = None;
    c.dp = None;
    c.play_cost = 5;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// A non-Glowing-Dawn Digimon in hand (must NOT be a play-or-use candidate).
fn plain_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 5;
    c
}

fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Yellow];
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

/// A Glowing Dawn HOST Digimon for the inherited clause.
fn gd_host(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
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
        .expect("ST23-04 in embedded DSL pack")
        .add_card(opp_digimon("OPP"))
        .add_card(gd_digimon("GD-DIGI"))
        .add_card(gd_option("GD-OPT"))
        .add_card(plain_digimon("PLAIN"))
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

/// Place a Tamer for p0 carrying a bottom face-down digivolution source (the
/// pay cost for the play-or-use half). Returns the Tamer's handle.
fn tamer_with_face_down(runner: &mut DebugRunner) -> PermanentHandle {
    let tamer = runner.place_stack(0, &["FILLER", "TAMER"]);
    runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down = true;
    tamer
}

/// Drive every pending prompt for the [On Play] resolution, choosing
/// `target_card` at the `Hand` prompt and the first legal action for every
/// other prompt (TriggerOrder clause ordering, the outer-optional Replacement
/// accept gate, the DP-minus opponent pick, and the Tamer trash-FD pick).
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
fn st23_04_metadata_alliance_alt_path_and_main_clause() {
    let runner = base();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Murasamemon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.cost, Some(7));
    assert_eq!(card.dp, Some(7000));

    // <Alliance> grant.
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, .. })
                if keyword == "Alliance"
        )),
        "<Alliance> grant present"
    );

    // Glowing Dawn Lv.4 alt-digivolve.
    assert!(
        card.alt_paths.iter().any(|p| p
            .from
            .as_ref()
            .and_then(|f| f.trait_has.as_deref())
            .map(|t| t == "Glowing Dawn")
            .unwrap_or(false)),
        "Lv.4 [Glowing Dawn] alt-path present"
    );

    // Two [On Play][When Digivolving] clauses (the DP-minus and the play-or-use).
    let op_wd = card
        .effects
        .iter()
        .filter(|c| matches!(
            c,
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.scope == CompiledScope::FaceUp
        ))
        .count();
    assert_eq!(op_wd, 2, "DP-minus clause + play-or-use clause both present");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play] DP-minus (mandatory)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st23_04_on_play_minus_5000_to_opponent_digimon() {
    let mut runner = base();
    let opp = runner.place_on_field(1, "OPP", Some(0));
    let dp_before = runner.effective_dp(opp).expect("opp dp");

    let murasame = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, murasame.index as usize);

    // First prompt: pick the opponent Digimon for the -5000 DP.
    let view = runner
        .pending_selection_view()
        .expect("DP-minus target prompt must surface");
    assert!(!view.is_optional, "the DP-minus target pick is mandatory");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .unwrap();
    // No Tamer/face-down stash and no GD card in hand → the play-or-use half is
    // inert; resolve any residual prompt.
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.effective_dp(opp).expect("opp dp"),
        dp_before - 5000,
        "opponent Digimon lost 5000 DP for the turn"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — play-or-use half (G-PLAY-OR-USE-FROM-HAND)
// ═══════════════════════════════════════════════════════════════════════════════

/// PLAY a [Glowing Dawn] Digimon from hand at cost-3 by trashing a face-down
/// card under a Tamer. cost 5 → pay 2 memory.
#[test]
fn st23_04_plays_glowing_dawn_digimon_at_cost_minus_3() {
    let mut runner = base();
    let _opp = runner.place_on_field(1, "OPP", Some(0));
    let tamer = tamer_with_face_down(&mut runner);
    push_to_hand(&mut runner, 0, "GD-DIGI");

    let murasame = runner.place_on_field(0, CARD_ID, Some(0));
    let trash_before = runner.trash_size(0);
    let mem_before = runner.memory();
    let field_before = runner.game.players[0].battle_area.len();

    runner.fire_on_play(0, murasame.index as usize);

    // TriggerOrder (DP-minus vs play-or-use) → DP-minus target → outer-optional
    // accept → Tamer trash-FD pick → hand pick (the GD Digimon). The driver
    // accepts the optional gate and picks the named card at the Hand prompt.
    drive_choosing(&mut runner, "GD-DIGI");

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the bottom face-down card under the Tamer was trashed as the pay cost"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "GD-DIGI"),
        "the [Glowing Dawn] Digimon was played to the battle area"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before + 1,
        "exactly one new permanent (the played GD Digimon) entered"
    );
    // cost 5 - 3 = 2 memory paid.
    assert_eq!(
        runner.memory(),
        mem_before - 2,
        "played at the printed cost (5) reduced by 3 → 2 memory"
    );
    let _ = tamer;
}

/// USE a [Glowing Dawn] Option from hand at cost-3. use cost 5 → pay 2 memory.
#[test]
fn st23_04_uses_glowing_dawn_option_at_cost_minus_3() {
    let mut runner = base();
    let _opp = runner.place_on_field(1, "OPP", Some(0));
    let _tamer = tamer_with_face_down(&mut runner);
    push_to_hand(&mut runner, 0, "GD-OPT");

    let murasame = runner.place_on_field(0, CARD_ID, Some(0));
    let trash_before = runner.trash_size(0);
    let mem_before = runner.memory();

    runner.fire_on_play(0, murasame.index as usize);

    // TriggerOrder → DP-minus → accept gate → Tamer pick → hand pick (GD Option).
    drive_choosing(&mut runner, "GD-OPT");

    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "the trashed face-down (pay cost) AND the used Option both went to trash"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "GD-OPT"),
        "the [Glowing Dawn] Option left the hand (it was USED)"
    );
    // use cost 5 - 3 = 2 memory paid.
    assert_eq!(
        runner.memory(),
        mem_before - 2,
        "used at the printed use cost (5) reduced by 3 → 2 memory"
    );
}

/// DECLINE: the player may decline the play-or-use entirely (no trash, no play).
/// The DP-minus (clause A) still resolves; only the optional engagement gate is
/// declined.
#[test]
fn st23_04_play_or_use_is_declinable() {
    use digimon_engine::selection::SelectionKind;
    let mut runner = base();
    let _opp = runner.place_on_field(1, "OPP", Some(0));
    let _tamer = tamer_with_face_down(&mut runner);
    push_to_hand(&mut runner, 0, "GD-DIGI");

    let murasame = runner.place_on_field(0, CARD_ID, Some(0));
    let trash_before = runner.trash_size(0);
    let field_before = runner.game.players[0].battle_area.len();

    runner.fire_on_play(0, murasame.index as usize);

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
            runner.execute_action(v.selecting_player, v.valid_action_ids[0]).unwrap();
        }
    }
    assert!(declined, "the optional engagement gate was offered and declined");

    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "declining means NO face-down card was trashed"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before,
        "declining means NO GD card was played"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "GD-DIGI"),
        "the GD Digimon is still in hand"
    );
}

/// NEGATIVE: no [Glowing Dawn] card in hand → even after paying the trash cost,
/// nothing is played. A non-GD hand card is never a legal pick.
#[test]
fn st23_04_no_glowing_dawn_card_in_hand_no_play() {
    let mut runner = base();
    let _opp = runner.place_on_field(1, "OPP", Some(0));
    let _tamer = tamer_with_face_down(&mut runner);
    push_to_hand(&mut runner, 0, "PLAIN"); // non-Glowing-Dawn

    let murasame = runner.place_on_field(0, CARD_ID, Some(0));
    let field_before = runner.game.players[0].battle_area.len();

    runner.fire_on_play(0, murasame.index as usize);

    // Accept every prompt (DP-minus, engagement gate, Tamer trash-FD). With no
    // GD card in hand, `select_hand` finds no candidate → no Hand prompt → the
    // play-or-use is inert. `auto_resolve` accepts every prompt by first id.
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "PLAIN"),
        "the non-Glowing-Dawn card stays in hand (never a legal pick)"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        field_before,
        "no GD card available → nothing played"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [End of Attack][OPT]: trash FD → unsuspend self
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn st23_04_inherited_unsuspends_glowing_dawn_host() {
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
        "the inherited end_of_attack OPT must install a prompt"
    );
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[host.index as usize].is_suspended,
        "the Glowing Dawn host unsuspends after paying the face-down trash cost"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "the bottom face-down source was trashed as the cost"
    );
}
