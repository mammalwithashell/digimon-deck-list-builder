//! BT24-081 Titamon + SkullBaluchimon — Digimon, Lv.7, Purple/Green,
//! DP 14000, Cost 14. Traits: Shaman / Titan / TS (+ Rule: Has [Demon] Type).
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/BT24-081.md`;
//! card image confirms)
//!
//! ＜Rush＞ ＜Piercing＞ ＜Execute＞
//! [On Play] [When Digivolving] [When Attacking] By trashing 1 card in your
//! hand, delete all of your opponent's Digimon with the lowest level.
//! [On Deletion] You may play 1 [Titamon] or 1 level 5 or lower Digimon card
//! with the [Titan] trait from your trash without paying the cost.
//! (Rule) Trait: Has [Demon] Type.
//! Assembly -6: [Titamon]×[SkullBaluchimon]
//!
//! Digivolve: Purple Lv.6 / cost 4; Green Lv.6 / cost 4.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Purple/BT24_081.cs
//! - `ExecuteSelfEffect`, `PierceSelfEffect`, `RushSelfStaticEffect`.
//! - `AddAssemblyConditionClass`: elements [Titamon]×1 + [SkullBaluchimon]×1
//!   (own Digimon cards, `EqualsCardName`), `reduceCost: 6`.
//! - Shared [OP]/[WD]/[WA]: `SetUpActivateClass(..., -1, false, ...)`,
//!   `CanActivateCondition` = ≥1 hand card; body = `SelectHandEffect
//!   Mode.Discard` (max 1, `canNoSelect: true`); if a card was trashed and a
//!   min-level opponent Digimon exists → `DestroyPermanentsClass` over ALL
//!   opponent Digimon with `IsMinLevel`.
//! - [On Deletion]: `SetUpActivateClass(..., -1, false, ...)` gated on an
//!   eligible trash card (IsDigimon && (name "Titamon" || (trait "Titan" &&
//!   Level ≤ 5)) && CanPlayAsNewPermanent); `SelectCardEffect` over own trash
//!   (`canNoSelect: true`) → `PlayPermanentCards(payCost: false)`.
//!
//! # Patterns this test covers
//! - H1 Rush / H3 Piercing / Execute as printed face-up keywords.
//! - Assembly alt-play with two named trash materials (AD1-025 idiom).
//! - Trash-a-hand-card cost ("By trashing …") + mass delete over an aggregate
//!   (`for_each` over `level_matches_aggregate: lowest_level`).
//! - F5-adjacent [On Deletion] optional free play from trash (BT19-065 idiom).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming, CompiledZone,
};
use digimon_engine::action::space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "BT24-081";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3 + level as u16;
    c.colors = vec![CardColor::Purple];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-081 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("TITAMON", "Titamon", 6, 12000, &["Titan"]))
        .add_card(digimon("SKULLBALU", "SkullBaluchimon", 6, 11000, &["Undead"]))
        .add_card(digimon("TITAN-L5", "Titan Five", 5, 7000, &["Titan"]))
        .add_card(digimon("TITAN-L6", "Titan Six", 6, 11000, &["Titan"]))
        .add_card(digimon("PLAIN-L4", "Plain Four", 4, 4000, &["Beast"]))
        .add_card(digimon("OPP-L3-A", "Opp Three A", 3, 2000, &["Beast"]))
        .add_card(digimon("OPP-L3-B", "Opp Three B", 3, 2000, &["Beast"]))
        .add_card(digimon("OPP-L5", "Opp Five", 5, 7000, &["Beast"]))
        .add_card(digimon("HAND-X", "Hand X", 3, 2000, &["Beast"]))
        .add_card(digimon("HAND-Y", "Hand Y", 3, 2000, &["Beast"]))
        .add_card(digimon("FILL", "Fill", 3, 2000, &["Beast"]))
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn trash_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s trash"))
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn stack_ids(runner: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn non_pass_ids(runner: &DebugRunner) -> Vec<u16> {
    runner
        .pending_selection_view()
        .expect("a prompt must be pending")
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect()
}

fn pick_first(runner: &mut DebugRunner, label: &str) {
    let view = runner
        .pending_selection_view()
        .unwrap_or_else(|| panic!("{label}: a prompt must be pending"));
    let id = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .unwrap_or_else(|| panic!("{label}: a non-PASS pick must exist"));
    runner
        .execute_action(view.selecting_player, id)
        .unwrap_or_else(|e| panic!("{label}: pick failed: {e:?}"));
}

fn decline(runner: &mut DebugRunner) {
    assert!(runner.pending_is_optional(), "PASS must be legal");
    let player = runner
        .pending_selection()
        .expect("prompt")
        .selecting_player;
    runner.execute_action(player, PASS).expect("decline");
    let _ = runner.auto_resolve();
}

fn play_titamon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.skip_mulligan();
    let idx = hand_index(runner, 0, CARD_ID);
    let field = runner.play(0, idx).expect("Titamon + SkullBaluchimon plays");
    PermanentHandle {
        player: 0,
        index: field as u8,
    }
}

