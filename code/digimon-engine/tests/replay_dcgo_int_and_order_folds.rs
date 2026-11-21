//! DCGO replay: three places where DCGO's recorded decision shape differs from
//! the engine's prompt, and the replay layer must fold the DCGO rows onto the
//! engine's prompts WITHOUT guessing.
//!
//! 1. Chirinmon (EX13-032): DCGO registers the [When Digivolving] / [When
//!    Attacking] effect `optional: false` and asks its own one-based int menu
//!    ("Trash top security card" = 1 / "Trash bottom face down card from 1
//!    tamer" = 2 / "Don't pay the cost" = 3, EX13_032.cs:69-73). The engine
//!    models the same "may" as an outer accept/decline gate
//!    (`ResumeFrame::OuterOptionalTrigger`), so DCGO's 3 is the decline of
//!    that gate and 1 / 2 are the acceptance plus the cost choice.
//! 2. J.P., Koji, & Koichi (AD1-023) and Takuya Kanbara & Koji Minamoto
//!    (BT18-088) at the start of the main phase: DCGO lists only the effects
//!    whose `CanActivate` holds (BT18-088 needs a [Hybrid] card in the trash,
//!    BT18_088.cs), activates a lone one without a `MultipleSkills` order row,
//!    and asks the effect's own zone menu. The engine queues both and parks a
//!    trigger order, so the menu int meets the order.
//! 3. Dark to Light, Thunder to Gunfire (BT18-097): DCGO asks the membership
//!    picks (hand, then trash) and then, for 2+ cards, a separate ORDER pick
//!    over exactly those cards ("cards with lower numbers are on top",
//!    BT18_097.cs:190-215) before `AddDigivolutionCardsBottom`. The engine's
//!    single `select_zone_cards` takes the PICK order as the placement order.
//!
//! Staged with `DebugRunner` (real YAML, synthetic boards). No recording
//! content.

use std::collections::HashMap;

use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::dcgo_recording::SelectionRow;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec};
use digimon_engine::TriggerSource;

struct StagedDcgo {
    game: Game,
    steps: Vec<StepSpec>,
}

impl RecordingSource for StagedDcgo {
    fn build_initial_game(&self, _: &HashMap<String, CardData>) -> Result<Game, ReplayError> {
        Ok(self.game.clone())
    }
    fn relay_initial_state(&self, game: &mut Game) -> Result<(), ReplayError> {
        *game = self.game.clone();
        Ok(())
    }
    fn steps(&self) -> &[StepSpec] {
        &self.steps
    }
    fn default_policy(&self) -> StepPolicy {
        StepPolicy::CheckThenApply
    }
    fn omits_dcgo_silent_prompts(&self) -> bool {
        true
    }
}

fn spec(action_id: u16, source: &str, selection: Option<SelectionRow>) -> StepSpec {
    StepSpec {
        actor: 0,
        action_id,
        phase: "Main".into(),
        source: source.into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection,
        board_p0: None,
        board_p1: None,
        card_id: None,
        dna_materials: None,
        inferred: false,
    }
}

fn row(v: serde_json::Value) -> StepSpec {
    let mut full = serde_json::json!({"step": 0, "actor": 0});
    for (k, val) in v.as_object().unwrap() {
        full[k] = val.clone();
    }
    spec(0, "selection", Some(serde_json::from_value::<SelectionRow>(full).unwrap()))
}

fn menu(v: i64) -> StepSpec {
    row(serde_json::json!({"prompt": "generic_int", "int_value": v}))
}

fn hand_pick(ids: &[&str]) -> StepSpec {
    row(serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ids}))
}

fn card_pick(ids: &[&str]) -> StepSpec {
    row(serde_json::json!({"prompt": "SelectCardEffect", "card_ids": ids}))
}

fn run(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    let mut s = ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session");
    for _ in 0..30 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    s
}

