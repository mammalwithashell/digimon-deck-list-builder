//! BT6-105 Gewalt Schwärmer — Option, Black, Cost 7. Traits: (none).
//!
//! # Card text (card image / official DB — authoritative)
//!
//! If you have a Digimon with [Three Musketeers] in its type in play, you may
//! use this Option card without meeting its color requirements.
//! [Main] Delete all Digimon with play costs of 7 or less.
//!
//! Inherited (Security):
//! Security Effect [Security] Add this card to its owner's hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT6/Black/BT6_105.cs
//!   - EffectTiming.None `IgnoreColorConditionClass` gated on
//!     `Owner.GetBattleAreaDigimons().Count(TopCard.CardTraits.Contains("Three
//!     Musketeers")) >= 1`, CardCondition `cardSource == card` (this card only)
//!     → top-level `use_requirement` (EX7-066 idiom; the engine's use-time
//!     colour check consults it via `option_color_requirement_bypass`).
//!   - OptionSkill: `Players_ForTurnPlayer.Map(GetBattleAreaDigimons().Filter(
//!     IsDigimon && GetCostItself <= 7 && HasPlayCost))` →
//!     `DestroyPermanentsClass(list).Destroy()` — ONE batched deletion of BOTH
//!     players' matches (tokens have no play cost → excluded).
//!   - SecuritySkill: `AddThisCardToHand`.
//!
//! # Patterns this test covers
//! - D3 colour bypass (`use_requirement`) gated on a [Three Musketeers]
//!   Digimon — positive AND negative, exercised through a live
//!   `play_option_from_hand`
//! - Board-wide batched deletion (`delete_all_permanents`, the step widened for
//!   this card: G-DSL-DELETE-ALL-PERMANENTS) filtered by `play_cost_lte: 7`
//! - [Security] add-self-to-hand (`add_this_option_to_hand`)

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::selection::OptionPlayResult;

const CARD_ID: &str = "BT6-105";

fn digimon(id: &str, color: CardColor, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.colors = vec![color];
    c.play_cost = cost;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT6-105 must load from the embedded DSL pack")
        .add_card(digimon("FILL", CardColor::Black, 3, &[]))
        .add_card(digimon("TM-PURPLE", CardColor::Purple, 6, &["Three Musketeers"]))
        .add_card(digimon("PURPLE-BIG", CardColor::Purple, 12, &[]))
        .add_card(digimon("BLACK-5", CardColor::Black, 5, &[]))
        .add_card(digimon("BLACK-8", CardColor::Black, 8, &[]))
        .add_card(digimon("OPP-7", CardColor::Red, 7, &[]))
        .add_card(digimon("OPP-12", CardColor::Red, 12, &[]))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
}

fn field_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

// ─── Section 1: structural ───────────────────────────────────────────────────

#[test]
fn bt6_105_metadata_and_clauses() {
    let runner = base().start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(compiled.kind, CompiledCardKind::Option);
    assert_eq!(compiled.cost, Some(7));

    assert!(
        compiled.use_requirement.is_some(),
        "'may use this Option card without meeting its color requirements' -> use_requirement"
    );
    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2, "main_from_hand + on_security");
    let main = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::MainFromHand])
        .expect("[Main] clause");
    assert!(!main.optional);
    assert!(triggered.iter().any(|t| t.when == vec![CompiledTiming::OnSecurity]));
}

// ─── Section 2: colour bypass ────────────────────────────────────────────────

#[test]
fn bt6_105_cannot_be_used_without_black_or_a_musketeer() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(0, "PURPLE-BIG", Some(0));
    assert_eq!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "no black permanent and no [Three Musketeers] Digimon → colour requirement unmet"
    );
    assert_eq!(runner.memory(), 10, "nothing was spent");
}

