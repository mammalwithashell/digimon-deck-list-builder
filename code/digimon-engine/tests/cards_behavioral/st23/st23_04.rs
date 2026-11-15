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
//! - CR 4.3 §§15-1-2, 15-1-4, 17-1-2-2: the printed OP/WD effect
//!   processes in text order and defers the 0-DP rule check until its tail ends.
//!
//! # Verdict — IMPLEMENTED

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::{
    build_action_mask,
    space::{encode_attack, PASS, PLAY_HAND_START},
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "ST23-04";

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// A high-DP opponent Digimon (so -5000 DP does not reach the 0-DP rule check).
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
/// other prompt (the DP-minus opponent pick and the optional Tamer trash pick).
fn drive_choosing(runner: &mut DebugRunner, target_card: &str) {
    use digimon_engine::selection::SelectionKind;
    let mut guard = 0;
    while let Some(v) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 16, "prompt loop did not terminate");
        assert_ne!(
            v.kind,
            SelectionKind::TriggerOrder,
            "one printed effect cannot be reordered internally"
        );
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

    // One printed effect contains both the mandatory DP reduction and its tail.
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
    assert_eq!(
        op_wd, 1,
        "DP-minus and the Then tail must share one effect boundary"
    );
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

    // DP-minus target → optional Tamer trash pick → optional GD hand pick.
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

    // DP-minus target → optional Tamer trash pick → optional GD Option pick.
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
/// The mandatory DP reduction still resolves; decline only the Tamer payment.
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

    // Walk prompts; DECLINE the optional Tamer payment (PASS).
    let mut declined = false;
    let mut guard = 0;
    while let Some(v) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 16);
        assert_ne!(
            v.kind,
            SelectionKind::TriggerOrder,
            "the Then tail is not independently orderable"
        );
        if v.kind == SelectionKind::OwnField && v.is_optional {
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
        "the optional Tamer payment was offered and declined"
    );

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

    // Accept every prompt (DP-minus and Tamer trash-FD). With no
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
// Section 4 — One-effect ordering, rule-check boundary, and optional-tail gates
// ═══════════════════════════════════════════════════════════════════════════════

fn fire_shared(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn assert_prompt(runner: &DebugRunner, kind: SelectionKind, ids: &[u16], optional: bool) {
    let view = runner
        .pending_selection_view()
        .expect("player-visible choice");
    assert_eq!(view.kind, kind);
    assert_eq!(view.selecting_player, 0);
    assert_eq!(view.is_optional, optional);
    assert_eq!(view.valid_action_ids, ids);
    for (action, enabled) in build_action_mask(&runner.game, 0).iter().enumerate() {
        assert_eq!(
            *enabled,
            if ids.contains(&(action as u16)) || (optional && action == PASS as usize) {
                1.0
            } else {
                0.0
            },
            "unexpected mask bit {action} for {kind:?}"
        );
    }
    assert!(build_action_mask(&runner.game, 1)
        .iter()
        .all(|value| *value == 0.0));
}

#[test]
fn st23_04_shared_effect_starts_with_mandatory_dp_choice_without_trigger_order() {
    for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        let mut runner = base();
        let first = runner.place_on_field(1, "OPP", Some(0));
        let second = runner.place_on_field(1, "FILLER", Some(0));
        tamer_with_face_down(&mut runner);
        push_to_hand(&mut runner, 0, "GD-DIGI");
        let source = runner.place_on_field(0, CARD_ID, Some(0));

        fire_shared(&mut runner, timing, source);

        assert_prompt(
            &runner,
            SelectionKind::OppField,
            &[
                encode_attack(0, first.index as u16),
                encode_attack(0, second.index as u16),
            ],
            false,
        );
    }
}

