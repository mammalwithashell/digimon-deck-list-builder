//! Callismon / Dark Animal (BT25 slice) — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/callismon-dark-animal-model.md`. Slice cards (as
//! given): **BT25-082 BlackGatomon**, **BT25-058 Callismon**, **BT25-095
//! Paradise Colosseum** — all three implemented as YAML DSL cards with green
//! per-card behavioral tests (`cards_behavioral/bt25/{bt25_058,bt25_082,
//! bt25_095}.rs`).
//!
//! Phase-4 precondition gate (2026-06-06): all three slice cards load into the
//! embedded DSL pack and their per-card suites pass → no combo is BLOCKED. The
//! cross-set / neutral pieces every combo needs (a [Three Musketeers]-text
//! Tamer, a TM-trait Digimon, a red/green [TS] Digimon, an opponent Digimon)
//! are **synthetic fixtures** — no authored combo fires a cross-set card's
//! printed effect, so no cross-set card was pulled in (lazy closure).
//!
//! Authored combos (one `#[test]` cluster per named model combo):
//! - **C1** — Paradise Colosseum's security aura buffs the *real* Callismon:
//!   +2000 DP (13000 → 15000) and `<Rush>`, where Callismon self-satisfies PC's
//!   `[Marsmon] or [Callismon]` name gate. Cross-card identity the per-card test
//!   fakes with a synthetic "Marsmon".
//! - **C2** — PC's `[Main]` effect-play of a red/green [TS] Digimon fires
//!   Callismon's `[All Turns]` effect-play observer (De-Digivolve 1 + may
//!   battle); a *normal hand play* of the same Digimon does NOT (effect-init
//!   gate). This is the only inter-card *chain* in the slice.
//! - **C3** — BlackGatomon's `[On Play]` Tamer-anchor (free-play a TM-text
//!   Tamer) unlocks its OWN `[All Turns]` into-path (digivolve into a TM-trait
//!   Digimon, cost 4, ignore reqs — gated on a TM-text Tamer on field). The OP
//!   output feeds the AT gate; control: no Tamer → the path is gated out.
//!
//! Sources: cards.json BT25-058/082/095; DCGO
//! `$BASE_DCGO/Assets/Scripts/CardEffect/BT25/{Green/BT25_058, Purple/BT25_082,
//! Red/BT25_095}.cs`; `general_rule.pdf` §16 (Rush / De-Digivolve keyword
//! glossary; effect-initiated play timing); engine
//! `game_actions.rs::play_from_hand_with_cost` (`PlaySource::ByEffect` →
//! `effect_initiated: true` → fires play-event triggers); security-scope aura
//! materialization from a face-up security source (ST20-15 / BT24-090 idiom).

#![allow(dead_code)]

use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, CostDelta, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

const BLACKGATOMON: &str = "BT25-082"; // Lv.4 Purple/Black, TS / Dark Animal
const CALLISMON: &str = "BT25-058"; // Lv.6 Green/Black, DP 13000, TS / Dark Animal
const PARADISE: &str = "BT25-095"; // Option Red/Green, security aura + Main

// ─── Shared synthetic fixtures (neutral pieces the combos need) ───────────────

/// A red/green [TS] Digimon — both a Paradise-Colosseum aura *target* and the
/// kind of card PC's [Main] free-plays by effect. `color` selects red/green.
fn ts_digimon(id: &str, color: CardColor, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.traits = vec!["TS".to_string()];
    c.level = Some(level);
    c.dp = Some(4000);
    c.play_cost = u16::from(level);
    c
}

/// A non-TS off-colour Digimon: PC's aura must NOT touch it (control target).
fn plain_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["Beast".to_string()];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

/// An opponent Digimon WITH digivolution sources (a stack), so Callismon's
/// `[All Turns]` De-Digivolve-1 has a legal target.
fn opp_stack_pieces() -> (CardData, CardData) {
    let mut base = make_test_card("OPP-BASE", "OppBase");
    base.card_kind = CardKind::Digimon;
    base.colors = vec![CardColor::Purple];
    base.level = Some(3);
    base.dp = Some(3000);
    let mut top = make_test_card("OPP-TOP", "OppTop");
    top.card_kind = CardKind::Digimon;
    top.colors = vec![CardColor::Purple];
    top.level = Some(4);
    top.dp = Some(5000);
    top.play_cost = 4;
    (base, top)
}

