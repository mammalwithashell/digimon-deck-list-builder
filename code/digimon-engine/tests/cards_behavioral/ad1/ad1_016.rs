//! AD1-016 ShineGreymon — Lv.6, Yellow, DP 12000, Cost 12.
//! Traits: Light Dragon, DATA SQUAD.
//!
//! # Card text (cards.json / AD1-016.json)
//!
//! ＜Alliance＞ (When this Digimon attacks, by suspending 1 of your other
//! Digimon, add the suspended Digimon's DP to this Digimon and it gains
//! ＜Security A. +1＞ for the attack.)
//! ＜Blocker＞ (This Digimon can block in the blocker timing.)
//! [When Digivolving] [When Attacking] [Once Per Turn] You may play 1
//! [Marcus Damon] from your hand or trash without paying the cost. Then, to
//! 1 of your opponent's Digimon, give -3000 DP for each of your Digimon and
//! Tamers until their turn ends.
//! [All Turns] [Once Per Turn] When any of your [Marcus Damon]s are played or
//! suspend, you may delete 1 of your opponent's Digimon with as much or less
//! DP as this Digimon.
//!
//! # Inherited / Security
//! (none)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Yellow/AD1_016.cs
//!
//! # Patterns this test covers
//! - H5 Blocker keyword (declarative grant — runtime-installed, behaviorally
//!   tested via tick_declarative_effects + has_keyword)
//! - H10 Alliance keyword (declarative grant — runtime-installed, behaviorally
//!   tested via tick_declarative_effects + has_keyword)
//! - E1 branch-choice (hand vs trash vs decline) for the free Tamer play
//!   (select_union_zone optional → hand cards + trash cards + PASS)
//! - E2 OPT + optional (the WD/WA cluster)
//! - E3 shared OPT hash across WhenDigivolving + WhenAttacking
//! - C1 union hand-OR-trash free play of named Tamer (play_union_bound_free)
//! - D1 formula-valued DP debuff (-3000 × own Digimon+Tamer count, expiry
//!   end_of_opponents_turn). "Your Digimon and Tamers" = unfiltered
//!   card_count_in_zone over battle_area (only Digimon/Tamers live there).
//! - B3 all-turns observer (on_enter_field_anyone OR on_suspend) for own
//!   Marcus Damon
//! - F delete 1 opp Digimon with DP ≤ this Digimon's DP (dp_lte source_dp)

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource, UnionZoneSet};

// ─── Compiled card helpers ────────────────────────────────────────────────────

fn compiled() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("AD1-016")
}

fn shinegreymon_runner() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("AD1-016")
        .expect("AD1-016 YAML in embedded pack")
}

fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

/// A Marcus Damon Tamer. Name must contain "Marcus Damon" for the
/// name_contains filter on the union-zone selection and the observer.
fn make_marcus_damon(id: &str) -> CardData {
    let mut card = make_test_card(id, "Marcus Damon");
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.colors = vec![CardColor::Yellow];
    card.play_cost = 3;
    card
}

fn make_tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.colors = vec![CardColor::Yellow];
    card
}

fn push_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card {card_id}"));
    let next = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_idx, player, next));
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card {card_id}"));
    let next = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, next));
}

fn has_marcus_on_field(runner: &DebugRunner, player: u8) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_name(&runner.game.card_data) == "Marcus Damon")
}

fn opp_digimon_count(runner: &DebugRunner) -> usize {
    runner.game.players[1]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_kind(&runner.game.card_data) == CardKind::Digimon)
        .count()
}

/// Fully rotate the active player to `target`, flushing the EndOfTurnAction
/// phase each step (that's where `end_of_opponents_turn` modifier expiry and
/// per-turn OPT resets fire). Plain `end_turn` does not flush the
/// EndOfTurnAction step — see BT22-042's expiry test idiom.
fn advance_to_turn_player(runner: &mut DebugRunner, target: u8) {
    for _ in 0..8 {
        if runner.turn_player() == target {
            return;
        }
        let _ = runner.auto_resolve();
        runner.pass_turn();
        let _ = runner.auto_resolve();
        if runner.game.current_phase == digimon_engine::enums::GamePhase::EndOfTurnAction {
            runner.game.pass_end_of_turn_action();
            let _ = runner.auto_resolve();
        }
    }
    assert_eq!(
        runner.turn_player(),
        target,
        "failed to rotate to target turn player"
    );
}

