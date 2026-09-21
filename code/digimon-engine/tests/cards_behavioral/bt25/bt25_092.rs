//! BT25-092 Asuna Shiroki — Tamer, Purple, Cost 4, [TS].
//!
//! # Card text (cards.json)
//! [Start of Your Main Phase] By trashing 1 card with [Three Musketeers] in its
//!   text or the [TS] trait from your hand, <Draw 1> and gain 1 memory.
//! [Main] By suspending this Tamer and trashing 1 Option card from your hand or
//!   your Digimon's digivolution cards, 1 of your Digimon may digivolve into a
//!   Digimon card with [Three Musketeers] in its text or the [TS] trait in the
//!   hand or trash with the cost reduced by 1.
//! [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Purple/BT25_092.cs
//!
//! # Patterns this test covers
//! - B1 start-of-main tamer (trash-as-cost → draw + memory)
//! - Tamer [Security] play-self
//! - [Main] suspend-self + trash-Option-from-{hand ∪ own Digimon's
//!   digivolution cards} cost → 1 own Digimon may digivolve into a
//!   [Three Musketeers]-text / [TS] Digimon card from {hand ∪ trash}, cost −1
//!   (G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST — RESOLVED: the
//!   `can_digivolve_onto: <binding>` card leaf + the `has_digivolve_candidate`
//!   permanent leaf close DCGO's `ValidTarget` / `CanDigivolveDigimon` gates).
//!
//! DCGO BT25_092.cs prompt order (Main): suspend → [zone bool] → trash pick
//! (mandatory) → permanent pick (optional) → [zone bool] → result pick
//! (optional) → `DigivolveIntoHandOrTrashCard(payCost, reduce 1)`. Our
//! union prompts fold each DCGO zone bool + per-zone pick into one prompt.

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::action::space::{encode_attack, encode_source_select, PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

const YAML: &str = include_str!("../../../cards/bt25/BT25-092.yaml");

// ── Section 1: structural ────────────────────────────────────────────────

#[test]
fn bt25_092_structure_start_of_main_and_security() {
    let runner = asuna_runner().start();
    let compiled = runner
        .compiled_card("BT25-092")
        .expect("BT25-092 compiled card present");

    assert_eq!(compiled.card, "BT25-092");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
    assert_eq!(compiled.cost, Some(4));
    assert!(compiled.traits.iter().any(|t| t == "TS"));

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let start_main = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::StartOfYourMainPhase])
        .expect("start-of-your-main-phase clause present");
    assert!(
        start_main.optional,
        "the trash-as-cost clause is optional (\"by trashing\")"
    );

    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSecurity]),
        "security play-self clause present"
    );

    let main = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::MainOnField])
        .expect("[Main] digivolve clause present (gap closed)");
    assert!(
        !main.optional,
        "[Main] activation is the opt-in (DCGO SetUpActivateClass optional=false)"
    );
    assert!(
        !main.once_per_turn,
        "DCGO maxCountPerTurn -1 → unlimited activations per turn"
    );
}

// ── Section 3: [Main] suspend + trash Option (hand ∪ sources) → digivolve ──
//
// Fixture (all player 0):
//   BT25-092 on field (unsuspended) · BASE (Lv.4 purple Digimon on field)
//   EVO-HAND  Lv.5 purple [TS] Digimon in hand, circle "purple Lv.4: 3"
//   EVO-TRASH Lv.5 purple Digimon in trash, "Three Musketeers" in text, same circle
//   EVO-LV6   Lv.6 purple [TS] Digimon in hand, circle "purple Lv.5: 4" (no route)
//   PLAIN-EVO Lv.5 purple Digimon in hand, no TS / no TM text (filtered out)
//   OPT-HAND  Option in hand · OPT-UNDER Option buried under BASE

fn main_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    asuna_runner()
        .add_card(make_base("BASE"))
        .add_card(make_evo_ts("EVO-HAND"))
        .add_card(make_evo_tm_text("EVO-TRASH"))
        .add_card(make_evo_lv6_ts("EVO-LV6"))
        .add_card(make_evo_plain("PLAIN-EVO"))
        .add_card(make_option("OPT-HAND"))
        .add_card(make_option("OPT-UNDER"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 6])
}

/// Enter the main phase and decline the Tamer's own `[Start of Your Main
/// Phase]` optional prompt when it installs (it fires whenever a [TS] /
/// [Three Musketeers]-text card sits in hand — the EVO-* fixtures do).
fn enter_main(runner: &mut DebugRunner) {
    runner.game.enter_main_phase();
    if runner.pending_selection().is_some() {
        assert!(runner.pending_is_optional(), "only the start-of-main optional prompt is expected");
        runner
            .decline_optional_trigger()
            .expect("decline the [Start of Your Main Phase] prompt");
    }
    assert!(runner.pending_selection().is_none());
}