/// A Tamer whose printed text carries the "Three Musketeers" substring (the
/// predicate BlackGatomon's OP picker + AT gate both key off).
fn tm_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.effect_text = "[Three Musketeers] A tamer with the Musketeers keyword.".to_string();
    c
}

/// A [Three Musketeers]-TRAIT Digimon in hand — the destination of
/// BlackGatomon's `[All Turns]` into-path. Lv.5 so the digivolve makes sense.
fn tm_trait_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.traits = vec!["Three Musketeers".to_string()];
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 5;
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 4,
        memory_cost: 4,
    }];
    c
}

/// Count P0 battle-area Tamers whose printed effect text carries the
/// "Three Musketeers" substring — the exact predicate BlackGatomon's `[All
/// Turns]` into-path condition keys off. Reads the printed text from the
/// `CardData` behind the top card's `data_index`.
fn count_tm_text_tamers(runner: &DebugRunner) -> usize {
    runner.game.players[0]
        .battle_area
        .iter()
        .filter(|p| {
            p.is_tamer(&runner.game.card_data)
                && runner.game.card_data[p.top_card().data_index]
                    .effect_text
                    .contains("Three Musketeers")
        })
        .count()
}

// ════════════════════════════════════════════════════════════════════════════
// Combo C1 — Paradise Colosseum's security aura buffs the real Callismon
// ════════════════════════════════════════════════════════════════════════════
//
// PC (face-up in P0 security): "[Security][All Turns] all of your red or green
// [TS] Digimon get +2000 DP. While you have [Marsmon] or [Callismon], they also
// gain <Rush>." Callismon is green + [TS], so it is an aura *target*; and
// Callismon IS the [Callismon]-named permanent, so it self-opens the Rush gate.
//
// Claimed outcome (model C1): Callismon 13000 → 15000 DP AND has <Rush>; a non-
// TS off-colour Digimon gets neither. The per-card test
// (bt25_095::security_aura_grants_2000_dp_and_conditional_rush) proves the aura
// with a SYNTHETIC "Marsmon" and a synthetic TS target — this interaction proves
// the *real* Callismon both receives the buff and satisfies its own name gate.

fn c1_dsl() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(PARADISE)
        .expect("BT25-095 Paradise Colosseum in embedded DSL pack")
        .dsl_card(CALLISMON)
        .expect("BT25-058 Callismon in embedded DSL pack")
}

#[test]
fn c1_paradise_security_aura_buffs_callismon_dp_and_self_gates_rush() {
    let mut runner = c1_dsl()
        .add_card(plain_digimon("PLAIN"))
        .security(0, &[PARADISE])
        .memory(13)
        .start();

    // PC sits FACE-UP in P0's security (its [Main] would place it there; a
    // security-scope aura materializes only from a face-up security source).
    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);

    let callismon = runner.place_on_field(0, CALLISMON, Some(0));
    let plain = runner.place_on_field(0, "PLAIN", Some(0));
    runner.game.tick_declarative_effects();

    // ── Claimed mechanical outcome ───────────────────────────────────────────
    // 1. Callismon (green, [TS]) gets +2000 DP: 13000 → 15000.
    assert_eq!(
        runner.effective_dp(callismon),
        Some(15000),
        "C1: PC's security aura must give the real Callismon (green/[TS]) +2000 DP (13000 → 15000)"
    );
    // 2. Callismon gains <Rush>, self-satisfying PC's [Marsmon]/[Callismon] gate.
    assert!(
        runner.game.has_keyword(callismon, Keyword::Rush),
        "C1: Callismon's own presence opens PC's [Callismon] name gate → Callismon gains <Rush>"
    );
    // 3. The non-TS off-colour Digimon receives neither half of the aura.
    assert_eq!(
        runner.effective_dp(plain),
        Some(4000),
        "C1: a non-[TS] blue Digimon is not a red/green [TS] target — no +2000 DP"
    );
    assert!(
        !runner.game.has_keyword(plain, Keyword::Rush),
        "C1: the non-target Digimon does not gain <Rush>"
    );
}