// ─── Section 1: Structural assertions ─────────────────────────────────────────

#[test]
fn ad1_016_has_printed_stats_and_traits() {
    let card = compiled();

    assert_eq!(card.card, "AD1-016");
    assert_eq!(card.name, "ShineGreymon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for trait_name in ["Light Dragon", "DATA SQUAD"] {
        assert!(
            card.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }
}

#[test]
fn ad1_016_declares_blocker_and_alliance_keyword_grants() {
    let card = compiled();

    let mut grants: Vec<&str> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                ..
            }) => Some(keyword.as_str()),
            _ => None,
        })
        .collect();
    grants.sort();

    assert!(
        grants.contains(&"Blocker"),
        "Blocker keyword grant missing — got {grants:?}"
    );
    assert!(
        grants.contains(&"Alliance"),
        "Alliance keyword grant missing — got {grants:?}"
    );
}

// ─── Section 1b: runtime keyword installation (declarative tick) ─────────────
//
// The whole-card grant_keyword clauses for <Blocker> and <Alliance> lower via
// src/dsl_cards/lower_grant_keyword.rs to a declarative effect that
// grant_declarative_keywords; Game::tick_declarative_effects installs it and
// Game::has_keyword reads it (consulted at combat.rs:1292 Alliance interrupt /
// combat.rs:1935 Blocker timing). The former G-DECLARATIVE-KEYWORD gap was
// resolved 2026-05-02 — these behavioral tests pin that the keywords are
// runtime-functional, not merely declared. Idiom mirrors bt21_021 /
// bt12_022's tick_declarative_effects() + has_keyword() pattern.

/// <Blocker> is installed at runtime after a declarative tick.
#[test]
fn ad1_016_blocker_keyword_runtime_installed() {
    use digimon_engine::enums::Keyword;

    let mut runner = shinegreymon_runner().memory(12).start();
    let shine = runner.place_on_field(0, "AD1-016", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(shine, Keyword::Blocker),
        "AD1-016 must have <Blocker> installed at runtime after a declarative tick"
    );
}

/// <Alliance> is installed at runtime after a declarative tick.
#[test]
fn ad1_016_alliance_keyword_runtime_installed() {
    use digimon_engine::enums::Keyword;

    let mut runner = shinegreymon_runner().memory(12).start();
    let shine = runner.place_on_field(0, "AD1-016", Some(0));

    runner.game.tick_declarative_effects();

    assert!(
        runner.game.has_keyword(shine, Keyword::Alliance),
        "AD1-016 must have <Alliance> installed at runtime after a declarative tick"
    );
}

#[test]
fn ad1_016_declares_alt_digivolve_lv5_cost3() {
    let card = compiled();
    // DCGO: Lv.5 w/[RizeGreymon] in name OR [DATA SQUAD] trait, cost 3.
    let has_alt_cost3 = card.alt_paths.iter().any(|p| {
        p.kind == CompiledAltPathKind::Digivolve
            && p.cost == Some(digimon_dsl::compiled::CompiledCost::Literal(3))
    });
    assert!(
        has_alt_cost3,
        "AD1-016 must declare a cost-3 alt digivolve path (RizeGreymon/DATA SQUAD source)"
    );
}

#[test]
fn ad1_016_has_wd_wa_cluster_and_all_turns_observer_clause() {
    let card = compiled();

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    // The WD/WA cluster: one clause with [when_digivolving, when_attacking], OPT.
    assert!(
        triggered
            .iter()
            .any(|t| t.when.contains(&CompiledTiming::WhenDigivolving)
                && t.when.contains(&CompiledTiming::WhenAttacking)
                && t.once_per_turn),
        "expected OPT cluster clause covering [when_digivolving, when_attacking]"
    );

    // The All-Turns observer: fires on Marcus Damon play OR suspend, OPT, optional.
    assert!(
        triggered.iter().any(|t| {
            (t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                || t.when.contains(&CompiledTiming::OnSuspend))
                && t.once_per_turn
                && t.optional
        }),
        "expected [All Turns][OPT] optional observer clause on Marcus Damon play/suspend"
    );

    // All authored clauses are own-scope (no inherited / security text).
    assert!(
        triggered
            .iter()
            .all(|t| matches!(t.scope, CompiledScope::FaceUp)),
        "AD1-016 has no inherited or security effects"
    );
}