fn opp_board(runner: &mut DebugRunner) {
    runner.place_on_field(1, "OPP-L3-A", Some(0));
    runner.place_on_field(1, "OPP-L5", Some(0));
    runner.place_on_field(1, "OPP-L3-B", Some(0));
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn bt24_081_metadata_matches_printed_card_including_rule_trait() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.name, "Titamon + SkullBaluchimon");
    assert_eq!(card.level, Some(7));
    assert_eq!(card.cost, Some(14));
    assert_eq!(card.dp, Some(14000));
    assert_eq!(card.color, vec![CompiledColor::Purple, CompiledColor::Green]);
    for t in ["Shaman", "Titan", "TS", "Demon"] {
        assert!(card.traits.iter().any(|x| x == t), "trait {t} (Demon via (Rule))");
    }
}

#[test]
fn bt24_081_has_face_up_rush_piercing_and_execute_grants() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    let mut keywords: Vec<&str> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) => {
                assert_eq!(*scope, CompiledScope::FaceUp, "{keyword} is a printed face-up keyword");
                Some(keyword.as_str())
            }
            _ => None,
        })
        .collect();
    keywords.sort();
    assert_eq!(keywords, vec!["Execute", "Piercing", "Rush"]);
}

#[test]
fn bt24_081_has_two_printed_circles_and_the_named_assembly() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    let circles: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(circles.len(), 2, "Purple Lv.6/4 + Green Lv.6/4");
    for c in &circles {
        assert_eq!(c.cost, Some(CompiledCost::Literal(4)));
        assert_eq!(c.from.as_ref().and_then(|f| f.level_eq), Some(6));
    }

    let assembly = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly -6 alt-path");
    assert_eq!(assembly.cost, Some(CompiledCost::Literal(6)), "Assembly -6 (cost reduction)");
    assert_eq!(assembly.materials.len(), 2, "[Titamon] × [SkullBaluchimon]");
    let names: Vec<Option<&str>> = assembly
        .materials
        .iter()
        .map(|m| m.filter.name_is.as_deref())
        .collect();
    assert!(names.contains(&Some("Titamon")));
    assert!(names.contains(&Some("SkullBaluchimon")));
    for m in &assembly.materials {
        assert!(m.stack_under, "Assembly materials go under the played card");
        assert!(m.zones.contains(&CompiledZone::Trash), "Assembly materials come from the trash (7-3-1)");
    }
}

#[test]
fn bt24_081_has_shared_op_wd_wa_cost_clause_and_optional_on_deletion() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2);

    let shared = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("shared clause");
    assert_eq!(
        shared.when,
        vec![
            CompiledTiming::OnPlay,
            CompiledTiming::WhenDigivolving,
            CompiledTiming::WhenAttacking
        ]
    );
    assert!(shared.optional, "'By trashing 1 card' is an optional processing condition");
    assert!(shared.condition.is_some(), "gated on having a hand card to trash");
    assert!(!shared.once_per_turn);

    let od = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnDeletion])
        .expect("[On Deletion] clause");
    assert!(od.optional, "'You may play'");
    assert!(od.condition.is_some(), "gated on an eligible trash card");
    for t in &triggered {
        assert_eq!(t.scope, CompiledScope::FaceUp);
    }
}