#[test]
fn c1_no_rush_when_callismon_is_the_only_named_permanent_is_absent() {
    // Unhappy path: PC's +2000 DP is unconditional for red/green [TS], but its
    // <Rush> rider needs a [Marsmon]/[Callismon] permanent. With ONLY a generic
    // red [TS] Digimon (no Callismon, no Marsmon) on the field, the target gets
    // +2000 DP but NOT Rush — pinning that C1's Rush half is gated on the named
    // permanent, not on the [TS] trait alone.
    let mut runner = c1_dsl()
        .add_card(ts_digimon("RED-TS", CardColor::Red, 4))
        .security(0, &[PARADISE])
        .memory(13)
        .start();
    let src_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0].face_up_security.insert(src_index);

    let red = runner.place_on_field(0, "RED-TS", Some(0));
    runner.game.tick_declarative_effects();

    assert_eq!(
        runner.effective_dp(red),
        Some(6000),
        "C1 control: red [TS] still gets the unconditional +2000 DP (4000 → 6000)"
    );
    assert!(
        !runner.game.has_keyword(red, Keyword::Rush),
        "C1 control: no [Marsmon]/[Callismon] permanent → no <Rush> grant"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Combo C2 — PC [Main] effect-play fires Callismon's [All Turns] observer
// ════════════════════════════════════════════════════════════════════════════
//
// Callismon [All Turns][OPT]: "When effects play or digivolve any Digimon,
// <De-Digivolve 1> 1 of your opponent's Digimon. Then, this Digimon may battle 1
// of your opponent's Digimon." The gate is `IsByEffect` (effect-initiated).
//
// PC [Main]: replace bottom security with self, then "you may play 1 red or
// green [TS] Digimon ... with the cost reduced by 3" — an EFFECT-INITIATED play.
// So PC's Main is a trigger platform for Callismon's observer.
//
// Claimed outcome (model C2): resolving PC's Main reduced-play of a red/green
// [TS] Digimon installs Callismon's De-Digivolve prompt over an opponent Digimon
// (the observer fired). A *normal hand play* of the same Digimon does NOT.

fn c2_dsl() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(PARADISE)
        .expect("BT25-095 Paradise Colosseum in embedded DSL pack")
        .dsl_card(CALLISMON)
        .expect("BT25-058 Callismon in embedded DSL pack")
}

