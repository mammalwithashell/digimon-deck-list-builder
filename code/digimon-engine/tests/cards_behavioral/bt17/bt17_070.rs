//! BT17-070 Gulfmon — Digimon, Lv.6, Purple, DP 11000, Cost 12.
//! Traits: Dark Animal.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/BT17-070.md`;
//! card image confirms)
//!
//! [Digivolve] Lv.5 w/[Dark Masters] in text: Cost 3
//! [On Play] [When Digivolving] By placing 1 level 5 card with [Dark Masters]
//! in its text from your hand or trash as this Digimon's bottom digivolution
//! card, delete 1 of your opponent's level 5 or lower Digimon.
//! [When Attacking] By returning 7 cards from your opponent's trash to the
//! bottom of the deck, unsuspend this Digimon.
//!
//! Digivolve: Purple Lv.5 / cost 3.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT17/Purple/BT17_070.cs
//! - Alt digivolve: `AddSelfDigivolutionRequirementStaticEffect(TopCard
//!   IsLevel5 && HasText("Dark Masters"), cost 3, ignore false)`.
//! - [OP]/[WD]: `SetUpActivateClass(..., -1, TRUE, ...)` (outer yes/no),
//!   gated on a qualifying card in hand OR trash; hand-vs-trash zone pick,
//!   then a declinable (`canNoSelect: true`) card pick; on a pick →
//!   `AddDigivolutionCardsBottom`, then a MANDATORY (`canNoSelect: false`)
//!   `SelectPermanentEffect Mode.Destroy` over opponent Digimon with
//!   `Level <= 5` (only if one exists).
//! - [WA]: `SetUpActivateClass(..., -1, TRUE, ...)`, gated on ≥7 cards in the
//!   opponent's trash; select exactly 7 (`maxCount 7, canEndNotMax false`),
//!   `AddLibraryBottomCards` (to their OWNER's deck), then unsuspend self.
//!
//! # Patterns this test covers
//! - A6-adjacent: hand-OR-trash union pick placed as own bottom source
//!   (`select_union_zone` + `place_as_bottom_source`).
//! - E2 optional cost ("By X, Y") with the DCGO outer yes/no
//!   (`optional: true` + `outer_prompt: true`).
//! - Count-capped multi-select over the OPPONENT's trash → deck bottom
//!   (`select_count_capped_multi` + `return_trash_list_to_deck_bottom`),
//!   then self-unsuspend mid-attack.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledScope,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource, UnionZoneSet};

const CARD_ID: &str = "BT17-070";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.level = Some(level);
    c.dp = Some(dp);
    c.colors = vec![CardColor::Purple];
    c
}

/// A Lv.5 Digimon whose printed text mentions [Dark Masters].
fn dark_master(id: &str, level: u8) -> CardData {
    let mut c = digimon(id, level, 7000);
    c.card_name = format!("Dark {id}");
    c.effect_text = "[On Play] Reveal 1 card with [Dark Masters] in its text.".to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT17-070 YAML parses, compiles and is in the embedded pack")
        .add_card(dark_master("DM-HAND", 5))
        .add_card(dark_master("DM-TRASH", 5))
        .add_card(dark_master("DM-L4", 4))
        .add_card(digimon("NODM-L5", 5, 7000))
        .add_card(digimon("OPP-L3", 3, 2000))
        .add_card(digimon("OPP-L5", 5, 7000))
        .add_card(digimon("OPP-L6", 6, 11000))
        .add_card(digimon("FILL", 3, 2000))
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn stack_ids(runner: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
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

fn play_gulfmon(runner: &mut DebugRunner) -> PermanentHandle {
    runner.skip_mulligan();
    let idx = hand_index(runner, 0, CARD_ID);
    let field = runner.play(0, idx).expect("Gulfmon plays from hand");
    PermanentHandle {
        player: 0,
        index: field as u8,
    }
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn bt17_070_metadata_matches_printed_card() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.name, "Gulfmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(11000));
    assert_eq!(card.color, vec![CompiledColor::Purple]);
    assert!(card.traits.iter().any(|t| t == "Dark Animal"));
}

#[test]
fn bt17_070_has_printed_circle_and_dark_masters_text_alt_path() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.alt_paths.len(), 2, "Purple Lv.5/3 circle + Lv.5 w/[Dark Masters] in text/3");
    for p in &card.alt_paths {
        assert_eq!(p.kind, CompiledAltPathKind::Digivolve);
        assert_eq!(p.cost, Some(CompiledCost::Literal(3)));
        assert_eq!(p.from.as_ref().and_then(|f| f.level_eq), Some(5));
    }
    assert_eq!(
        card.alt_paths[1]
            .from
            .as_ref()
            .and_then(|f| f.in_text_contains.as_deref()),
        Some("Dark Masters"),
        "'[Dark Masters] in text' is the whole-card text scan"
    );
}