#[test]
fn bt6_105_a_musketeer_digimon_lets_a_non_black_board_use_it() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(0, "TM-PURPLE", Some(0)); // purple, cost 6 — also a deletion target
    runner.place_on_field(0, "PURPLE-BIG", Some(0));
    assert_ne!(
        runner.game.play_option_from_hand(0, 0),
        OptionPlayResult::Invalid,
        "a [Three Musketeers] Digimon waives the colour requirement"
    );
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), 3, "cost 7 paid");
    assert_eq!(field_ids(&runner, 0), vec!["PURPLE-BIG".to_string()], "the cost-6 Musketeer itself is deleted");
}

// ─── Section 3: [Main] delete all play cost ≤ 7 ──────────────────────────────

#[test]
fn bt6_105_main_deletes_every_digimon_with_play_cost_7_or_less_on_both_sides() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(0, "BLACK-5", Some(0));
    runner.place_on_field(0, "BLACK-8", Some(0));
    runner.place_on_field(1, "OPP-7", Some(0));
    runner.place_on_field(1, "OPP-12", Some(0));
    let opp_trash_before = runner.trash_size(1);
    let own_trash_before = runner.trash_size(0);

    let result = runner.game.play_option_from_hand(0, 0);
    assert_ne!(result, OptionPlayResult::Invalid);
    let _ = runner.auto_resolve();

    assert_eq!(field_ids(&runner, 0), vec!["BLACK-8".to_string()], "own cost-5 deleted, cost-8 survives");
    assert_eq!(field_ids(&runner, 1), vec!["OPP-12".to_string()], "opp cost-7 deleted (≤ 7 inclusive), cost-12 survives");
    assert_eq!(runner.trash_size(1), opp_trash_before + 1);
    // Own trash: BLACK-5 + the used Option itself.
    assert_eq!(runner.trash_size(0), own_trash_before + 2);
}

#[test]
fn bt6_105_main_deletes_the_whole_set_as_one_batch() {
    use digimon_engine::events::GameEvent;
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(0, "BLACK-5", Some(0));
    runner.place_on_field(1, "OPP-7", Some(0));
    runner.place_on_field(1, "FILL", Some(0));

    let cp = runner.event_checkpoint();
    let _ = runner.game.play_option_from_hand(0, 0);
    let _ = runner.auto_resolve();

    let trashed: Vec<String> = runner
        .events_since(cp)
        .iter()
        .filter_map(|e| match e {
            GameEvent::Trash { card_id, .. } => Some(card_id.clone()),
            _ => None,
        })
        .collect();
    for id in ["BLACK-5", "OPP-7", "FILL"] {
        assert!(trashed.iter().any(|t| t == id), "{id} must be trashed by the [Main]; trashed = {trashed:?}");
    }
    assert!(runner.game.players[0].battle_area.is_empty());
    assert!(runner.game.players[1].battle_area.is_empty());
}

#[test]
fn bt6_105_main_with_no_eligible_digimon_deletes_nothing() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.place_on_field(0, "BLACK-8", Some(0));
    runner.place_on_field(1, "OPP-12", Some(0));
    let _ = runner.game.play_option_from_hand(0, 0);
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), 1);
    assert_eq!(runner.battle_area_size(1), 1);
}

// ─── Section 4: [Security] add this card to hand ─────────────────────────────

#[test]
fn bt6_105_security_adds_itself_to_the_owners_hand() {
    let mut attacker = make_test_card("ATK", "ATK");
    attacker.dp = Some(5000);
    let mut runner = base()
        .add_card(attacker)
        .security(1, &[CARD_ID])
        .memory(3)
        .start();
    let atk = runner.place_on_field(0, "ATK", Some(0));
    let hand_before = runner.hand_size(1);

    let _ = runner.attack_player(atk, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(runner.security_count(1), 0);
    assert_eq!(runner.hand_size(1), hand_before + 1);
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "Gewalt Schwärmer went to P1's hand, not the trash"
    );
    assert_eq!(runner.trash_size(1), 0);
}