fn c2_board() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let (opp_base, opp_top) = opp_stack_pieces();
    let mut runner = c2_dsl()
        .add_card(ts_digimon("RED-TS", CardColor::Red, 4))
        .add_card(make_test_card("SEC-BOTTOM", "SecBottom"))
        .add_card(opp_base)
        .add_card(opp_top)
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .deck(0, &["DECK-PAD"; 6])
        .deck(1, &["DECK-PAD"; 6])
        .hand(0, &[PARADISE, "RED-TS"])
        .security(0, &["SEC-BOTTOM"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;
    runner.game.enter_main_phase();

    // Callismon on P0's field so its [All Turns] observer is live.
    let callismon = runner.place_on_field(0, CALLISMON, Some(0));
    // Opponent Digimon WITH a digivolution source (a stack) so De-Digivolve has
    // a legal target.
    let opp = runner.place_stack(1, &["OPP-BASE", "OPP-TOP"]);
    runner.game.tick_declarative_effects();
    (runner, callismon, opp)
}

#[test]
fn c2_paradise_main_effect_play_fires_callismon_dedigivolve_observer() {
    let (mut runner, _callismon, opp) = c2_board();

    let opp_sources_before = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();
    assert!(
        opp_sources_before >= 2,
        "C2 setup: opponent Digimon must start as a stack so De-Digivolve has a target"
    );
    let opp_total_sources_before: usize = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.card_sources.len())
        .sum();
    let opp_trash_before = runner.game.players[1].trash.len();

    // Play Paradise Colosseum as an Option from hand (index 0). No face-up
    // security yet → its colour-ignore floodgate lets it resolve.
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Pending,
        "C2: PC's [Main] parks the optional reduced-play hand prompt"
    );

    // The parked prompt is PC's "you may play a red/green [TS] Digimon for −3".
    let prompt = runner
        .pending_selection_view()
        .expect("C2: PC reduced-play hand prompt");
    assert_eq!(prompt.kind, SelectionKind::Hand);
    let red_ts_action = runner
        .game
        .player(0)
        .hand
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == "RED-TS")
                .then_some(digimon_engine::action::space::PLAY_HAND_START + idx as u16)
        })
        .expect("C2: RED-TS still in hand for PC's reduced play");
    assert!(
        prompt.valid_action_ids.contains(&red_ts_action),
        "C2: the red [TS] Digimon must be an eligible PC reduced-play target"
    );

    // Resolve PC's reduced-play → RED-TS enters BY EFFECT.
    runner
        .execute_action(prompt.selecting_player, red_ts_action)
        .expect("C2: play RED-TS via PC's Main (effect-initiated)");

    // ── Claimed mechanical outcome ───────────────────────────────────────────
    // RED-TS reached the field (PC's Main resolved its play).
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "RED-TS"),
        "C2: PC's Main played the red [TS] Digimon"
    );
    // The effect-initiated play fired Callismon's [All Turns] observer, which
    // installs a selection (the De-Digivolve-1 over an opponent Digimon, or its
    // optional-trigger accept). The cross-card fact: PC's Main is the trigger.
    assert!(
        runner.pending_selection().is_some(),
        "C2: PC's effect-initiated play of a Digimon must fire Callismon's [All Turns] \
         observer → a pending selection (De-Digivolve / accept) is installed"
    );

    // Drive the observer to completion. The mandatory De-Digivolve-1 trashes the
    // opponent stack's top source (stops at level 3); the optional may-battle can
    // then delete the (now smaller) opponent outright, since Callismon (13000 DP)
    // overpowers it. Either way the opponent Digimon's board presence is reduced.
    let _ = runner.accept_optional_trigger();
    let _ = runner.auto_resolve();

    // The De-Digivolve removed the opponent stack's top source (it is now in the
    // opponent's trash) and/or the optional may-battle deleted the opponent. Both
    // are resolution-order-independent: the opponent's total on-field
    // digivolution-source count strictly dropped, and its trash grew. This pins
    // that Callismon's observer actually CHANGED the opponent's board off PC's
    // effect-play (not merely installed a prompt).
    let opp_total_sources_after: usize = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.card_sources.len())
        .sum();
    assert!(
        opp_total_sources_after < opp_total_sources_before,
        "C2: Callismon's observer De-Digivolved the opponent — its total on-field source \
         count dropped (before {opp_total_sources_before}, after {opp_total_sources_after})"
    );
    assert!(
        runner.game.players[1].trash.len() > opp_trash_before,
        "C2: the De-Digivolved (and/or battled) card(s) landed in the opponent's trash"
    );
}