// ─── Section 2: WD/WA — free Marcus Damon play, then mandatory DP debuff ──────

#[test]
fn ad1_016_when_digivolving_offers_three_way_union_choice_hand_trash_decline() {
    // Marcus Damon in both hand AND trash → union prompt must expose BOTH
    // cards plus PASS (decline) — the faithful 3-way choice.
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD-HAND"))
        .add_card(make_marcus_damon("MD-TRASH"))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-016", Some(0));
    push_to_hand(&mut runner, 0, "MD-HAND");
    push_to_trash(&mut runner, 0, "MD-TRASH");

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("union-zone selection installs at WhenDigivolving");
    assert_eq!(
        view.kind,
        SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        },
        "must be a hand-OR-trash union prompt"
    );
    // Two Marcus Damon picks (hand + trash) and the optional PASS (decline).
    use digimon_engine::action::space::PASS;
    let non_pass = view
        .valid_action_ids
        .iter()
        .filter(|&&id| id != PASS)
        .count();
    assert_eq!(
        non_pass, 2,
        "both the hand and trash Marcus Damon must be selectable (got {non_pass})"
    );
    assert!(
        runner.pending_is_optional(),
        "the play is optional ('You may') — PASS/decline must be legal"
    );
}

#[test]
fn ad1_016_when_digivolving_play_marcus_from_hand_puts_it_on_field() {
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD-HAND"))
        .add_card(make_digimon("OPP", 5, 6000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-016", Some(0));
    push_to_hand(&mut runner, 0, "MD-HAND");
    runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    // Pick the (only) Marcus Damon to play.
    let (sel_player, action_id) = {
        let view = runner.pending_selection_view().expect("union prompt");
        use digimon_engine::action::space::PASS;
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .expect("a real Marcus Damon pick");
        (view.selecting_player, pick)
    };
    runner
        .execute_action(sel_player, action_id)
        .expect("play Marcus Damon from hand");
    let _ = runner.auto_resolve();

    assert!(
        has_marcus_on_field(&runner, 0),
        "Marcus Damon must be on the field after the free play"
    );
}

#[test]
fn ad1_016_when_digivolving_decline_play_still_applies_dp_debuff() {
    // Decline the free play (PASS) — the "Then, give -3000 DP" must STILL fire
    // (DCGO applies the debuff regardless of whether Marcus Damon was played).
    use digimon_engine::action::space::PASS;
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD-HAND"))
        .add_card(make_digimon("OPP", 6, 12000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-016", Some(0));
    push_to_hand(&mut runner, 0, "MD-HAND");
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    // Decline the play.
    let sel_player = runner.pending_selection().unwrap().selecting_player;
    runner
        .execute_action(sel_player, PASS)
        .expect("decline the free play");

    // Now the mandatory DP-debuff target selection must install (1 own Digimon
    // → -3000 to the opponent's 12000 DP Digimon = 9000).
    let view = runner
        .pending_selection_view()
        .expect("mandatory DP-debuff target selection installs after declining play");
    assert!(
        !view.valid_action_ids.is_empty(),
        "opponent Digimon is a valid debuff target"
    );
    let _ = runner.auto_resolve();

    assert!(
        !has_marcus_on_field(&runner, 0),
        "Marcus Damon must NOT be on the field — we declined the play"
    );
    // 12000 - 3000 (1 own Digimon: ShineGreymon) = 9000.
    assert_eq!(
        runner.dp_of(opp),
        Some(9000),
        "opponent Digimon must drop by 3000 × (own Digimon+Tamer count = 1)"
    );
}

#[test]
fn ad1_016_dp_debuff_scales_with_own_digimon_and_tamer_count() {
    // ShineGreymon + 1 ally Digimon + 1 Tamer = 3 own battle-area permanents →
    // -3000 × 3 = -9000 to the opponent's 14000 DP Digimon = 5000.
    use digimon_engine::action::space::PASS;
    let mut runner = shinegreymon_runner()
        .add_card(make_digimon("ALLY", 5, 6000))
        .add_card(make_tamer("TAMER"))
        .add_card(make_digimon("OPP", 7, 14000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-016", Some(0));
    runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(0, "TAMER", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    // No Marcus Damon available → union prompt may not install; resolve any
    // pending selections, always taking the first non-PASS (real target).
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(view.selecting_player, accept)
            .expect("resolve pending selection");
    }
    let _ = runner.auto_resolve();

    // 14000 - 3000×3 = 5000.
    assert_eq!(
        runner.dp_of(opp),
        Some(5000),
        "debuff must be -3000 × (own Digimon + Tamers = 3)"
    );
}

#[test]
fn ad1_016_dp_debuff_expires_at_end_of_opponents_turn() {
    // The debuff lasts "until their turn ends" → end_of_opponents_turn expiry.
    use digimon_engine::action::space::PASS;
    let mut runner = shinegreymon_runner()
        .add_card(make_digimon("OPP", 6, 12000))
        .add_card(make_digimon("DECK", 3, 3000))
        // Give both players a deck so neither decks out on their draw step
        // while we rotate turns to drive the modifier expiry.
        .deck(0, &["DECK", "DECK", "DECK", "DECK"])
        .deck(1, &["DECK", "DECK", "DECK", "DECK"])
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-016", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(view.selecting_player, accept)
            .expect("resolve pending selection");
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.dp_of(opp), Some(9000), "debuff applied (12000-3000)");

    // Rotate to player 1's turn — debuff persists through the opponent's turn.
    advance_to_turn_player(&mut runner, 1);
    assert_eq!(
        runner.dp_of(opp),
        Some(9000),
        "debuff persists through the opponent's turn"
    );
    // Rotate back to player 0 — the opponent's turn has ended → debuff clears.
    advance_to_turn_player(&mut runner, 0);
    assert_eq!(
        runner.dp_of(opp),
        Some(12000),
        "debuff must expire at end of opponent's turn"
    );
}

#[test]
fn ad1_016_no_dp_debuff_selection_when_opponent_has_no_digimon() {
    // No opponent Digimon → the mandatory debuff installs no selection
    // (DCGO gates on GetBattleAreaDigimons().Any()).
    let mut runner = shinegreymon_runner().memory(20).start();
    let perm = runner.place_on_field(0, "AD1-016", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no Marcus available and no opponent Digimon → nothing to do"
    );
}

#[test]
fn ad1_016_when_attacking_offers_same_body() {
    let mut runner = shinegreymon_runner()
        .add_card(make_digimon("OPP", 6, 12000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-016", Some(0));
    runner.place_on_field(1, "OPP", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "When Attacking must fire the same WD/WA body (mandatory debuff target)"
    );
}

// ─── Section 3: shared OPT lockout across WD + WA ────────────────────────────

#[test]
fn ad1_016_wd_and_wa_share_one_opt_lockout_per_turn() {
    // Fire WhenDigivolving and resolve. Then fire WhenAttacking the same turn:
    // the shared OPT must block it — no second DP debuff applies.
    use digimon_engine::action::space::PASS;
    let mut runner = shinegreymon_runner()
        .add_card(make_digimon("OPP", 7, 14000))
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-016", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(view.selecting_player, accept)
            .expect("resolve WD selection");
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.dp_of(opp), Some(11000), "first debuff (14000-3000)");

    // Same turn, fire WhenAttacking — shared OPT must lock it.
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection().is_none(),
        "shared OPT must lock the When Attacking firing after When Digivolving same turn"
    );
    assert_eq!(
        runner.dp_of(opp),
        Some(11000),
        "no second debuff — DP unchanged"
    );
}

#[test]
fn ad1_016_wd_opt_clears_next_turn() {
    use digimon_engine::action::space::PASS;
    let mut runner = shinegreymon_runner()
        .add_card(make_digimon("OPP", 7, 14000))
        .add_card(make_digimon("DECK", 3, 3000))
        // Decks so neither player decks out while rotating turns.
        .deck(0, &["DECK", "DECK", "DECK", "DECK"])
        .deck(1, &["DECK", "DECK", "DECK", "DECK"])
        .memory(20)
        .start();
    let perm = runner.place_on_field(0, "AD1-016", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(view.selecting_player, accept)
            .expect("resolve WD selection");
    }
    let _ = runner.auto_resolve();

    // Rotate to the opponent's turn and back to player 0; the per-turn OPT
    // lockout must clear on the controller's next turn.
    advance_to_turn_player(&mut runner, 1);
    advance_to_turn_player(&mut runner, 0);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(perm),
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_selection().is_some(),
        "OPT must clear next turn — WD body fires again"
    );
    let _ = opp;
}

// ─── Section 4: All-Turns observer — Marcus Damon play / suspend → delete ────

#[test]
fn ad1_016_own_marcus_play_offers_delete_of_weaker_or_equal_opp_digimon() {
    // Marcus Damon played by controller → may delete 1 opp Digimon with
    // DP ≤ ShineGreymon's 12000. Two opp Digimon: one 8000 (eligible) and one
    // 15000 (ineligible).
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD"))
        .add_card(make_digimon("OPP-WEAK", 5, 8000))
        .add_card(make_digimon("OPP-STRONG", 7, 15000))
        .memory(20)
        .start();
    runner.place_on_field(0, "AD1-016", Some(0));
    runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.place_on_field(1, "OPP-STRONG", Some(0));
    push_to_hand(&mut runner, 0, "MD");

    // Play Marcus Damon (a Tamer) from hand through the engine play path so the
    // OnEnterFieldAnyone observer fires.
    let hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "MD")
        .expect("MD in hand");
    runner.play(0, hand_idx).expect("play Marcus Damon");

    let view = runner
        .pending_selection_view()
        .expect("delete selection installs on Marcus Damon play");
    use digimon_engine::action::space::PASS;
    let non_pass = view
        .valid_action_ids
        .iter()
        .filter(|&&id| id != PASS)
        .count();
    assert_eq!(
        non_pass, 1,
        "only the 8000 DP opp Digimon is ≤ 12000 (the 15000 one is excluded)"
    );
    assert!(
        runner.pending_is_optional(),
        "deletion is optional ('you may delete')"
    );
}

#[test]
fn ad1_016_own_marcus_play_deletes_chosen_opp_digimon() {
    use digimon_engine::action::space::PASS;
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD"))
        .add_card(make_digimon("OPP-WEAK", 5, 8000))
        .memory(20)
        .start();
    runner.place_on_field(0, "AD1-016", Some(0));
    runner.place_on_field(1, "OPP-WEAK", Some(0));
    push_to_hand(&mut runner, 0, "MD");

    let hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "MD")
        .expect("MD in hand");
    runner.play(0, hand_idx).expect("play Marcus Damon");

    let (sel_player, action_id) = {
        let view = runner.pending_selection_view().expect("delete selection");
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .expect("a real delete target");
        (view.selecting_player, pick)
    };
    runner
        .execute_action(sel_player, action_id)
        .expect("delete the opp Digimon");
    let _ = runner.auto_resolve();

    assert_eq!(
        opp_digimon_count(&runner),
        0,
        "the chosen opponent Digimon must be deleted"
    );
}

#[test]
fn ad1_016_own_marcus_suspend_offers_delete() {
    // Marcus Damon already on field; suspending it fires the observer too.
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD"))
        .add_card(make_digimon("OPP-WEAK", 5, 8000))
        .memory(20)
        .start();
    runner.place_on_field(0, "AD1-016", Some(0));
    let marcus = runner.place_on_field(0, "MD", Some(0));
    runner.place_on_field(1, "OPP-WEAK", Some(0));

    runner.game.suspend(marcus);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "suspending your Marcus Damon must offer the delete"
    );
}

#[test]
fn ad1_016_opponent_marcus_play_does_not_trigger_delete() {
    // A Marcus Damon controlled by the OPPONENT must NOT trigger this clause
    // (gate: "any of YOUR Marcus Damons").
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD"))
        .add_card(make_digimon("OPP-WEAK", 5, 8000))
        .memory(20)
        .start();
    runner.place_on_field(0, "AD1-016", Some(0));
    runner.place_on_field(1, "OPP-WEAK", Some(0));
    push_to_hand(&mut runner, 1, "MD");

    // Opponent (player 1) plays the Marcus Damon.
    runner.game.turn_count = 6; // ensure it's a real game state
    let hand_idx = runner.game.players[1]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "MD")
        .expect("MD in opp hand");
    // Place via field so we don't fight the turn-player play gate; then fire
    // the observer with an opponent-owned event to prove the owner gate.
    let _ = hand_idx;
    let marcus_opp = runner.place_on_field(1, "MD", Some(0));
    let card = runner.game.players[1].battle_area[marcus_opp.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnEnterFieldAnyone,
        TriggerSource::EnteredField {
            player: 1,
            permanent: marcus_opp,
            card,
            effect_initiated: false,
        },
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "opponent's Marcus Damon must not trigger this clause (owner gate)"
    );
}