/// Activate the Tamer's `[Main]` through the real action path
/// (`Game::activate_field_main`, the FIELD_EFFECT +2 sub-slot decoder target)
/// so the clause-level `condition` and the suspend cost are exercised.
fn activate_main(runner: &mut DebugRunner, tamer_index: usize) -> bool {
    runner.game.activate_field_main(0, tamer_index)
}

fn field_top_id(runner: &DebugRunner, index: usize) -> String {
    let perm = &runner.game.player(0).battle_area[index];
    perm.top_card().card_id(&runner.game.card_data).to_string()
}

fn trash_has(runner: &DebugRunner, id: &str) -> bool {
    runner
        .game
        .player(0)
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == id)
}

fn hand_index_of(runner: &DebugRunner, id: &str) -> usize {
    runner
        .game
        .player(0)
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in hand"))
}

fn trash_index_of(runner: &DebugRunner, id: &str) -> usize {
    runner
        .game
        .player(0)
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in trash"))
}

#[test]
fn bt25_092_main_not_offered_when_tamer_suspended() {
    let mut runner = main_runner().hand(0, &["OPT-HAND"]).memory(5).start();
    let tamer = runner.place_on_field(0, "BT25-092", Some(0));
    enter_main(&mut runner);
    runner.game.suspend(tamer);

    assert!(
        !activate_main(&mut runner, tamer.index as usize),
        "suspended Tamer cannot pay the suspend cost (DCGO CanActivateSuspendCostEffect)"
    );
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.hand_size(0), 1, "no cost was taken");
}

#[test]
fn bt25_092_main_not_offered_without_any_option_to_trash() {
    // Unsuspended Tamer, but no Option in hand and no Option under any Digimon.
    let mut runner = main_runner().hand(0, &["EVO-HAND"]).memory(5).start();
    let tamer = runner.place_on_field(0, "BT25-092", Some(0));
    let _base = runner.place_on_field(0, "BASE", Some(0));
    enter_main(&mut runner);

    assert!(
        !activate_main(&mut runner, tamer.index as usize),
        "no Option in hand or digivolution cards → CanUseCondition false"
    );
    assert!(
        !runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "Tamer not suspended when the activation is refused"
    );
}

#[test]
fn bt25_092_main_cost_from_hand_then_digivolve_from_hand_cost_reduced() {
    let mut runner = main_runner()
        .hand(0, &["OPT-HAND", "EVO-HAND", "PLAIN-EVO", "EVO-LV6"])
        .memory(5)
        .start();
    let tamer = runner.place_on_field(0, "BT25-092", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.inject_trash(0, "EVO-TRASH");
    enter_main(&mut runner);
    let deck_before = runner.deck_size(0);

    assert!(activate_main(&mut runner, tamer.index as usize));
    assert!(
        runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "suspend cost paid first"
    );

    // Cost prompt: ONE union prompt over hand ∪ digivolution cards. Only the
    // Option in hand qualifies here (BASE has no sources) — mandatory.
    let view = runner.pending_selection_view().expect("cost union prompt");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { zones } if zones.contains(UnionZoneSet::HAND) && zones.contains(UnionZoneSet::MATERIAL)),
        "cost prompt spans hand ∪ material, got {:?}",
        view.kind
    );
    assert!(!view.is_optional, "the Option trash is a mandatory cost (DCGO canNoSelect false)");
    let opt_action = PLAY_HAND_START + hand_index_of(&runner, "OPT-HAND") as u16;
    assert_eq!(view.valid_action_ids, vec![opt_action], "only the Option card is trashable");
    runner.execute_action(0, opt_action).expect("trash the Option from hand");
    assert!(trash_has(&runner, "OPT-HAND"), "Option trashed as cost");

    // Target prompt: optional own-Digimon pick (DCGO canNoSelect TRUE),
    // restricted to Digimon with a legal result card.
    let view = runner.pending_selection_view().expect("own-Digimon target prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.is_optional, "\"may digivolve\" → declinable target pick");
    let base_action = encode_attack(0, base.index as u16);
    assert_eq!(
        view.valid_action_ids,
        vec![base_action],
        "only BASE is offered (the Tamer is not a Digimon)"
    );
    runner.execute_action(0, base_action).expect("pick BASE");

    // Result prompt: ONE union prompt over hand ∪ trash. EVO-HAND (hand, TS)
    // and EVO-TRASH (trash, TM text) qualify; PLAIN-EVO (no TS / TM text)
    // and EVO-LV6 (no route onto a Lv.4) are filtered out.
    let view = runner.pending_selection_view().expect("result union prompt");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { zones } if zones == (UnionZoneSet::HAND | UnionZoneSet::TRASH)),
        "result prompt spans hand ∪ trash, got {:?}",
        view.kind
    );
    assert!(view.is_optional, "result pick is declinable (DCGO isOptional)");
    let evo_hand_action = PLAY_HAND_START + hand_index_of(&runner, "EVO-HAND") as u16;
    let evo_trash_action = TRASH_EFFECT_START + trash_index_of(&runner, "EVO-TRASH") as u16;
    let mut offered = view.valid_action_ids.clone();
    offered.sort_unstable();
    let mut expected = vec![evo_hand_action, evo_trash_action];
    expected.sort_unstable();
    assert_eq!(offered, expected, "exactly the routable [TS]/[Three Musketeers] cards are offered");

    let memory_before = runner.memory();
    runner.execute_action(0, evo_hand_action).expect("digivolve into EVO-HAND");
    let _ = runner.auto_resolve();

    assert_eq!(field_top_id(&runner, base.index as usize), "EVO-HAND");
    assert_eq!(
        runner.memory(),
        memory_before - 2,
        "printed circle cost 3 reduced by 1 → paid 2"
    );
    assert_eq!(runner.deck_size(0), deck_before - 1, "digivolution draw (§8-1-3-3)");
    assert!(runner.pending_selection().is_none());
}