#[test]
fn bt17_070_has_shared_optional_op_wd_clause_and_optional_wa_clause() {
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
    assert_eq!(triggered.len(), 2, "[OP][WD] shared clause + [WA] clause");

    let op_wd = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play][When Digivolving] clause");
    assert_eq!(
        op_wd.when,
        vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]
    );
    assert!(op_wd.optional, "'By placing …' is an optional processing condition");
    assert!(op_wd.condition.is_some(), "gated on a qualifying card in hand or trash");

    let wa = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::WhenAttacking])
        .expect("[When Attacking] clause");
    assert!(wa.optional, "'By returning 7 cards …' is optional");
    assert!(wa.condition.is_some(), "gated on ≥7 cards in the opponent's trash");
    for t in &triggered {
        assert_eq!(t.scope, CompiledScope::FaceUp);
        assert!(!t.once_per_turn);
    }
}

// ─── Section 2 / 3 — [On Play] / [When Digivolving] ─────────────────────────

#[test]
fn bt17_070_on_play_places_dark_master_from_hand_then_deletes_lv5_or_lower() {
    let mut runner = builder().hand(0, &[CARD_ID, "DM-HAND"]).memory(15).start();
    runner.place_on_field(1, "OPP-L3", Some(0));
    runner.place_on_field(1, "OPP-L5", Some(0));
    runner.place_on_field(1, "OPP-L6", Some(0));

    let gulf = play_gulfmon(&mut runner);

    assert!(runner.pending_is_optional(), "DCGO outer yes/no must be exposed");
    runner
        .accept_optional_trigger()
        .expect("accept the 'By placing …' condition");

    let view = runner.pending_selection_view().expect("union prompt");
    assert_eq!(
        view.kind,
        SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        },
        "hand-OR-trash pick"
    );
    assert_eq!(non_pass_ids(&runner).len(), 1, "only DM-HAND qualifies");
    pick_first(&mut runner, "place DM-HAND");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "after placing, a MANDATORY delete pick over level ≤5 opponent Digimon"
    );
    let view = runner.pending_selection_view().expect("delete prompt");
    assert!(!view.valid_action_ids.contains(&PASS), "the delete is not declinable");
    assert_eq!(non_pass_ids(&runner).len(), 2, "OPP-L3 + OPP-L5 (OPP-L6 excluded)");
    pick_first(&mut runner, "delete");
    let _ = runner.auto_resolve();

    let stack = stack_ids(&runner, gulf);
    assert_eq!(stack.first().map(String::as_str), Some("DM-HAND"), "bottom digivolution card");
    assert_eq!(stack.last().map(String::as_str), Some(CARD_ID));
    assert_eq!(runner.hand_size(0), 0, "DM-HAND left the hand");
    assert_eq!(runner.battle_area_size(1), 2, "one opponent Digimon deleted");
    assert!(field_ids(&runner, 1).contains(&"OPP-L6".to_string()));
}