#[test]
fn ad1_016_own_non_marcus_play_does_not_trigger_delete() {
    // A non-Marcus own Digimon play must NOT trigger the observer (name gate).
    let mut runner = shinegreymon_runner()
        .add_card(make_digimon("ALLY", 4, 4000))
        .add_card(make_digimon("OPP-WEAK", 5, 8000))
        .hand(0, &["ALLY"])
        .memory(20)
        .start();
    runner.place_on_field(0, "AD1-016", Some(0));
    runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.game.turn_count = 6;

    runner.play(0, 0).expect("play non-Marcus ally");

    assert!(
        runner.pending_selection().is_none(),
        "non-Marcus play must not trigger the [All Turns] clause (name gate)"
    );
}

#[test]
fn ad1_016_all_turns_observer_no_eligible_target_installs_nothing() {
    // Marcus played but the only opp Digimon is 15000 DP (> 12000) → no
    // eligible delete target → no selection installs.
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD"))
        .add_card(make_digimon("OPP-STRONG", 7, 15000))
        .memory(20)
        .start();
    runner.place_on_field(0, "AD1-016", Some(0));
    runner.place_on_field(1, "OPP-STRONG", Some(0));
    push_to_hand(&mut runner, 0, "MD");

    let hand_idx = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "MD")
        .expect("MD in hand");
    runner.play(0, hand_idx).expect("play Marcus Damon");

    assert!(
        runner.pending_selection().is_none(),
        "no opp Digimon with DP ≤ 12000 → no delete selection"
    );
}