fn selection_reason(s: &ReplaySession) -> String {
    s.divergences()
        .iter()
        .find_map(|d| match &d.kind {
            DivergenceKind::SelectionResolution { reason } => Some(reason.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a SelectionResolution divergence: {:?}", s.divergences()))
}

fn hybrid(id: &str, name: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![color];
    c.traits = vec!["Hybrid".to_string()];
    c
}

/// The digivolution cards under the permanent whose top card is `top`, bottom
/// first (the engine's `card_sources`, index 0 = bottom).
fn under(game: &Game, top: &str) -> Vec<String> {
    let p = game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&game.card_data) == top)
        .unwrap_or_else(|| panic!("{top} on the field"));
    p.card_sources.iter().map(|c| c.card_id(&game.card_data).to_string()).filter(|c| c != top).collect()
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. Chirinmon (EX13-032): the int menu against the outer gate
// ═════════════════════════════════════════════════════════════════════════════

fn chirinmon_gate(security: usize, face_down_tamer: bool) -> Game {
    let digimon = |id: &str, level: u8, dp: i32| {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Digimon;
        c.colors = vec![CardColor::Yellow];
        c.level = Some(level);
        c.dp = Some(dp);
        c.play_cost = 3;
        c
    };
    let mut tamer = make_test_card("TAMER", "TAMER");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.colors = vec![CardColor::Yellow];
    let mut r = DebugRunner::builder()
        .dsl_card("EX13-032")
        .expect("EX13-032 YAML")
        .add_card(digimon("FILL", 3, 1000))
        .add_card(digimon("SEC", 3, 1000))
        .add_card(digimon("BASE-L4", 4, 4000))
        .add_card(digimon("OPP", 4, 4000))
        .add_card(tamer)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &vec!["SEC"; security])
        .security(1, &["SEC"; 5])
        .memory(5)
        .start();
    if face_down_tamer {
        let t = r.place_stack(0, &["FILL", "TAMER"]);
        r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    }
    let chirin = r.place_stack(0, &["BASE-L4", "EX13-032"]);
    r.place_on_field(1, "OPP", Some(0));
    r.attack_player(chirin, 1, false);
    assert!(r.pending_is_optional(), "the outer accept/decline gate is parked");
    r.game
}

fn chirinmon_suspended(game: &Game) -> bool {
    game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&game.card_data) == "EX13-032")
        .expect("Chirinmon")
        .is_suspended
}

/// DCGO's "Don't pay the cost" (3) declines the engine's outer gate: nothing is
/// paid, so Chirinmon stays suspended and no security card moves.
#[test]
fn ex13_032_dont_pay_declines_the_outer_gate() {
    let s = run(chirinmon_gate(2, true), vec![menu(3)]);
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete());
    assert!(s.game.pending_selection.is_none(), "the declined effect left nothing parked");
    assert!(chirinmon_suspended(&s.game), "no cost paid, no unsuspend");
    assert_eq!(s.game.players[0].security.len(), 2, "no security card trashed");
}

/// "Trash top security card" (1) accepts the gate and takes the matching branch
/// of the engine's cost choice.
#[test]
fn ex13_032_trash_top_security_accepts_and_pays_the_security_cost() {
    let s = run(chirinmon_gate(2, true), vec![menu(1)]);
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert_eq!(s.game.players[0].security.len(), 1, "the top security card was trashed");
    assert!(!chirinmon_suspended(&s.game), "the paid cost unsuspends this Digimon");
}

/// "Trash bottom face down card from 1 tamer" (2) takes the other branch.
#[test]
fn ex13_032_trash_face_down_card_pays_the_tamer_cost() {
    let s = run(chirinmon_gate(2, true), vec![menu(2), row(serde_json::json!({
        "prompt": "SelectPermanentEffect", "targets": [{"player": 0, "frame": 0}]
    }))]);
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert_eq!(s.game.players[0].security.len(), 2, "security untouched");
    assert!(!chirinmon_suspended(&s.game), "the paid cost unsuspends this Digimon");
}

/// Only the security cost is payable: DCGO's menu lists 1 and 3 only, so the 1
/// is forced and accepting the gate is the whole answer.
#[test]
fn ex13_032_security_only_menu_value_one_accepts() {
    let s = run(chirinmon_gate(2, false), vec![menu(1)]);
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete());
    assert_eq!(s.game.players[0].security.len(), 1, "the top security card was trashed");
    assert!(!chirinmon_suspended(&s.game));
}

/// ... and a 2 names a cost nobody can pay (DCGO's menu has no such entry
/// there): no reading is grounded, so the replay stops.
#[test]
fn ex13_032_a_cost_nobody_can_pay_is_refused() {
    let s = run(chirinmon_gate(2, false), vec![menu(2)]);
    assert!(s.is_paused(), "the replay stops");
    let reason = selection_reason(&s);
    assert!(reason.contains("EX13-032"), "{reason}");
    assert_eq!(s.game.players[0].security.len(), 2, "nothing paid");
}

/// Only the face-down cost is payable: 2 accepts, and the Tamer pick that
/// follows is the next row's.
#[test]
fn ex13_032_face_down_only_menu_value_two_accepts() {
    let s = run(
        chirinmon_gate(0, true),
        vec![menu(2), row(serde_json::json!({
            "prompt": "SelectPermanentEffect", "targets": [{"player": 0, "frame": 0}]
        }))],
    );
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete());
    assert!(!chirinmon_suspended(&s.game), "the paid cost unsuspends this Digimon");
}