#[test]
fn st23_04_zero_dp_target_survives_tamer_and_hand_choices_until_whole_effect_ends() {
    for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        for payload in [Some("GD-DIGI"), Some("GD-OPT"), None] {
            let mut runner = base();
            let other = runner.place_on_field(1, "OPP", Some(0));
            let doomed = runner.place_on_field(1, "FILLER", Some(0));
            let first_tamer = tamer_with_face_down(&mut runner);
            let paid_tamer = tamer_with_face_down(&mut runner);
            let face_up_tamer = runner.place_stack(0, &["FILLER", "TAMER"]);
            push_to_hand(&mut runner, 0, "PLAIN");
            push_to_hand(&mut runner, 0, "GD-DIGI");
            push_to_hand(&mut runner, 0, "GD-OPT");
            let source = runner.place_on_field(0, CARD_ID, Some(0));
            let memory_before = runner.memory();
            let trash_before = runner.trash_size(0);

            fire_shared(&mut runner, timing, source);
            // Admit the old authoring's DP-first ordering only here, so this
            // regression independently exposes its premature rule check.
            // The separate test above forbids that TriggerOrder altogether.
            if runner.pending_kind() == Some(SelectionKind::TriggerOrder) {
                let first = runner.pending_selection_view().unwrap().valid_action_ids[0];
                runner.execute_action(0, first).unwrap();
            }
            assert_prompt(
                &runner,
                SelectionKind::OppField,
                &[
                    encode_attack(0, other.index as u16),
                    encode_attack(0, doomed.index as u16),
                ],
                false,
            );
            runner
                .execute_action(0, encode_attack(0, doomed.index as u16))
                .unwrap();

            assert_eq!(
                runner.game.players[1].battle_area.len(),
                2,
                "CR 17-1-2-2: no 0-DP deletion before the Tamer decision"
            );
            assert!(runner.effective_dp(doomed).unwrap() <= 0);
            assert_eq!(runner.effective_dp(other), Some(9000));
            assert_eq!(runner.trash_size(1), 0);
            assert_prompt(
                &runner,
                SelectionKind::OwnField,
                &[
                    encode_attack(0, first_tamer.index as u16),
                    encode_attack(0, paid_tamer.index as u16),
                ],
                true,
            );
            runner
                .execute_action(0, encode_attack(0, paid_tamer.index as u16))
                .unwrap();

            assert_eq!(
                runner.game.players[1].battle_area.len(),
                2,
                "CR 17-1-2-2: no 0-DP deletion before the hand decision"
            );
            assert!(runner.effective_dp(doomed).unwrap() <= 0);
            assert_eq!(runner.trash_size(1), 0);
            assert_eq!(runner.trash_size(0), trash_before + 1);
            assert_eq!(
                runner.game.players[0].battle_area[first_tamer.index as usize]
                    .card_sources
                    .len(),
                2
            );
            assert_eq!(
                runner.game.players[0].battle_area[paid_tamer.index as usize]
                    .card_sources
                    .len(),
                1
            );
            assert_eq!(
                runner.game.players[0].battle_area[face_up_tamer.index as usize]
                    .card_sources
                    .len(),
                2
            );
            let digimon = PLAY_HAND_START + hand_index_of(&runner, 0, "GD-DIGI") as u16;
            let option = PLAY_HAND_START + hand_index_of(&runner, 0, "GD-OPT") as u16;
            assert_prompt(&runner, SelectionKind::Hand, &[digimon, option], true);
            let action = match payload {
                Some("GD-DIGI") => digimon,
                Some("GD-OPT") => option,
                None => PASS,
                _ => unreachable!(),
            };
            runner.execute_action(0, action).unwrap();

            assert!(runner.pending_selection().is_none());
            assert_eq!(
                runner.game.players[1].battle_area.len(),
                1,
                "the rule check deletes the 0-DP Digimon after the whole effect"
            );
            assert_eq!(
                runner.game.players[1].battle_area[0]
                    .top_card()
                    .card_id(&runner.game.card_data),
                "OPP"
            );
            assert_eq!(runner.trash_size(1), 1);
            assert_eq!(
                runner.memory(),
                memory_before - if payload.is_some() { 2 } else { 0 }
            );
            assert_eq!(
                runner.trash_size(0),
                trash_before + if payload == Some("GD-OPT") { 2 } else { 1 }
            );
            assert_eq!(
                runner.game.players[0]
                    .battle_area
                    .iter()
                    .filter(|p| p.top_card().card_id(&runner.game.card_data) == "GD-DIGI")
                    .count(),
                usize::from(payload == Some("GD-DIGI"))
            );
            assert!(runner.game.players[0]
                .hand
                .iter()
                .any(|c| c.card_id(&runner.game.card_data) == "PLAIN"));
        }
    }
}

#[test]
fn st23_04_declining_tamer_payment_finishes_dp_effect_without_spending_source() {
    for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        let mut runner = base();
        let doomed = runner.place_on_field(1, "FILLER", Some(0));
        let tamer = tamer_with_face_down(&mut runner);
        push_to_hand(&mut runner, 0, "GD-DIGI");
        let source = runner.place_on_field(0, CARD_ID, Some(0));
        let memory_before = runner.memory();
        fire_shared(&mut runner, timing, source);
        assert_prompt(
            &runner,
            SelectionKind::OppField,
            &[encode_attack(0, doomed.index as u16)],
            false,
        );
        runner
            .execute_action(0, encode_attack(0, doomed.index as u16))
            .unwrap();
        assert_eq!(runner.game.players[1].battle_area.len(), 1);
        assert_prompt(
            &runner,
            SelectionKind::OwnField,
            &[encode_attack(0, tamer.index as u16)],
            true,
        );
        runner.execute_action(0, PASS).unwrap();

        assert!(runner.pending_selection().is_none());
        assert!(runner.game.players[1].battle_area.is_empty());
        assert_eq!(runner.trash_size(1), 1);
        assert_eq!(
            runner.game.players[0].battle_area[tamer.index as usize]
                .card_sources
                .len(),
            2
        );
        assert_eq!(runner.trash_size(0), 0);
        assert_eq!(runner.memory(), memory_before);
        assert!(runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "GD-DIGI"));
    }
}