#[test]
fn ad1_016_all_turns_observer_opt_blocks_second_marcus_play_same_turn() {
    // Two Marcus Damons played in one turn — only the first may delete (OPT).
    use digimon_engine::action::space::PASS;
    let mut runner = shinegreymon_runner()
        .add_card(make_marcus_damon("MD1"))
        .add_card(make_marcus_damon("MD2"))
        .add_card(make_digimon("OPP-A", 5, 8000))
        .add_card(make_digimon("OPP-B", 5, 8000))
        .memory(40)
        .start();
    runner.place_on_field(0, "AD1-016", Some(0));
    runner.place_on_field(1, "OPP-A", Some(0));
    runner.place_on_field(1, "OPP-B", Some(0));
    push_to_hand(&mut runner, 0, "MD1");
    push_to_hand(&mut runner, 0, "MD2");

    // First Marcus play → delete one opp Digimon.
    let i1 = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "MD1")
        .unwrap();
    runner.play(0, i1).expect("play MD1");
    let (sp, a) = {
        let view = runner.pending_selection_view().expect("first delete");
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .expect("a delete target");
        (view.selecting_player, pick)
    };
    runner
        .execute_action(sp, a)
        .expect("delete on first Marcus");
    let _ = runner.auto_resolve();
    assert_eq!(opp_digimon_count(&runner), 1, "first delete removed one");

    // Second Marcus play same turn → OPT must lock; no further delete.
    let i2 = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "MD2")
        .unwrap();
    runner.play(0, i2).expect("play MD2");
    assert!(
        runner.pending_selection().is_none(),
        "OPT must block the second Marcus Damon's delete same turn"
    );
    assert_eq!(opp_digimon_count(&runner), 1, "no second delete");
}