/// A value that is no constant of Chirinmon's DCGO menu is read as nothing: the
/// replay stops instead of declining or accepting on a guess.
#[test]
fn ex13_032_an_unknown_menu_value_is_refused() {
    let s = run(chirinmon_gate(2, true), vec![menu(7)]);
    assert!(s.is_paused(), "the replay stops");
    let reason = selection_reason(&s);
    assert!(reason.contains("EX13-032"), "{reason}");
    assert!(chirinmon_suspended(&s.game));
    assert_eq!(s.game.players[0].security.len(), 2);
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. AD1-023 + BT18-088: the zone menu against the trigger order
// ═════════════════════════════════════════════════════════════════════════════

/// P0 holds both Tamers; the two [Start of Your Main Phase] effects are queued
/// and parked at the engine's trigger order. `hand` / `trash` are Hybrids.
fn start_of_main(hand: &[&str], trash: &[&str]) -> Game {
    let mut runner = DebugRunner::builder()
        .dsl_card("AD1-023")
        .expect("AD1-023 YAML")
        .dsl_card("BT18-088")
        .expect("BT18-088 YAML")
        .add_card(hybrid("HYB-RED", "Red Hybrid", CardColor::Red))
        .add_card(hybrid("HYB-BLUE", "Blue Hybrid", CardColor::Blue))
        .add_card(make_test_card("MISS", "MISS"))
        .hand(0, hand)
        .deck(0, &["MISS"; 6])
        .deck(1, &["MISS"; 6])
        .memory(10)
        .start();
    for id in trash {
        runner.inject_trash(0, id);
    }
    runner.set_first_player(0);
    runner.skip_mulligan();
    runner.game.enter_main_phase();
    let jp = runner.place_on_field(0, "AD1-023", None);
    let tk = runner.place_on_field(0, "BT18-088", None);
    runner.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(jp));
    runner.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(tk));
    runner.game.drain_effect_queue();
    let p = runner.game.pending_selection.as_ref().expect("the trigger order is parked");
    assert!(matches!(p.kind, digimon_engine::selection::SelectionKind::TriggerOrder), "{:?}", p.kind);
    runner.game
}

/// The recorded shape: no `MultipleSkills` row (DCGO had ONE activatable
/// effect: BT18-088 needs a [Hybrid] card in the trash and the trash is
/// empty), then AD1-023's own menu - From hand = 1, a hand card, Do not place
/// = 3.
#[test]
fn zone_menu_int_meeting_the_trigger_order_chooses_the_effect_that_asks_it() {
    let s = run(
        start_of_main(&["HYB-RED", "HYB-BLUE"], &[]),
        vec![menu(1), hand_pick(&["HYB-RED"]), menu(3)],
    );
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays");
    assert_eq!(under(&s.game, "AD1-023"), vec!["HYB-RED"]);
    assert!(s.game.pending_selection.is_none(), "BT18-088's empty-trash pick left nothing parked");
}