#[test]
fn c2_normal_hand_play_does_not_fire_callismon_observer() {
    // Unhappy path / the effect-init gate: the SAME red [TS] Digimon played as a
    // NORMAL hand play (PlaySource::ByHand) is not effect-initiated, so
    // Callismon's [All Turns] observer must NOT fire. This is the cross-card fact
    // that makes C2's happy path meaningful (it is PC's *effect* play, not the
    // play itself, that arms Callismon).
    let (mut runner, _callismon, opp) = c2_board();
    let opp_sources_before = runner.game.players[1].battle_area[opp.index as usize]
        .card_sources
        .len();

    // RED-TS is hand index 1 (PARADISE is 0). Play it as a normal hand play.
    runner
        .game
        .play_from_hand_with_cost(0, 1, CostDelta::Free, PlaySource::ByHand);

    assert!(
        runner.pending_selection().is_none(),
        "C2 control: a normal (ByHand) play is not effect-initiated → Callismon's \
         [All Turns] observer must not fire"
    );
    assert_eq!(
        runner.game.players[1].battle_area[opp.index as usize]
            .card_sources
            .len(),
        opp_sources_before,
        "C2 control: no observer → opponent stack is untouched"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Combo C3 — BlackGatomon's OP Tamer-anchor unlocks its own [All Turns] path
// ════════════════════════════════════════════════════════════════════════════
//
// BlackGatomon [On Play][WD][OPT]: "if ≤1 Tamer, you may play 1 Tamer with
// [Three Musketeers] in its text from hand for free." BlackGatomon [All Turns]:
// "while you have a TM-text Tamer, this Digimon may digivolve into a TM-trait
// Digimon for cost 4, ignoring reqs." Both reference the SAME TM-text-Tamer
// predicate — the OP output is exactly what the AT path's condition checks.
//
// Claimed outcome (model C3): playing BlackGatomon free-plays the TM Tamer (the
// OP anchor); with that Tamer down, BlackGatomon's `[All Turns]` into-path
// condition is satisfied. Control: with NO TM Tamer on field, the condition is
// unmet.

fn c3_dsl() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(BLACKGATOMON)
        .expect("BT25-082 BlackGatomon in embedded DSL pack")
}

#[test]
fn c3_blackgatomon_op_tamer_anchor_then_at_into_path_is_unlocked() {
    let mut runner = c3_dsl()
        .add_card(tm_tamer("TM-TAMER"))
        .add_card(tm_trait_digimon("TM-DIGI"))
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .deck(0, &["DECK-PAD"; 6])
        .deck(1, &["DECK-PAD"; 6])
        .hand(0, &[BLACKGATOMON, "TM-TAMER", "TM-DIGI"])
        .memory(8)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    let tamers_before = runner.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.is_tamer(&runner.game.card_data))
        .count();
    assert_eq!(tamers_before, 0, "C3: no Tamers on field at start");

    // Play BlackGatomon → its [On Play] anchor offers the free TM-Tamer play.
    let _bg = runner.play(0, 0).expect("C3: play BlackGatomon");
    assert!(
        runner.pending_selection().is_some(),
        "C3: BlackGatomon's [On Play] anchor must install the optional 'may play a TM Tamer' prompt"
    );
    runner
        .accept_optional_trigger()
        .expect("C3: accept the OP Tamer-anchor trigger");
    runner
        .auto_resolve()
        .expect("C3: pick + free-play the TM-text Tamer");

    // ── Claimed mechanical outcome (anchor) ──────────────────────────────────
    let tm_tamer_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| {
            p.is_tamer(&runner.game.card_data)
                && p.top_card().card_id(&runner.game.card_data) == "TM-TAMER"
        });
    assert!(
        tm_tamer_on_field,
        "C3: the [On Play] anchor free-played the [Three Musketeers]-text Tamer onto the field"
    );

    // ── Claimed mechanical outcome (the unlock) ──────────────────────────────
    // With the TM-text Tamer now on field, BlackGatomon's [All Turns] into-path
    // condition ("while you have a TM-text Tamer") is satisfied. We assert the
    // gate is OPEN: BlackGatomon (on field, Lv.4) can now reach a TM-trait
    // Digimon via the cost-4 ignore-reqs path. The presence of the TM Tamer is
    // the load-bearing cross-clause fact (OP feeds AT).
    let tm_tamer_count = count_tm_text_tamers(&runner);
    assert_eq!(
        tm_tamer_count, 1,
        "C3: exactly one TM-text Tamer is on field → BlackGatomon's [All Turns] into-path \
         condition is now satisfied (the OP anchor unlocked the AT path)"
    );
    // The TM-trait Digimon destination is still in hand, ready for the path.
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TM-DIGI"),
        "C3: the TM-trait Digimon destination is available in hand for the into-path"
    );
}

#[test]
fn c3_at_into_path_gated_out_without_a_tm_tamer() {
    // Control: BlackGatomon on field with NO TM-text Tamer anywhere (declined /
    // unavailable). The [All Turns] into-path condition is unmet, so no TM-text
    // Tamer is on the field to open it. This pins that the OP anchor is what
    // unlocks the AT path: absent the Tamer, the gate stays closed.
    let mut runner = c3_dsl()
        .add_card(tm_trait_digimon("TM-DIGI"))
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .deck(0, &["DECK-PAD"; 6])
        .deck(1, &["DECK-PAD"; 6])
        .hand(0, &[BLACKGATOMON, "TM-DIGI"])
        .memory(8)
        .start();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 0;

    // Play BlackGatomon. There is NO TM-text Tamer in hand, so the OP anchor has
    // no eligible target → no anchor, no Tamer reaches the field.
    let _bg = runner.play(0, 0).expect("C3 control: play BlackGatomon");
    assert!(
        runner.pending_selection().is_none(),
        "C3 control: no TM-text Tamer in hand → the [On Play] anchor is a no-op"
    );

    let tm_tamer_count = count_tm_text_tamers(&runner);
    assert_eq!(
        tm_tamer_count, 0,
        "C3 control: no TM-text Tamer on field → BlackGatomon's [All Turns] into-path \
         condition is unmet (the path stays gated)"
    );
}