#[test]
fn bt17_070_on_play_can_place_the_dark_master_from_trash() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(15).start();
    runner.inject_trash(0, "DM-TRASH");
    runner.place_on_field(1, "OPP-L5", Some(0));

    let gulf = play_gulfmon(&mut runner);
    runner.accept_optional_trigger().expect("accept");
    assert_eq!(non_pass_ids(&runner).len(), 1, "DM-TRASH qualifies from trash");
    pick_first(&mut runner, "place DM-TRASH");
    pick_first(&mut runner, "delete OPP-L5");
    let _ = runner.auto_resolve();

    assert_eq!(stack_ids(&runner, gulf).first().map(String::as_str), Some("DM-TRASH"));
    assert_eq!(runner.trash_size(0), 0, "the card left the trash");
    assert_eq!(runner.battle_area_size(1), 0);
}

#[test]
fn bt17_070_on_play_offers_both_hand_and_trash_candidates() {
    let mut runner = builder().hand(0, &[CARD_ID, "DM-HAND"]).memory(15).start();
    runner.inject_trash(0, "DM-TRASH");
    runner.place_on_field(1, "OPP-L5", Some(0));

    play_gulfmon(&mut runner);
    runner.accept_optional_trigger().expect("accept");
    assert_eq!(non_pass_ids(&runner).len(), 2, "hand DM + trash DM both selectable");
}

#[test]
fn bt17_070_on_play_excludes_level_4_and_non_dark_masters_cards() {
    // NEGATIVE: neither a Lv.4 [Dark Masters]-text card nor a Lv.5 without the
    // text qualifies → the gate fails and nothing is offered.
    let mut runner = builder().hand(0, &[CARD_ID, "DM-L4", "NODM-L5"]).memory(15).start();
    runner.place_on_field(1, "OPP-L5", Some(0));

    play_gulfmon(&mut runner);

    assert!(runner.pending_selection().is_none(), "no qualifying card → no prompt");
    assert_eq!(runner.hand_size(0), 2);
    assert_eq!(runner.battle_area_size(1), 1);
}

#[test]
fn bt17_070_on_play_with_no_candidate_anywhere_installs_nothing() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(15).start();
    runner.place_on_field(1, "OPP-L5", Some(0));

    play_gulfmon(&mut runner);

    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.battle_area_size(1), 1);
}

#[test]
fn bt17_070_on_play_declining_places_nothing_and_deletes_nothing() {
    let mut runner = builder().hand(0, &[CARD_ID, "DM-HAND"]).memory(15).start();
    runner.place_on_field(1, "OPP-L3", Some(0));

    let gulf = play_gulfmon(&mut runner);
    assert!(runner.pending_is_optional());
    let player = runner
        .pending_selection()
        .expect("outer prompt")
        .selecting_player;
    runner.execute_action(player, PASS).expect("decline");
    let _ = runner.auto_resolve();

    assert_eq!(stack_ids(&runner, gulf), vec![CARD_ID.to_string()], "no source placed");
    assert_eq!(runner.hand_size(0), 1, "DM-HAND stays in hand");
    assert_eq!(runner.battle_area_size(1), 1, "nothing deleted");
}

#[test]
fn bt17_070_on_play_level_6_opponent_digimon_cannot_be_deleted() {
    // NEGATIVE: only "level 5 or lower". With just a Lv.6 on the other side
    // the source is still placed (the cost was paid) but no delete prompt.
    let mut runner = builder().hand(0, &[CARD_ID, "DM-HAND"]).memory(15).start();
    runner.place_on_field(1, "OPP-L6", Some(0));

    let gulf = play_gulfmon(&mut runner);
    runner.accept_optional_trigger().expect("accept");
    pick_first(&mut runner, "place DM-HAND");
    let _ = runner.auto_resolve();

    assert!(runner.pending_selection().is_none(), "no legal delete target → no prompt");
    assert_eq!(stack_ids(&runner, gulf).first().map(String::as_str), Some("DM-HAND"));
    assert_eq!(runner.battle_area_size(1), 1, "OPP-L6 survives");
}