#[test]
fn st23_04_opponents_turn_applies_dp_but_does_not_offer_payment_or_payload() {
    for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        let mut runner = base();
        runner.set_first_player(1);
        let opp = runner.place_on_field(1, "OPP", Some(0));
        let tamer = tamer_with_face_down(&mut runner);
        push_to_hand(&mut runner, 0, "GD-DIGI");
        let source = runner.place_on_field(0, CARD_ID, Some(0));
        let memory_before = runner.memory();
        fire_shared(&mut runner, timing, source);
        assert_prompt(
            &runner,
            SelectionKind::OppField,
            &[encode_attack(0, opp.index as u16)],
            false,
        );
        runner
            .execute_action(0, encode_attack(0, opp.index as u16))
            .unwrap();

        assert_eq!(runner.effective_dp(opp), Some(4000));
        assert!(runner.pending_selection().is_none());
        assert_eq!(
            runner.game.players[0].battle_area[tamer.index as usize]
                .card_sources
                .len(),
            2
        );
        assert_eq!(runner.memory(), memory_before);
        assert_eq!(runner.trash_size(0), 0);
        assert!(runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "GD-DIGI"));
    }
}

#[test]
fn st23_04_no_payable_tamer_still_applies_dp_without_free_payload() {
    for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        for has_face_up_tamer in [false, true] {
            let mut runner = base();
            let opp = runner.place_on_field(1, "OPP", Some(0));
            if has_face_up_tamer {
                runner.place_stack(0, &["FILLER", "TAMER"]);
            }
            // A face-down card under a Digimon does not pay the Tamer cost.
            let wrong_host = runner.place_stack(0, &["FILLER", "HOST"]);
            runner.game.players[0].battle_area[wrong_host.index as usize].card_sources[0]
                .face_down = true;
            push_to_hand(&mut runner, 0, "GD-DIGI");
            let source = runner.place_on_field(0, CARD_ID, Some(0));
            let memory_before = runner.memory();
            fire_shared(&mut runner, timing, source);
            assert_prompt(
                &runner,
                SelectionKind::OppField,
                &[encode_attack(0, opp.index as u16)],
                false,
            );
            runner
                .execute_action(0, encode_attack(0, opp.index as u16))
                .unwrap();

            assert_eq!(runner.effective_dp(opp), Some(4000));
            assert!(
                runner.pending_selection().is_none(),
                "unpayable cost must not offer a hand pick"
            );
            assert_eq!(runner.memory(), memory_before);
            assert_eq!(runner.trash_size(0), 0);
            assert!(runner.game.players[0]
                .hand
                .iter()
                .any(|c| c.card_id(&runner.game.card_data) == "GD-DIGI"));
        }
    }
}