#[test]
fn bt25_092_main_cost_from_digivolution_cards_of_any_own_digimon() {
    // No Option in hand; OPT-UNDER is buried under BASE → the cost union
    // prompt offers the source-select action for it (single origin, no
    // zone-choice), and the trash fires from the digivolution stack.
    let mut runner = main_runner().hand(0, &["EVO-HAND"]).memory(5).start();
    let tamer = runner.place_on_field(0, "BT25-092", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.push_source(base, "OPT-UNDER");
    let sources_before = runner.game.player(0).battle_area[base.index as usize]
        .card_sources
        .len();
    enter_main(&mut runner);

    assert!(activate_main(&mut runner, tamer.index as usize));
    let view = runner.pending_selection_view().expect("cost union prompt");
    let source_action = encode_source_select(base.index as u16, 0).expect("encodes");
    assert_eq!(
        view.valid_action_ids,
        vec![source_action],
        "only the buried Option (BASE source 0) is offered; the hand holds no Option"
    );
    runner.execute_action(0, source_action).expect("trash the buried Option");
    assert!(trash_has(&runner, "OPT-UNDER"));
    assert_eq!(
        runner.game.player(0).battle_area[base.index as usize]
            .card_sources
            .len(),
        sources_before - 1,
        "the Option left BASE's digivolution cards"
    );

    // Digivolve still proceeds (BASE remains a legal Lv.4 base).
    let view = runner.pending_selection_view().expect("target prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    runner
        .execute_action(0, encode_attack(0, base.index as u16))
        .expect("pick BASE");
    let view = runner.pending_selection_view().expect("result prompt");
    let evo_action = PLAY_HAND_START + hand_index_of(&runner, "EVO-HAND") as u16;
    assert_eq!(view.valid_action_ids, vec![evo_action]);
    runner.execute_action(0, evo_action).expect("digivolve");
    let _ = runner.auto_resolve();
    assert_eq!(field_top_id(&runner, base.index as usize), "EVO-HAND");
}

#[test]
fn bt25_092_main_cost_union_offers_both_hand_and_sources_in_one_prompt() {
    let mut runner = main_runner().hand(0, &["OPT-HAND", "EVO-HAND"]).memory(5).start();
    let tamer = runner.place_on_field(0, "BT25-092", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.push_source(base, "OPT-UNDER");
    enter_main(&mut runner);

    assert!(activate_main(&mut runner, tamer.index as usize));
    let view = runner.pending_selection_view().expect("cost union prompt");
    let mut offered = view.valid_action_ids.clone();
    offered.sort_unstable();
    let mut expected = vec![
        PLAY_HAND_START + hand_index_of(&runner, "OPT-HAND") as u16,
        encode_source_select(base.index as u16, 0).expect("encodes"),
    ];
    expected.sort_unstable();
    assert_eq!(
        offered, expected,
        "DCGO's From-hand / From-digivolution-cards bool + pick fold into one union prompt"
    );
}

#[test]
fn bt25_092_main_digivolve_into_card_resident_in_trash() {
    let mut runner = main_runner().hand(0, &["OPT-HAND"]).memory(5).start();
    let tamer = runner.place_on_field(0, "BT25-092", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    runner.inject_trash(0, "EVO-TRASH");
    enter_main(&mut runner);

    assert!(activate_main(&mut runner, tamer.index as usize));
    let opt_action = PLAY_HAND_START + hand_index_of(&runner, "OPT-HAND") as u16;
    runner.execute_action(0, opt_action).expect("trash Option");
    runner
        .execute_action(0, encode_attack(0, base.index as u16))
        .expect("pick BASE");

    let view = runner.pending_selection_view().expect("result prompt");
    let evo_trash_action = TRASH_EFFECT_START + trash_index_of(&runner, "EVO-TRASH") as u16;
    assert_eq!(
        view.valid_action_ids,
        vec![evo_trash_action],
        "only the trash-resident [Three Musketeers]-text card is offered"
    );
    let memory_before = runner.memory();
    runner.execute_action(0, evo_trash_action).expect("digivolve from trash");
    let _ = runner.auto_resolve();

    assert_eq!(field_top_id(&runner, base.index as usize), "EVO-TRASH");
    assert!(!trash_has(&runner, "EVO-TRASH"), "the card left the trash");
    assert_eq!(runner.memory(), memory_before - 2, "cost 3 − 1");
}

#[test]
fn bt25_092_main_target_pick_excludes_digimon_without_a_legal_result() {
    // BASE is Lv.4; the only result card is EVO-LV6 (needs a Lv.5 base) →
    // BASE has no digivolve candidate → after the cost the clause ends with
    // no target prompt (DCGO's `if HasMatchConditionOwnersPermanent(...)`).
    let mut runner = main_runner().hand(0, &["OPT-HAND", "EVO-LV6"]).memory(5).start();
    let tamer = runner.place_on_field(0, "BT25-092", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    enter_main(&mut runner);

    assert!(activate_main(&mut runner, tamer.index as usize));
    let opt_action = PLAY_HAND_START + hand_index_of(&runner, "OPT-HAND") as u16;
    runner.execute_action(0, opt_action).expect("trash Option");

    assert!(
        runner.pending_selection().is_none(),
        "no Digimon with a legal result card → no target prompt"
    );
    assert!(trash_has(&runner, "OPT-HAND"), "cost stays paid");
    assert!(runner.game.player(0).battle_area[tamer.index as usize].is_suspended);
    assert_eq!(field_top_id(&runner, base.index as usize), "BASE");
}

#[test]
fn bt25_092_main_decline_target_keeps_cost_paid_and_does_not_digivolve() {
    let mut runner = main_runner().hand(0, &["OPT-HAND", "EVO-HAND"]).memory(5).start();
    let tamer = runner.place_on_field(0, "BT25-092", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    enter_main(&mut runner);
    let memory_before = runner.memory();

    assert!(activate_main(&mut runner, tamer.index as usize));
    let opt_action = PLAY_HAND_START + hand_index_of(&runner, "OPT-HAND") as u16;
    runner.execute_action(0, opt_action).expect("trash Option");

    let view = runner.pending_selection_view().expect("target prompt");
    assert!(view.is_optional);
    runner.execute_action(0, PASS).expect("decline the digivolve");

    assert!(runner.pending_selection().is_none(), "declining ends the clause");
    assert_eq!(field_top_id(&runner, base.index as usize), "BASE");
    assert_eq!(runner.memory(), memory_before, "no digivolution cost paid");
    assert!(trash_has(&runner, "OPT-HAND"), "the trash cost is not refunded");
    assert!(runner.game.player(0).battle_area[tamer.index as usize].is_suspended);
}

#[test]
fn bt25_092_main_decline_result_pick_after_target() {
    let mut runner = main_runner().hand(0, &["OPT-HAND", "EVO-HAND"]).memory(5).start();
    let tamer = runner.place_on_field(0, "BT25-092", Some(0));
    let base = runner.place_on_field(0, "BASE", Some(0));
    enter_main(&mut runner);
    let memory_before = runner.memory();

    assert!(activate_main(&mut runner, tamer.index as usize));
    let opt_action = PLAY_HAND_START + hand_index_of(&runner, "OPT-HAND") as u16;
    runner.execute_action(0, opt_action).expect("trash Option");
    runner
        .execute_action(0, encode_attack(0, base.index as u16))
        .expect("pick BASE");

    let view = runner.pending_selection_view().expect("result prompt");
    assert!(view.is_optional);
    runner.execute_action(0, PASS).expect("decline the result pick");

    assert!(runner.pending_selection().is_none());
    assert_eq!(field_top_id(&runner, base.index as usize), "BASE");
    assert_eq!(runner.memory(), memory_before);
    assert_eq!(runner.hand_size(0), 1, "EVO-HAND stays in hand");
}

// ── Section 2: [Start of Your Main Phase] trash → draw + memory ──────────

#[test]
fn bt25_092_start_of_main_trashes_ts_card_draws_and_gains_memory() {
    let mut runner = asuna_runner()
        .hand(0, &["TS-CARD"])
        .add_card(make_ts_hand_card("TS-CARD"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .memory(3)
        .start();
    runner.place_on_field(0, "BT25-092", Some(0));

    let trash_before = runner.trash_size(0);
    let deck_before = runner.deck_size(0);
    let memory_before = runner.memory();

    runner.game.enter_main_phase();

    // The clause is optional ("by trashing …"), so an accept/decline prompt
    // installs first; accept it.
    let opt = runner
        .pending_selection_view()
        .expect("[Start of Main] optional-activation prompt installs");
    assert_eq!(opt.kind, SelectionKind::Replacement);
    assert!(runner.pending_is_optional());
    runner
        .execute_action(opt.selecting_player, opt.valid_action_ids[0])
        .expect("accept the optional cost");

    // Then the trash-as-cost select installs over the TS card in hand.
    let view = runner
        .pending_selection_view()
        .expect("trash select installs after accepting");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("trash the TS card as cost");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "TS card trashed as cost"
    );
    assert_eq!(runner.deck_size(0), deck_before - 1, "Draw 1 fired");
    assert_eq!(runner.memory(), memory_before + 1, "gained 1 memory");
}

#[test]
fn bt25_092_start_of_main_no_cost_card_does_not_fire() {
    // Hand holds only a non-TS, non-Three-Musketeers card → condition fails →
    // no selection installs, no draw, no memory gain.
    let mut runner = asuna_runner()
        .hand(0, &["PLAIN"])
        .add_card(make_plain_hand_card("PLAIN"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .memory(3)
        .start();
    runner.place_on_field(0, "BT25-092", Some(0));

    let deck_before = runner.deck_size(0);
    let memory_before = runner.memory();
    runner.game.enter_main_phase();

    assert!(
        runner.pending_selection().is_none(),
        "no eligible cost card → clause does not install"
    );
    assert_eq!(runner.deck_size(0), deck_before, "no draw");
    assert_eq!(runner.memory(), memory_before, "no memory gain");
}

// ── fixtures ─────────────────────────────────────────────────────────────

fn asuna_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT25-092 YAML loads")
}

fn make_ts_hand_card(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card.traits = vec!["TS".to_string()];
    card
}

fn make_plain_hand_card(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card
}

fn make_filler(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(3);
    card.dp = Some(3000);
    card.play_cost = 3;
    card
}

/// Lv.4 purple Digimon — the digivolve base on the field.
fn make_base(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(4);
    card.dp = Some(4000);
    card.play_cost = 4;
    card
}

/// Lv.5 purple Digimon with a printed "purple Lv.4: cost 3" circle.
fn make_lv5_purple(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(5);
    card.dp = Some(7000);
    card.play_cost = 8;
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Purple as u8,
        level: 4,
        memory_cost: 3,
    }];
    card
}

fn make_evo_ts(id: &str) -> digimon_engine::CardData {
    let mut card = make_lv5_purple(id);
    card.traits = vec!["TS".to_string()];
    card
}

fn make_evo_tm_text(id: &str) -> digimon_engine::CardData {
    let mut card = make_lv5_purple(id);
    card.effect_text = "[When Digivolving] 1 of your [Three Musketeers] Digimon gets +1000 DP.".to_string();
    card
}

fn make_evo_plain(id: &str) -> digimon_engine::CardData {
    make_lv5_purple(id)
}

/// Lv.6 purple [TS] Digimon whose only circle needs a Lv.5 base — never
/// routable onto the Lv.4 BASE.
fn make_evo_lv6_ts(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(6);
    card.dp = Some(11000);
    card.play_cost = 12;
    card.traits = vec!["TS".to_string()];
    card.evo_costs = vec![EvoCost {
        card_color: CardColor::Purple as u8,
        level: 5,
        memory_cost: 4,
    }];
    card
}

fn make_option(id: &str) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.colors = vec![CardColor::Purple];
    card.level = None;
    card.dp = None;
    card.play_cost = 2;
    card
}