// ─── Section 3 — printed keywords live on the carrier ────────────────────────

#[test]
fn bt24_081_carrier_has_rush_piercing_and_execute() {
    let mut runner = builder().start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(runner.game.has_keyword(perm, Keyword::Rush));
    assert!(runner.game.has_keyword(perm, Keyword::Piercing));
    assert!(runner.game.has_keyword(perm, Keyword::Execute));
}

// ─── Section 2 / 3 — [OP][WD][WA] trash 1 → delete all lowest-level ──────────

#[test]
fn bt24_081_on_play_trashing_a_hand_card_deletes_every_lowest_level_opponent_digimon() {
    let mut runner = builder()
        .hand(0, &[CARD_ID, "HAND-X", "HAND-Y"])
        .memory(15)
        .start();
    opp_board(&mut runner);

    play_titamon(&mut runner);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand), "pick the hand card to trash");
    assert!(runner.pending_is_optional(), "the cost may be declined");
    assert_eq!(non_pass_ids(&runner).len(), 2, "HAND-X and HAND-Y are both trashable");
    pick_first(&mut runner, "trash a hand card");
    let _ = runner.auto_resolve();

    assert_eq!(runner.hand_size(0), 1, "one hand card trashed");
    assert_eq!(runner.trash_size(0), 1);
    assert_eq!(
        field_ids(&runner, 1),
        vec!["OPP-L5".to_string()],
        "BOTH level-3 Digimon (the lowest level) are deleted; the Lv.5 survives"
    );
    assert_eq!(runner.trash_size(1), 2, "the deleted Digimon are in the opponent's trash");
}

#[test]
fn bt24_081_on_play_declining_the_trash_cost_deletes_nothing() {
    let mut runner = builder()
        .hand(0, &[CARD_ID, "HAND-X", "HAND-Y"])
        .memory(15)
        .start();
    opp_board(&mut runner);

    play_titamon(&mut runner);
    decline(&mut runner);

    assert_eq!(runner.hand_size(0), 2, "no card trashed");
    assert_eq!(runner.battle_area_size(1), 3, "no deletion without paying the cost");
}

#[test]
fn bt24_081_on_play_with_an_empty_hand_installs_no_prompt() {
    // NEGATIVE: nothing to trash → the cost is unpayable → no prompt.
    let mut runner = builder().hand(0, &[CARD_ID]).memory(15).start();
    opp_board(&mut runner);

    play_titamon(&mut runner);

    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.battle_area_size(1), 3);
}

#[test]
fn bt24_081_on_play_with_no_opponent_digimon_still_lets_you_trash() {
    // The cost is a legal action even with nothing to delete (DCGO trashes,
    // then finds no min-level target).
    let mut runner = builder().hand(0, &[CARD_ID, "HAND-X"]).memory(15).start();

    play_titamon(&mut runner);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    pick_first(&mut runner, "trash");
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), 0);
    assert_eq!(runner.trash_size(0), 1);
}

#[test]
fn bt24_081_when_digivolving_fires_the_same_clause() {
    let mut runner = builder().hand(0, &["HAND-X"]).memory(15).start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    opp_board(&mut runner);

    fire(&mut runner, EffectTiming::WhenDigivolving, perm);

    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    pick_first(&mut runner, "trash");
    let _ = runner.auto_resolve();
    assert_eq!(field_ids(&runner, 1), vec!["OPP-L5".to_string()]);
}

#[test]
fn bt24_081_when_attacking_fires_the_same_clause() {
    let mut runner = builder()
        .hand(0, &["HAND-X"])
        .security(1, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    opp_board(&mut runner);

    runner.attack_player(perm, 1, false);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "[When Attacking] offers the trash cost"
    );
    pick_first(&mut runner, "trash");
    let _ = runner.auto_resolve();
    assert!(
        !field_ids(&runner, 1).iter().any(|c| c.starts_with("OPP-L3")),
        "both lowest-level opponent Digimon are gone: {:?}",
        field_ids(&runner, 1)
    );
}