/// With a [Hybrid] card in the trash BT18-088 WAS activatable: DCGO would have
/// asked which of the two effects to resolve first (a `MultipleSkills` row),
/// and the recording has none. The replay cannot tell which effect the menu
/// belongs to and stops, applying nothing.
#[test]
fn zone_menu_int_with_two_really_activatable_effects_is_refused() {
    let s = run(
        start_of_main(&["HYB-RED"], &["HYB-BLUE"]),
        vec![menu(1), hand_pick(&["HYB-RED"]), menu(3)],
    );
    assert!(s.is_paused(), "the replay stops");
    let reason = selection_reason(&s);
    assert!(reason.contains("AD1-023"), "{reason}");
    assert!(
        matches!(
            s.game.pending_selection.as_ref().map(|p| &p.kind),
            Some(digimon_engine::selection::SelectionKind::TriggerOrder)
        ),
        "the trigger order is still parked"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. BT18-097: the membership picks and DCGO's separate order pick
// ═════════════════════════════════════════════════════════════════════════════

fn gunfire(hand: &[&str], trash: &[&str]) -> Game {
    let mut tamer = make_test_card("TAMER", "TAMER");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    // The option needs ALL of its colours on the field (CR 4.3 4-22-3).
    tamer.colors = vec![CardColor::Yellow, CardColor::Purple, CardColor::Black];
    let mut cards: Vec<&str> = vec!["BT18-097"];
    cards.extend_from_slice(hand);
    let mut r = DebugRunner::builder()
        .dsl_card("BT18-097")
        .expect("BT18-097 YAML")
        .add_card(tamer)
        .add_card(hybrid("HYB-A", "Alpha Hybrid", CardColor::Red))
        .add_card(hybrid("HYB-B", "Beta Hybrid", CardColor::Blue))
        .add_card(hybrid("HYB-C", "Gamma Hybrid", CardColor::Yellow))
        .add_card(make_test_card("MISS", "MISS"))
        .hand(0, &cards)
        .deck(0, &["MISS"; 6])
        .deck(1, &["MISS"; 6])
        .memory(10)
        .start();
    for id in trash {
        r.inject_trash(0, id);
    }
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_on_field(0, "TAMER", None);
    r.game
}

fn play_option() -> StepSpec {
    spec(PLAY_HAND_START, "main_phase", None)
}

fn pick_tamer() -> StepSpec {
    row(serde_json::json!({"prompt": "SelectPermanentEffect", "targets": [{"player": 0, "frame": 0}]}))
}

/// The recorded shape: membership pick (hand) of 2, then the order row in the
/// OPPOSITE order. DCGO places "lower numbers on top", so the order row's last
/// card is the very bottom; the engine's pick order is the placement order.
#[test]
fn bt18_097_order_row_sets_the_placement_order() {
    let s = run(
        gunfire(&["HYB-A", "HYB-B"], &[]),
        vec![play_option(), pick_tamer(), hand_pick(&["HYB-A", "HYB-B"]), card_pick(&["HYB-B", "HYB-A"])],
    );
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete(), "every row replays, the order row included");
    assert_eq!(under(&s.game, "TAMER"), vec!["HYB-A", "HYB-B"], "bottom first: the order row's last card is the bottom");
}

/// The same membership with the order row the other way round places the other
/// way round: the pick row's own order says nothing.
#[test]
fn bt18_097_the_membership_rows_order_is_not_the_placement_order() {
    let s = run(
        gunfire(&["HYB-A", "HYB-B"], &[]),
        vec![play_option(), pick_tamer(), hand_pick(&["HYB-A", "HYB-B"]), card_pick(&["HYB-A", "HYB-B"])],
    );
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete());
    assert_eq!(under(&s.game, "TAMER"), vec!["HYB-B", "HYB-A"]);
}

/// Hand pick, then a trash pick (a second membership row), then the order row
/// over the union.
#[test]
fn bt18_097_hand_and_trash_membership_rows_share_one_order_row() {
    let s = run(
        gunfire(&["HYB-A"], &["HYB-B", "HYB-C"]),
        vec![
            play_option(),
            pick_tamer(),
            hand_pick(&["HYB-A"]),
            card_pick(&["HYB-C"]),
            card_pick(&["HYB-C", "HYB-A"]),
        ],
    );
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete());
    assert_eq!(under(&s.game, "TAMER"), vec!["HYB-A", "HYB-C"]);
    let trash: Vec<String> =
        s.game.players[0].trash.iter().map(|c| c.card_id(&s.game.card_data).to_string()).collect();
    assert!(trash.contains(&"HYB-B".to_string()), "HYB-B stayed in the trash: {trash:?}");
    assert!(!trash.contains(&"HYB-C".to_string()), "HYB-C left the trash: {trash:?}");
}

/// An order row that is not a permutation of the picked cards is not the order
/// of those cards: nothing is folded and the replay does not pass it.
#[test]
fn bt18_097_an_order_row_that_is_not_a_permutation_is_not_folded() {
    let s = run(
        gunfire(&["HYB-A", "HYB-B", "HYB-C"], &[]),
        vec![play_option(), pick_tamer(), hand_pick(&["HYB-A", "HYB-B"]), card_pick(&["HYB-B", "HYB-C"])],
    );
    assert!(!s.divergences().is_empty() || !s.is_complete(), "the replay must not pass an unproven order");
    assert!(
        !under(&s.game, "TAMER").contains(&"HYB-C".to_string()),
        "a card outside the membership pick is never placed from the order row"
    );
}

/// One card picked: DCGO asks no order, and the replay still places it.
#[test]
fn bt18_097_a_single_card_needs_no_order_row() {
    let s = run(
        gunfire(&["HYB-A", "HYB-B"], &[]),
        vec![play_option(), pick_tamer(), hand_pick(&["HYB-B"])],
    );
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.is_complete());
    assert_eq!(under(&s.game, "TAMER"), vec!["HYB-B"]);
    let _ = PASS;
}