#[test]
fn bt17_070_when_digivolving_fires_the_same_clause() {
    let mut runner = builder().hand(0, &["DM-HAND"]).memory(15).start();
    let gulf = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-L3", Some(0));

    fire(&mut runner, EffectTiming::WhenDigivolving, gulf);

    assert!(runner.pending_is_optional(), "[When Digivolving] surfaces the same outer yes/no");
    runner.accept_optional_trigger().expect("accept");
    pick_first(&mut runner, "place DM-HAND");
    pick_first(&mut runner, "delete OPP-L3");
    let _ = runner.auto_resolve();

    assert_eq!(stack_ids(&runner, gulf).first().map(String::as_str), Some("DM-HAND"));
    assert_eq!(runner.battle_area_size(1), 0);
}

// ─── Section 2 / 3 — [When Attacking] ────────────────────────────────────────

fn attack_setup(opp_trash: usize) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .security(1, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    let gulf = runner.place_on_field(0, CARD_ID, Some(0));
    for _ in 0..opp_trash {
        runner.inject_trash(1, "FILL");
    }
    (runner, gulf)
}

#[test]
fn bt17_070_when_attacking_returns_7_opponent_trash_cards_to_their_deck_bottom_and_unsuspends() {
    let (mut runner, gulf) = attack_setup(8);
    let opp_deck_before = runner.deck_size(1);
    let opp_trash_before = runner.trash_size(1);

    runner.attack_player(gulf, 1, false);

    assert!(runner.pending_is_optional(), "outer yes/no for 'By returning 7 cards …'");
    runner.accept_optional_trigger().expect("accept");
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect { max: 7, .. })
        ),
        "exactly-7 pick over the opponent's trash, got {:?}",
        runner.pending_kind()
    );
    for i in 0..7 {
        pick_first(&mut runner, &format!("trash pick {i}"));
    }
    let _ = runner.auto_resolve();

    // The attack's security check trashes the checked FILL card afterwards,
    // so measure the cost by the deck delta (exactly 7 returned) and by the
    // trash having shrunk by 7 before that +1.
    assert_eq!(
        runner.deck_size(1),
        opp_deck_before + 7,
        "exactly 7 cards went to the bottom of the OPPONENT's deck"
    );
    assert_eq!(
        runner.trash_size(1),
        opp_trash_before - 7 + 1,
        "8 − 7 returned + 1 security card checked by the attack"
    );
    assert!(
        !runner.game.players[0].battle_area[gulf.index as usize].is_suspended,
        "Gulfmon is unsuspended after paying the cost"
    );
}

#[test]
fn bt17_070_when_attacking_with_fewer_than_7_opponent_trash_cards_offers_nothing() {
    // NEGATIVE: the cost is unpayable → no prompt, Gulfmon stays suspended.
    let (mut runner, gulf) = attack_setup(6);
    let opp_deck_before = runner.deck_size(1);

    runner.attack_player(gulf, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(runner.deck_size(1), opp_deck_before, "nothing returned to the deck");
    assert!(runner.trash_size(1) >= 6, "the opponent's trash was not reduced");
    assert!(runner.game.players[0].battle_area[gulf.index as usize].is_suspended);
}

#[test]
fn bt17_070_when_attacking_declining_leaves_gulfmon_suspended() {
    let (mut runner, gulf) = attack_setup(7);
    let opp_deck_before = runner.deck_size(1);

    runner.attack_player(gulf, 1, false);
    assert!(runner.pending_is_optional());
    let player = runner
        .pending_selection()
        .expect("outer prompt")
        .selecting_player;
    runner.execute_action(player, PASS).expect("decline");
    let _ = runner.auto_resolve();

    assert_eq!(runner.deck_size(1), opp_deck_before, "declined — nothing returned to the deck");
    assert!(runner.trash_size(1) >= 7, "declined — the opponent's trash was not reduced");
    assert!(runner.game.players[0].battle_area[gulf.index as usize].is_suspended);
}