// ─── Section 2 / 3 — [On Deletion] free play from trash ──────────────────────

fn deletion_setup(trash: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder().memory(15).start();
    let perm = runner.place_on_field(0, CARD_ID, Some(0));
    for id in trash {
        runner.inject_trash(0, id);
    }
    (runner, perm)
}

#[test]
fn bt24_081_on_deletion_offers_titamon_and_low_level_titans_only() {
    let (mut runner, perm) = deletion_setup(&["PLAIN-L4", "TITAMON", "TITAN-L6", "TITAN-L5"]);

    runner
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);

    assert!(runner.pending_is_optional(), "'You may play' — outer accept/decline");
    runner.accept_optional_trigger().expect("accept");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Trash));
    let ids = non_pass_ids(&runner);
    let titamon = TRASH_EFFECT_START + trash_index(&runner, 0, "TITAMON") as u16;
    let titan5 = TRASH_EFFECT_START + trash_index(&runner, 0, "TITAN-L5") as u16;
    assert_eq!(ids.len(), 2, "exactly [Titamon] and the Lv.5 [Titan]: {ids:?}");
    assert!(ids.contains(&titamon));
    assert!(ids.contains(&titan5));
}

#[test]
fn bt24_081_on_deletion_accepting_plays_the_chosen_card_free() {
    let (mut runner, perm) = deletion_setup(&["TITAMON", "TITAN-L5"]);
    let memory_before = runner.memory();

    runner
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);
    runner.accept_optional_trigger().expect("accept");
    let titamon = TRASH_EFFECT_START + trash_index(&runner, 0, "TITAMON") as u16;
    runner.execute_action(0, titamon).expect("pick Titamon");
    let _ = runner.auto_resolve();

    assert_eq!(field_ids(&runner, 0), vec!["TITAMON".to_string()]);
    assert_eq!(runner.memory(), memory_before, "played without paying the cost");
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "TITAMON"),
        "Titamon left the trash"
    );
}

#[test]
fn bt24_081_on_deletion_declining_plays_nothing() {
    let (mut runner, perm) = deletion_setup(&["TITAMON"]);

    runner
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);
    decline(&mut runner);

    assert_eq!(runner.battle_area_size(0), 0);
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "TITAMON"));
}

#[test]
fn bt24_081_on_deletion_with_no_eligible_trash_card_installs_nothing() {
    // NEGATIVE: a Lv.6 [Titan] and a non-Titan are not eligible.
    let (mut runner, perm) = deletion_setup(&["TITAN-L6", "PLAIN-L4"]);

    runner
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();

    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.battle_area_size(0), 0);
}

// ─── Section 3 — Assembly -6 [Titamon]×[SkullBaluchimon] ─────────────────────

#[test]
fn bt24_081_assembly_plays_for_8_and_stacks_both_named_materials_from_trash() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(8).start();
    runner.skip_mulligan();
    runner.inject_trash(0, "TITAMON"); // trash index 0
    runner.inject_trash(0, "SKULLBALU"); // trash index 1
    let memory_before = runner.game.memory;

    runner.game.decode_action(PLAY_HAND_START, 0);
    assert!(
        runner.game.pending_selection.is_some(),
        "with both materials in trash the Assembly flow must surface"
    );
    runner.game.decode_action(TRASH_EFFECT_START, 0); // Titamon
    assert!(runner.game.pending_selection.is_some(), "second element (SkullBaluchimon)");
    runner.game.decode_action(TRASH_EFFECT_START + 1, 0); // SkullBaluchimon
    let _ = runner.auto_resolve();

    let perm = PermanentHandle { player: 0, index: 0 };
    let stack = stack_ids(&runner, perm);
    assert_eq!(stack.last().map(String::as_str), Some(CARD_ID));
    assert_eq!(stack.len(), 3, "Titamon + SkullBaluchimon underneath");
    assert!(stack.contains(&"TITAMON".to_string()));
    assert!(stack.contains(&"SKULLBALU".to_string()));
    assert!(runner.game.players[0].trash.is_empty(), "both materials left the trash");
    assert_eq!(memory_before - runner.game.memory, 8, "14 − 6 = 8");
}
