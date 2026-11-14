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
//! - CR 4.3 §§15-1-2, 15-1-4: one shared effect processes the mandatory
//!   DP increase before its optional Tamer-payment and hand choices.
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
/// `target_card` at the `Hand` prompt and the first legal Tamer at its payment
/// prompt. The mandatory DP increase and optional tail are one effect.
fn drive_choosing(runner: &mut DebugRunner, target_card: &str) {
    let mut guard = 0;
    while let Some(v) = runner.pending_selection_view() {
        guard += 1;
        assert!(guard < 16, "prompt loop did not terminate");
        assert_ne!(
            v.kind,
            SelectionKind::TriggerOrder,
            "the Then tail is not independently orderable"
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
    assert_eq!(op_wd, 1, "DP increase and Then tail are one printed effect");
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

    // Mandatory DP increase → optional Tamer payment → optional hand pick.
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
    for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        let mut runner = base();
        let tamer = tamer_with_face_down(&mut runner);
        push_to_hand(&mut runner, 0, "GD-DIGI");
        let monarch = runner.place_on_field(0, CARD_ID, Some(0));
        let memory_before = runner.memory();
        let field_before = runner.game.players[0].battle_area.len();

        fire_shared(&mut runner, timing, monarch);

        assert_eq!(runner.effective_dp(monarch), Some(10000));
        assert_optional_prompt(
            &runner,
            SelectionKind::OwnField,
            &[encode_attack(0, tamer.index as u16)],
        );
        runner.execute_action(0, PASS).unwrap();

        assert!(runner.pending_selection().is_none());
        assert_eq!(runner.effective_dp(monarch), Some(10000));
        assert_eq!(runner.trash_size(0), 0, "no trash on decline");
        assert_eq!(runner.memory(), memory_before);
        assert_eq!(runner.game.players[0].battle_area.len(), field_before);
        assert_eq!(
            runner.game.players[0].battle_area[tamer.index as usize]
                .card_sources
                .len(),
            2
        );
        assert!(runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "GD-DIGI"));
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — One-effect ordering and optional-tail action masks
// ═══════════════════════════════════════════════════════════════════════════════

fn fire_shared(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn assert_optional_prompt(runner: &DebugRunner, kind: SelectionKind, ids: &[u16]) {
    let view = runner
        .pending_selection_view()
        .expect("player-visible optional choice");
    assert_eq!(
        view.kind, kind,
        "one printed effect has no TriggerOrder gate"
    );
    assert_eq!(view.selecting_player, 0);
    assert!(view.is_optional);
    assert_eq!(view.valid_action_ids, ids);
    for (action, enabled) in build_action_mask(&runner.game, 0).iter().enumerate() {
        assert_eq!(
            *enabled,
            if ids.contains(&(action as u16)) || action == PASS as usize {
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
fn st23_08_shared_effect_buffs_before_optional_payment_and_filtered_hand_choices() {
    for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        for payload in [Some("GD-DIGI"), Some("GD-OPT"), None] {
            let mut runner = base();
            let untouched_tamer = tamer_with_face_down(&mut runner);
            let paid_tamer = runner.place_stack(0, &["FILLER", "HOST", "TAMER"]);
            for source in
                &mut runner.game.players[0].battle_area[paid_tamer.index as usize].card_sources[..2]
            {
                source.face_down = true;
            }
            let face_up_tamer = runner.place_stack(0, &["FILLER", "TAMER"]);
            let wrong_host = runner.place_stack(0, &["FILLER", "HOST"]);
            runner.game.players[0].battle_area[wrong_host.index as usize].card_sources[0]
                .face_down = true;
            let opposing_tamer = runner.place_stack(1, &["FILLER", "TAMER"]);
            runner.game.players[1].battle_area[opposing_tamer.index as usize].card_sources[0]
                .face_down = true;
            push_to_hand(&mut runner, 0, "FILLER");
            push_to_hand(&mut runner, 0, "GD-DIGI");
            push_to_hand(&mut runner, 0, "GD-OPT");
            let monarch = runner.place_on_field(0, CARD_ID, Some(0));
            let memory_before = runner.memory();

            fire_shared(&mut runner, timing, monarch);

            assert_eq!(
                runner.effective_dp(monarch),
                Some(10000),
                "the mandatory DP increase precedes every optional decision"
            );
            assert_optional_prompt(
                &runner,
                SelectionKind::OwnField,
                &[
                    encode_attack(0, untouched_tamer.index as u16),
                    encode_attack(0, paid_tamer.index as u16),
                ],
            );
            runner
                .execute_action(0, encode_attack(0, paid_tamer.index as u16))
                .unwrap();

            assert_eq!(runner.effective_dp(monarch), Some(10000));
            assert_eq!(runner.trash_size(0), 1);
            assert_eq!(
                runner.game.players[0].trash[0].card_id(&runner.game.card_data),
                "FILLER",
                "payment trashes the bottom face-down source"
            );
            assert_eq!(
                runner.game.players[0].battle_area[paid_tamer.index as usize].card_sources[0]
                    .card_id(&runner.game.card_data),
                "HOST",
                "the higher face-down source remains"
            );
            for untouched in [untouched_tamer, face_up_tamer, wrong_host] {
                assert_eq!(
                    runner.game.players[0].battle_area[untouched.index as usize]
                        .card_sources
                        .len(),
                    2
                );
            }
            assert_eq!(
                runner.game.players[1].battle_area[opposing_tamer.index as usize]
                    .card_sources
                    .len(),
                2
            );
            let digimon = PLAY_HAND_START + hand_index_of(&runner, 0, "GD-DIGI") as u16;
            let option = PLAY_HAND_START + hand_index_of(&runner, 0, "GD-OPT") as u16;
            assert_optional_prompt(&runner, SelectionKind::Hand, &[digimon, option]);
            let action = match payload {
                Some("GD-DIGI") => digimon,
                Some("GD-OPT") => option,
                None => PASS,
                _ => unreachable!(),
            };
            runner.execute_action(0, action).unwrap();

            assert!(runner.pending_selection().is_none());
            assert_eq!(runner.effective_dp(monarch), Some(10000));
            assert_eq!(
                runner.memory(),
                memory_before - if payload.is_some() { 2 } else { 0 }
            );
            assert_eq!(
                runner.trash_size(0),
                if payload == Some("GD-OPT") { 2 } else { 1 }
            );
            assert_eq!(
                runner.game.players[0]
                    .battle_area
                    .iter()
                    .filter(|p| p.top_card().card_id(&runner.game.card_data) == "GD-DIGI")
                    .count(),
                usize::from(payload == Some("GD-DIGI"))
            );
            for card in ["GD-DIGI", "GD-OPT"] {
                assert_eq!(
                    runner.game.players[0]
                        .hand
                        .iter()
                        .any(|c| c.card_id(&runner.game.card_data) == card),
                    payload != Some(card)
                );
            }
            assert!(runner.game.players[0]
                .hand
                .iter()
                .any(|c| c.card_id(&runner.game.card_data) == "FILLER"));
        }
    }
}

#[test]
fn st23_08_opponents_turn_still_buffs_without_payment_or_payload() {
    for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        let mut runner = base();
        runner.set_first_player(1);
        let tamer = tamer_with_face_down(&mut runner);
        push_to_hand(&mut runner, 0, "GD-DIGI");
        let monarch = runner.place_on_field(0, CARD_ID, Some(0));
        let memory_before = runner.memory();

        fire_shared(&mut runner, timing, monarch);

        assert_eq!(runner.effective_dp(monarch), Some(10000));
        assert!(runner.pending_selection().is_none());
        assert_eq!(runner.trash_size(0), 0);
        assert_eq!(runner.memory(), memory_before);
        assert_eq!(
            runner.game.players[0].battle_area[tamer.index as usize]
                .card_sources
                .len(),
            2
        );
        assert!(runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "GD-DIGI"));
    }
}

#[test]
fn st23_08_unpayable_tamer_cost_still_buffs_without_free_payload() {
    for timing in [EffectTiming::OnPlay, EffectTiming::WhenDigivolving] {
        for has_face_up_tamer in [false, true] {
            let mut runner = base();
            if has_face_up_tamer {
                runner.place_stack(0, &["FILLER", "TAMER"]);
            }
            let wrong_host = runner.place_stack(0, &["FILLER", "HOST"]);
            runner.game.players[0].battle_area[wrong_host.index as usize].card_sources[0]
                .face_down = true;
            push_to_hand(&mut runner, 0, "GD-DIGI");
            let monarch = runner.place_on_field(0, CARD_ID, Some(0));
            let memory_before = runner.memory();

            fire_shared(&mut runner, timing, monarch);

            assert_eq!(runner.effective_dp(monarch), Some(10000));
            assert!(
                runner.pending_selection().is_none(),
                "unpayable cost must not offer a hand pick"
            );
            assert_eq!(runner.trash_size(0), 0);
            assert_eq!(runner.memory(), memory_before);
            assert!(runner.game.players[0]
                .hand
                .iter()
                .any(|c| c.card_id(&runner.game.card_data) == "GD-DIGI"));
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited [End of Attack][OPT]: trash FD → unsuspend self
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

/// CR 4.3 §15-7-5: the cost can be paid even when the non-Glowing-Dawn
/// carrier cannot receive the conditional unsuspend payload.
#[test]
fn st23_08_inherited_non_glowing_dawn_host_can_pay_or_decline_cost() {
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
// DCGO ST23_08.cs:98-104 CanSelectCardCondition = `EqualsTraits("Glowing
// Dawn") && ((IsOption && !CanNotPlayThisOption) || (HasPlayCost &&
// CanPlayAsNewPermanent(cardSource, true, activateClass, fixedCost:
// GetCostItself - 3)))`, the SelectHandEffect filter (:114) — identical to
// Murasamemon (ST23-04):
// - a Digi-Egg has no play cost (CR 4.3 7-1-1) — e.g. Kekkomon (ST23-01);
// - a Digimon needs a free frame, no effect-play lock, and memory for the
//   reduced cost (CanPlayCardTargetFrame: `Owner.MaxMemoryCost < cost` refuses);
//   the frame cap is the engine's `rules.field_slots`, mirroring DCGO's frame
//   limit (CR 4.3 3-4-8-2 itself sets no battle-area limit);
// - an Option needs no "can't use Options" lock and its colour requirement met
//   (CanNotPlayThisOption, CardSource.cs:184-246; CR 4.3 4-22-1..3).
// DCGO does not check memory for the Option half; the rules do: CR 4.3
// 1-3-11-1, 4-2-2 + 1-4-2-2, 9-1-8. A DUAL is used (Option information, CR 4.3
// 4-6-2) and priced at its use cost.

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

/// Monarchlizamon (ST23-08) on the field with a payable Tamer; `hand` pushed to
/// p0's hand. Includes the real Kekkomon (ST23-01, [Glowing Dawn] Digi-Egg) and
/// Habakirimon (BT25-043, [Glowing Dawn] DUAL, use cost 6).
fn legality_runner(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST23-08 in embedded DSL pack")
        .dsl_card("ST23-01")
        .expect("Kekkomon in embedded DSL pack")
        .dsl_card("BT25-043")
        .expect("Habakirimon in embedded DSL pack")
        .add_card(gd_digimon("GD-DIGI"))
        .add_card(gd_option("GD-OPT"))
        .add_card(gd_card("GD-BIG", CardKind::Digimon, CardColor::Green, 14))
        .add_card(gd_card("GD-OPT-BIG", CardKind::Option, CardColor::Green, 14))
        .add_card(gd_card("GD-OPT-RED", CardKind::Option, CardColor::Red, 3))
        .add_card(make_tamer("TAMER"))
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(8)
        .start();
    runner.set_first_player(0);
    tamer_with_face_down(&mut runner);
    for h in hand {
        push_to_hand(&mut runner, 0, h);
    }
    let source = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, source)
}

/// Fire [On Play], pay the Tamer trash, and return the sorted card ids offered
/// at the [Glowing Dawn] hand pick (empty when no hand pick installs).
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
            .expect("a non-decline action (Tamer payment)");
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
fn st23_08_hand_pick_offers_playable_digimon_and_usable_option() {
    let (mut runner, source) = legality_runner(&["GD-DIGI", "GD-OPT"]);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-DIGI", "GD-OPT"]);
}

/// Kekkomon (ST23-01) is a [Glowing Dawn] Digi-Egg with no play cost: never
/// offered (CR 4.3 7-1-1).
#[test]
fn st23_08_hand_pick_excludes_glowing_dawn_digi_egg() {
    let (mut runner, source) = legality_runner(&["ST23-01", "GD-DIGI"]);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-DIGI"]);
}

/// "You can't play Digimon by effects": the Digimon is not offered, the Option is.
#[test]
fn st23_08_hand_pick_excludes_digimon_under_effect_play_prohibition() {
    let (mut runner, source) = legality_runner(&["GD-DIGI", "GD-OPT"]);
    add_player_lock(&mut runner, digimon_engine::enums::ModifierType::CannotPlayDigimonByEffect);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT"]);
}

/// No free frame: the Digimon is not offered, the Option is.
/// The frame cap is the engine's `rules.field_slots` (mirroring DCGO's frame
/// limit); CR 4.3 3-4-8-2 itself sets no battle-area limit.
#[test]
fn st23_08_hand_pick_excludes_digimon_when_battle_area_full() {
    let (mut runner, source) = legality_runner(&["GD-DIGI", "GD-OPT"]);
    runner.game.rules.field_slots = runner.game.players[0].battle_area.len() as u8;
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT"]);
}

/// "You can't use Option cards": the Option is not offered, the Digimon is.
#[test]
fn st23_08_hand_pick_excludes_option_when_options_cannot_be_used() {
    let (mut runner, source) = legality_runner(&["GD-DIGI", "GD-OPT"]);
    add_player_lock(&mut runner, digimon_engine::enums::ModifierType::CannotUseOptionCards);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-DIGI"]);
}

/// A red [Glowing Dawn] Option with no `<Use Req.>` while only green cards are
/// on p0's field: its colour requirement is unmet, so it can't be used.
#[test]
fn st23_08_hand_pick_excludes_option_whose_colour_requirement_is_unmet() {
    let (mut runner, source) = legality_runner(&["GD-OPT-RED", "GD-OPT"]);
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT"]);
}

/// Memory 0: a cost-14 Digimon would cost 11 after the reduction, past the
/// 10-memory limit on the opponent's side, so it can't be played (DCGO
/// `MaxMemoryCost`); the cost-5 one (pay 2) still is.
#[test]
fn st23_08_hand_pick_excludes_unaffordable_digimon() {
    let (mut runner, source) = legality_runner(&["GD-BIG", "GD-DIGI"]);
    runner.game.memory = 0;
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-DIGI"]);
}

/// Memory 0: a use-cost-14 Option (pay 11) can't be used. DCGO offers it (no
/// memory check on the Option half); CR 4.3 1-3-11-1 / 9-1-8 do not.
#[test]
fn st23_08_hand_pick_excludes_unaffordable_option() {
    let (mut runner, source) = legality_runner(&["GD-OPT-BIG", "GD-OPT"]);
    runner.game.memory = 0;
    assert_eq!(offered_after_paying(&mut runner, source), vec!["GD-OPT"]);
}

/// Habakirimon (BT25-043), a [Glowing Dawn] DUAL, is used, so it is priced at
/// its use cost 6 (pay 3 after the reduction), affordable at memory 0.
#[test]
fn st23_08_hand_pick_offers_dual_priced_at_its_use_cost() {
    let (mut runner, source) = legality_runner(&["BT25-043"]);
    runner.game.memory = 0;
    assert_eq!(offered_after_paying(&mut runner, source), vec!["BT25-043"]);
}