#[test]
fn st23_04_without_opponent_digimon_still_offers_then_payment_and_payload() {
    for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        let mut runner = base();
        runner.place_on_field(1, "TAMER", Some(0));
        let tamer = tamer_with_face_down(&mut runner);
        push_to_hand(&mut runner, 0, "GD-DIGI");
        let source = runner.place_on_field(0, CARD_ID, Some(0));
        let memory_before = runner.memory();
        fire_shared(&mut runner, timing, source);
        assert_prompt(
            &runner,
            SelectionKind::OwnField,
            &[encode_attack(0, tamer.index as u16)],
            true,
        );
        runner
            .execute_action(0, encode_attack(0, tamer.index as u16))
            .unwrap();
        let hand_pick = PLAY_HAND_START + hand_index_of(&runner, 0, "GD-DIGI") as u16;
        assert_prompt(&runner, SelectionKind::Hand, &[hand_pick], true);
        runner.execute_action(0, hand_pick).unwrap();

        assert!(runner.pending_selection().is_none());
        assert_eq!(runner.memory(), memory_before - 2);
        assert_eq!(runner.trash_size(0), 1);
        assert!(runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "GD-DIGI"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited [End of Attack][OPT]: trash FD → unsuspend self
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

/// CR 4.3 §15-7-5: the cost can be paid even when the non-Glowing-Dawn
/// carrier cannot receive the conditional unsuspend payload.
#[test]
fn st23_04_inherited_non_glowing_dawn_host_can_pay_or_decline_cost() {
    for decline in [false, true] {
        let mut host_data = gd_host("PLAIN-HOST");
        host_data.traits.clear();
        let mut runner = DebugRunner::builder()
            .dsl_card(CARD_ID)
            .unwrap()
            .add_card(host_data)
            .add_card(make_tamer("TAMER"))
            .add_card(filler("FILLER"))
            .deck(0, &["FILLER"; 8])
            .deck(1, &["FILLER"; 8])
            .memory(8)
            .start();
        runner.set_first_player(0);
        let host = runner.place_stack(0, &[CARD_ID, "PLAIN-HOST"]);
        runner.game.players[0].battle_area[host.index as usize].is_suspended = true;
        let first = runner.place_stack(0, &["FILLER", "TAMER"]);
        let second = runner.place_stack(0, &["FILLER", "TAMER"]);
        for tamer in [first, second] {
            runner.game.players[0].battle_area[tamer.index as usize].card_sources[0].face_down =
                true;
        }
        fire_shared(&mut runner, EffectTiming::EndOfAttack, host);
        let view = runner
            .pending_selection_view()
            .expect("payment remains optional on a non-GD host");
        assert_eq!(view.kind, SelectionKind::Replacement);
        assert_eq!(view.selecting_player, 0);
        assert!(view.is_optional);
        let accept = digimon_engine::action::space::REPLACEMENT_ACCEPT;
        for (action, legal) in build_action_mask(&runner.game, 0).iter().enumerate() {
            assert_eq!(
                *legal,
                if [accept, PASS].contains(&(action as u16)) {
                    1.0
                } else {
                    0.0
                }
            );
        }
        if decline {
            runner.execute_action(0, PASS).unwrap();
            assert!(runner.game.pending_selection.is_none());
            assert_eq!(runner.trash_size(0), 0);
            fire_shared(&mut runner, EffectTiming::EndOfAttack, host);
            assert!(
                runner.game.pending_selection.is_some(),
                "decline preserves the per-turn use"
            );
        }
        runner.execute_action(0, accept).unwrap();
        let view = runner
            .pending_selection_view()
            .expect("choose the payment Tamer");
        assert_eq!(view.kind, SelectionKind::OwnField);
        assert_eq!(view.valid_action_ids, vec![101, 102]);
        assert!(!view.is_optional);
        for (action, legal) in build_action_mask(&runner.game, 0).iter().enumerate() {
            assert_eq!(
                *legal,
                if [101, 102].contains(&(action as u16)) {
                    1.0
                } else {
                    0.0
                }
            );
        }
        runner.execute_action(0, 102).unwrap();
        assert_eq!(runner.trash_size(0), 1);
        assert_eq!(
            runner.game.players[0].battle_area[first.index as usize]
                .card_sources
                .len(),
            2
        );
        assert_eq!(
            runner.game.players[0].battle_area[second.index as usize]
                .card_sources
                .len(),
            1
        );
        assert!(
            runner.game.players[0].battle_area[host.index as usize].is_suspended,
            "the non-GD host pays but does not unsuspend"
        );
        assert!(runner.game.pending_selection.is_none());
        fire_shared(&mut runner, EffectTiming::EndOfAttack, host);
        assert!(
            runner.game.pending_selection.is_none(),
            "paid activation consumes OPT even with no payload"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — hand-pick legality: only cards that can actually be played/used
// ═══════════════════════════════════════════════════════════════════════════════
//
// DCGO ST23_04.cs:128-134 CanSelectCardCondition = `EqualsTraits("Glowing
// Dawn") && ((IsOption && !CanNotPlayThisOption) || (HasPlayCost &&
// CanPlayAsNewPermanent(cardSource, true, activateClass, fixedCost:
// GetCostItself - 3)))`, the SelectHandEffect filter (:144):
// - a Digi-Egg has no play cost (CR 4.3 7-1-1) — e.g. Kekkomon (ST23-01);
// - a Digimon needs a free frame, no effect-play lock, and memory for the
//   reduced cost (CanPlayCardTargetFrame: `Owner.MaxMemoryCost < cost` refuses);
//   the frame cap is the engine's `rules.field_slots`, mirroring DCGO's frame
//   limit (CR 4.3 3-4-8-2 itself sets no battle-area limit);
// - an Option needs no "can't use Options" lock and its colour requirement met
//   (CanNotPlayThisOption, CardSource.cs:184-246; CR 4.3 4-22-1..3).
// DCGO does not check memory for the Option half; the rules do: CR 4.3
// 1-3-11-1 (a card can't be declared for use if its cost can't be paid),
// 4-2-2 + 1-4-2-2 (payment moves the gauge at most to 10 on the opponent's
// side), 9-1-8. So an unaffordable Option is not offered either. A DUAL is used
// (Option information, CR 4.3 4-6-2) and priced at its use cost.

fn gd_card(id: &str, kind: CardKind, color: CardColor, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = kind;
    c.colors = vec![color];
    if kind == CardKind::Digimon {
        c.level = Some(4);
        c.dp = Some(5000);
    } else {
        c.level = None;
        c.dp = None;
    }
    c.play_cost = cost;
    c.traits = vec!["Glowing Dawn".to_string()];
    c
}

/// Murasamemon (ST23-04) on the field with a payable Tamer and a high-DP
/// opponent Digimon; `hand` pushed to p0's hand. Includes the real Kekkomon
/// (ST23-01, [Glowing Dawn] Digi-Egg) and Habakirimon (BT25-043, [Glowing Dawn]
/// DUAL, use cost 6).
fn legality_runner(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-04 in embedded DSL pack")
        .dsl_card("ST23-01")
        .expect("Kekkomon in embedded DSL pack")
        .dsl_card("BT25-043")
        .expect("Habakirimon in embedded DSL pack")
        .add_card(opp_digimon("OPP"))
        .add_card(gd_digimon("GD-DIGI"))
        .add_card(gd_option("GD-OPT"))
        .add_card(gd_card("GD-BIG", CardKind::Digimon, CardColor::Yellow, 14))
        .add_card(gd_card("GD-OPT-BIG", CardKind::Option, CardColor::Yellow, 14))
        .add_card(gd_card("GD-OPT-RED", CardKind::Option, CardColor::Red, 3))
        .add_card(make_tamer("TAMER"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(8)
        .start();
    runner.set_first_player(0);
    runner.place_on_field(1, "OPP", Some(0));
    tamer_with_face_down(&mut runner);
    for h in hand {
        push_to_hand(&mut runner, 0, h);
    }
    let source = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, source)
}

/// Fire [On Play], take the DP target and pay the Tamer trash, and return the
/// sorted card ids offered at the [Glowing Dawn] hand pick (empty when no hand
/// pick installs).
fn offered_after_paying(runner: &mut DebugRunner, source: PermanentHandle) -> Vec<String> {
    use digimon_engine::action::space::HAND_EFFECT_START;
    fire_shared(runner, EffectTiming::OnPlay, source);
    for _ in 0..8 {
        let Some(v) = runner.pending_selection_view() else {
            return Vec::new();
        };
        if v.kind == SelectionKind::Hand {
            let hand: Vec<String> = runner.game.players[0]
                .hand
                .iter()
                .map(|c| c.card_id(&runner.game.card_data).to_string())
                .collect();
            let mut out: Vec<String> = hand
                .iter()
                .enumerate()
                .filter(|(i, _)| {
                    let i = *i as u16;
                    v.valid_action_ids.contains(&(PLAY_HAND_START + i))
                        || v.valid_action_ids.contains(&(HAND_EFFECT_START + i))
                })
                .map(|(_, id)| id.clone())
                .collect();
            out.sort();
            return out;
        }
        let action = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("a non-decline action (DP target / Tamer payment)");
        runner.execute_action(v.selecting_player, action).unwrap();
    }
    panic!("hand pick never reached");
}

fn add_player_lock(runner: &mut DebugRunner, modifier: digimon_engine::enums::ModifierType) {
    use digimon_engine::enums::Expiry;
    use digimon_engine::modifiers::PlayerModifierEntry;
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(modifier, 0, Expiry::Permanent, None, 1),
    );
}

/// Regression guard: a playable Digimon and a usable Option are both offered.
#[test]
fn st23_04_hand_pick_offers_playable_digimon_and_usable_option() {
    let (mut runner, source) = legality_runner(&["GD-DIGI", "GD-OPT"]);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-DIGI", "GD-OPT"]);
}

/// Kekkomon (ST23-01) is a [Glowing Dawn] Digi-Egg with no play cost: never
/// offered (CR 4.3 7-1-1).
#[test]
fn st23_04_hand_pick_excludes_glowing_dawn_digi_egg() {
    let (mut runner, source) = legality_runner(&["ST23-01", "GD-DIGI"]);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-DIGI"]);
}

/// "You can't play Digimon by effects": the Digimon is not offered, the Option is.
#[test]
fn st23_04_hand_pick_excludes_digimon_under_effect_play_prohibition() {
    let (mut runner, source) = legality_runner(&["GD-DIGI", "GD-OPT"]);
    add_player_lock(&mut runner, digimon_engine::enums::ModifierType::CannotPlayDigimonByEffect);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT"]);
}

/// No free frame: the Digimon is not offered, the Option is.
/// The frame cap is the engine's `rules.field_slots` (mirroring DCGO's frame
/// limit); CR 4.3 3-4-8-2 itself sets no battle-area limit.
#[test]
fn st23_04_hand_pick_excludes_digimon_when_battle_area_full() {
    let (mut runner, source) = legality_runner(&["GD-DIGI", "GD-OPT"]);
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT"]);
}

/// "You can't use Option cards": the Option is not offered, the Digimon is.
#[test]
fn st23_04_hand_pick_excludes_option_when_options_cannot_be_used() {
    let (mut runner, source) = legality_runner(&["GD-DIGI", "GD-OPT"]);
    add_player_lock(&mut runner, digimon_engine::enums::ModifierType::CannotUseOptionCards);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-DIGI"]);
}

/// A red [Glowing Dawn] Option with no `<Use Req.>` while only yellow cards are
/// on p0's field: its colour requirement is unmet, so it can't be used.
#[test]
fn st23_04_hand_pick_excludes_option_whose_colour_requirement_is_unmet() {
    let (mut runner, source) = legality_runner(&["GD-OPT-RED", "GD-OPT"]);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT"]);
}

/// Memory 0: a cost-14 Digimon would cost 11 after the reduction, past the
/// 10-memory limit on the opponent's side, so it can't be played (DCGO
/// `MaxMemoryCost`); the cost-5 one (pay 2) still is.
#[test]
fn st23_04_hand_pick_excludes_unaffordable_digimon() {
    let (mut runner, source) = legality_runner(&["GD-BIG", "GD-DIGI"]);
    runner.game.memory = 0;
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-DIGI"]);
}

/// Memory 0: a use-cost-14 Option (pay 11) can't be used. DCGO offers it (no
/// memory check on the Option half); CR 4.3 1-3-11-1 / 9-1-8 do not.
#[test]
fn st23_04_hand_pick_excludes_unaffordable_option() {
    let (mut runner, source) = legality_runner(&["GD-OPT-BIG", "GD-OPT"]);
    runner.game.memory = 0;
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT"]);
}

/// Habakirimon (BT25-043), a [Glowing Dawn] DUAL, is used, so it is priced at
/// its use cost 6 (pay 3 after the reduction), affordable at memory 0.
#[test]
fn st23_04_hand_pick_offers_dual_priced_at_its_use_cost() {
    let (mut runner, source) = legality_runner(&["BT25-043"]);
    runner.game.memory = 0;
    assert_eq!(offered_after_paying(&mut runner, source), vec!["BT25-043"]);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 7 — G-DSL-AFFORDABLE-WITH-COST-REDUCER-CREDIT (reducer credit)
// ═══════════════════════════════════════════════════════════════════════════════
//
// CR 4.3 1-3-11-1..3: a card can be declared for play / use only if its
// (alternate) cost can be paid, counting "when this card would be played,
// reduce the cost" effects whose processing conditions are met (1-3-11-2) --
// optional ones included, which the player must then perform as far as needed
// to pay (1-3-11-3). DCGO prices a play the same way: `CanPlayAsNewPermanent(
// card, true, ..., fixedCost)` -> `PayingCost(checkAvailability: true)`
// (CardSource.cs:1150-1162, 668-686, 925-986) applies every static cost change,
// including the hidden `isCheckAvailability: () => true` ChangeCostClass an
// optional "by trashing a face-down card, reduce the cost" reducer keeps while
// that cost is payable (Kyo Sawashiro BT25_088.cs:246-275, Armalizamon
// BT25_049.cs:226-255), and the reducer itself refuses to be declined while it
// is needed (`CanNoSelect`, BT25_088.cs:155-171).

/// Murasamemon (ST23-04) on the field, a Tamer with ONE face-down card (the
/// cost this effect pays), a high-DP opponent Digimon, `hand` in p0's hand and
/// memory 0. Registers the real Kyo Sawashiro (BT25-088) and Armalizamon
/// (BT25-049); neither is on the field until a test places it.
fn reducer_runner(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-04 in embedded DSL pack")
        .dsl_card("BT25-088")
        .expect("Kyo Sawashiro in embedded DSL pack")
        .dsl_card("BT25-049")
        .expect("Armalizamon in embedded DSL pack")
        .dsl_card("BT25-090")
        .expect("Tomoro Tenma in embedded DSL pack")
        .add_card(opp_digimon("OPP"))
        .add_card(gd_card("GD-14", CardKind::Digimon, CardColor::Yellow, 14))
        .add_card(gd_card("GD-OPT-13", CardKind::Option, CardColor::Yellow, 13))
        .add_card(gd_card("GD-OPT-14", CardKind::Option, CardColor::Yellow, 14))
        .add_card(gd_card("GD-OPT-16", CardKind::Option, CardColor::Yellow, 16))
        .add_card(gd_card("GD-OPT-17", CardKind::Option, CardColor::Yellow, 17))
        .add_card(make_tamer("TAMER"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(8)
        .start();
    runner.set_first_player(0);
    runner.place_on_field(1, "OPP", Some(0));
    tamer_with_face_down(&mut runner);
    for h in hand {
        push_to_hand(&mut runner, 0, h);
    }
    let source = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 0;
    (runner, source)
}

/// Kyo Sawashiro (BT25-088) on p0's field with `n` face-down cards under it.
fn kyo_with_face_down(runner: &mut DebugRunner, n: usize) -> PermanentHandle {
    let mut ids: Vec<&str> = vec!["FILLER"; n];
    ids.push("BT25-088");
    let kyo = runner.place_stack(0, &ids);
    for s in runner.game.players[0].battle_area[kyo.index as usize].card_sources[..n].iter_mut() {
        s.face_down = true;
    }
    kyo
}

/// p0's memory, whichever player's turn it now is.
fn p0_memory(runner: &DebugRunner) -> i16 {
    if runner.game.turn_player() == 0 {
        runner.memory()
    } else {
        -runner.memory()
    }
}

/// Pick `card` at the pending hand pick, then take the first non-decline
/// action at every later prompt until `card` leaves the hand and nothing is
/// pending.
fn pick_and_settle(runner: &mut DebugRunner, card: &str) {
    let v = runner.pending_selection_view().expect("hand pick");
    let want = PLAY_HAND_START + hand_index_of(runner, 0, card) as u16;
    assert!(v.valid_action_ids.contains(&want), "{card} must be offered: {:?}", v.valid_action_ids);
    runner.execute_action(0, want).unwrap();
    for _ in 0..12 {
        let Some(v) = runner.pending_selection_view() else {
            return;
        };
        let action = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .unwrap_or(v.valid_action_ids[0]);
        runner.execute_action(v.selecting_player, action).unwrap();
    }
    panic!("prompts did not settle");
}

fn on_field(runner: &DebugRunner, card: &str) -> bool {
    runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == card)
}

fn in_trash(runner: &DebugRunner, card: &str) -> bool {
    runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == card)
}

/// Kyo Sawashiro (BT25-088) with a face-down card left after Murasamemon's own
/// cost: a cost-14 [Glowing Dawn] Digimon (14 - 3 = 11 from memory 0, one past
/// the 10 limit) is payable at 10 with Kyo's -1, so it is offered (DCGO: the
/// hidden availability -1, BT25_088.cs:246-275).
#[test]
fn st23_04_hand_pick_credits_kyo_sawashiro_play_reducer() {
    let (mut runner, source) = reducer_runner(&["GD-14"]);
    kyo_with_face_down(&mut runner, 2);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-14"]);
}

/// Kyo with no face-down card under any Tamer once Murasamemon has paid can't
/// pay its reducer (BT25_088.cs:258 `TamerWithOneFaceDownSource`), so nothing
/// is credited and the cost-14 Digimon is not offered.
#[test]
fn st23_04_hand_pick_does_not_credit_kyo_without_a_face_down_card() {
    let (mut runner, source) = reducer_runner(&["GD-14"]);
    kyo_with_face_down(&mut runner, 0);
    assert!(offered_after_paying(&mut runner, source).is_empty());
}

/// The credited pick resolves: Kyo's reducer is paid and the Digimon is
/// played for 14 - 3 - 1 = 10.
#[test]
fn st23_04_kyo_credited_pick_plays_for_the_credited_cost() {
    let (mut runner, source) = reducer_runner(&["GD-14"]);
    kyo_with_face_down(&mut runner, 2);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-14"]);
    pick_and_settle(&mut runner, "GD-14");
    assert!(on_field(&runner, "GD-14"), "GD-14 was played");
    assert_eq!(p0_memory(&runner), -10, "0 - (14 - 3 - 1) = -10");
}

/// CR 4.3 1-3-11-3: once GD-14 is declared, Kyo's reducer is needed to pay, so
/// the player must perform it -- its prompt carries no decline (DCGO
/// `CanNoSelect` false, BT25_088.cs:155-171).
#[test]
fn st23_04_needed_kyo_reducer_cannot_be_declined() {
    let (mut runner, source) = reducer_runner(&["GD-14"]);
    kyo_with_face_down(&mut runner, 2);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-14"]);
    let want = PLAY_HAND_START + hand_index_of(&runner, 0, "GD-14") as u16;
    runner.execute_action(0, want).unwrap();
    let v = runner.pending_selection_view().expect("Kyo's reducer prompt");
    assert!(!v.is_optional, "a needed reducer can't be declined: {} {:?}", v.prompt, v.valid_action_ids);
}

/// A reducer that ISN'T needed stays optional: GD-14 at memory 5 (14 - 3 = 11
/// -> -6) is payable without Kyo, so Kyo's prompt may be declined and the play
/// still goes through at 11.
#[test]
fn st23_04_unneeded_kyo_reducer_stays_declinable() {
    let (mut runner, source) = reducer_runner(&["GD-14"]);
    kyo_with_face_down(&mut runner, 2);
    runner.game.memory = 5;
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-14"]);
    let want = PLAY_HAND_START + hand_index_of(&runner, 0, "GD-14") as u16;
    runner.execute_action(0, want).unwrap();
    let v = runner.pending_selection_view().expect("Kyo's reducer prompt");
    assert!(v.is_optional, "an unneeded reducer stays optional: {} {:?}", v.prompt, v.valid_action_ids);
    runner.execute_action(0, PASS).unwrap();
    assert!(on_field(&runner, "GD-14"), "GD-14 was played without Kyo's reducer");
    assert_eq!(p0_memory(&runner), -6, "5 - (14 - 3) = -6");
}

/// Armalizamon (BT25-049) on the field and a Tamer face-down card left: a
/// use-cost-16 [Glowing Dawn] Option (16 - 3 = 13 from memory 0) is payable at
/// 10 with Armalizamon's -3, so it is offered.
#[test]
fn st23_04_hand_pick_credits_armalizamon_option_reducer() {
    let (mut runner, source) = reducer_runner(&["GD-OPT-16"]);
    runner.place_on_field(0, "BT25-049", Some(0));
    tamer_with_face_down(&mut runner);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT-16"]);
}

/// The credited Option use resolves: Armalizamon's reducer is paid and the
/// Option is used for 16 - 3 - 3 = 10.
#[test]
fn st23_04_armalizamon_credited_option_use_pays_the_credited_cost() {
    let (mut runner, source) = reducer_runner(&["GD-OPT-16"]);
    runner.place_on_field(0, "BT25-049", Some(0));
    tamer_with_face_down(&mut runner);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT-16"]);
    pick_and_settle(&mut runner, "GD-OPT-16");
    assert!(in_trash(&runner, "GD-OPT-16"), "GD-OPT-16 was used");
    assert_eq!(p0_memory(&runner), -10, "0 - (16 - 3 - 3) = -10");
}

/// An Option payable only at the effect's reduced cost (13 - 3 = 10 from
/// memory 0) is used: the use is judged on the cost it pays, not on its
/// unreduced use cost (CR 4.3 1-3-11-1, 4-2-3-1).
#[test]
fn st23_04_option_payable_only_after_the_reduction_is_used() {
    let (mut runner, source) = reducer_runner(&["GD-OPT-13"]);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT-13"]);
    pick_and_settle(&mut runner, "GD-OPT-13");
    assert!(in_trash(&runner, "GD-OPT-13"), "GD-OPT-13 was used");
    assert_eq!(p0_memory(&runner), -10, "0 - (13 - 3) = -10");
}

// ─── Review follow-ups: which reducers the credit counts ────────────────────

/// Kyo Sawashiro (BT25-088) reduces a [Glowing Dawn] card that "would be
/// PLAYED". Using an Option is not playing it (CR 4.3 7-1-1 vs 9-1-1), and
/// DCGO's reducer requires `cardSource.HasPlayCost` (BT25_088.cs:98-103), which
/// is false for every Option (`HasPlayCost => !cardKind.Contains(Option) && ..`,
/// CEntity_Base.cs:340). So a use-cost-14 Option (14 - 3 = 11 from memory 0,
/// one past the limit) is NOT made payable by Kyo's -1 and is not offered.
#[test]
fn st23_04_kyo_play_reducer_is_not_credited_on_an_option_use() {
    let (mut runner, source) = reducer_runner(&["GD-OPT-14"]);
    kyo_with_face_down(&mut runner, 2);
    assert!(
        offered_after_paying(&mut runner, source).is_empty(),
        "Kyo's play reducer must not make an Option use payable"
    );
}

/// Kyo's reducer is not offered on an Option use either: the use of a
/// use-cost-13 Option (13 - 3 = 10 from memory 0) goes through at 10 with no
/// Kyo prompt and Kyo's face-down cards untouched.
#[test]
fn st23_04_kyo_play_reducer_is_not_offered_on_an_option_use() {
    let (mut runner, source) = reducer_runner(&["GD-OPT-13"]);
    let kyo = kyo_with_face_down(&mut runner, 2);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT-13"]);
    let want = PLAY_HAND_START + hand_index_of(&runner, 0, "GD-OPT-13") as u16;
    runner.execute_action(0, want).unwrap();
    assert!(
        runner.pending_selection_view().is_none(),
        "no Kyo reducer prompt on an Option use: {:?}",
        runner.pending_selection_view().map(|v| v.prompt)
    );
    assert!(in_trash(&runner, "GD-OPT-13"), "GD-OPT-13 was used");
    assert_eq!(p0_memory(&runner), -10, "0 - (13 - 3) = -10, no Kyo -1");
    let face_down = runner.game.players[0].battle_area[kyo.index as usize]
        .card_sources
        .iter()
        .filter(|c| c.face_down)
        .count();
    assert!(face_down >= 1, "Kyo's reducer cost was not paid on the Option use");
}

/// Two Option-use reducers apply separately, as in DCGO, where each of
/// Armalizamon (BT25-049) and Tomoro Tenma (BT25-090) keeps its own
/// [Once Per Turn] BeforePayCost activation and its own hidden availability
/// ChangeCostClass (BT25_049.cs:226-255; BT25_090 the same shape), so
/// `PayingCost(checkAvailability: true)` counts -4. A use-cost-17 Option
/// (17 - 3 = 14 from memory 0) is payable at 10 only with both; Tomoro comes
/// first in battle-area order, so crediting only the first reducer drops it.
#[test]
fn st23_04_two_option_use_reducers_are_both_credited_and_paid() {
    let (mut runner, source) = reducer_runner(&["GD-OPT-17"]);
    runner.place_on_field(0, "BT25-090", Some(0));
    runner.place_on_field(0, "BT25-049", Some(0));
    tamer_with_face_down(&mut runner);
    tamer_with_face_down(&mut runner);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT-17"]);
    pick_and_settle(&mut runner, "GD-OPT-17");
    assert!(in_trash(&runner, "GD-OPT-17"), "GD-OPT-17 was used");
    assert_eq!(p0_memory(&runner), -10, "0 - (17 - 3 - 3 - 1) = -10");
}
