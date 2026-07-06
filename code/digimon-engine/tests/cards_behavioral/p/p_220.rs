//! P-220 Millenniummon — Digimon, Lv.7, Red/Black, DP 14000, Cost 14.
//! Traits: [Composite, DM, Ver.3, Ver.5]. Attribute: Virus.
//!
//! # Card text (code/digimon-engine/cards/p/P-220.json, cross-checked against
//! # data/card_bundles/P-220.md — official Bandai DB — and the printed card
//! # image at C:\Users\james\Documents\DCGO_Application\Assets\Textures\Card\P-220.webp)
//!
//! ```text
//! <Reboot> (Unsuspend this Digimon during your opponent's unsuspend phase.)
//! <Blocker> (This Digimon can block in the blocker timing.)
//! [On Play] [When Digivolving] <De-Digivolve 2> 1 of your opponent's Digimon.
//! Then, you may delete 1 Digimon.
//! [On Deletion] By returning 3 [Composite], [Wicked God] or [DM] cards from
//! your trash to the bottom of the deck, you may play 2 level 6 or lower
//! [Composite], [Ver.3] or [Ver.5] Digimon cards from your trash without
//! paying the costs. This effect can't play cards of the same level.
//!
//! Digivolve: [Kimeramon] — Cost 6
//! DNA Digivolve: [Kimeramon] + [Machinedramon] — Cost 0 (stack unsuspended)
//! Assembly -6: 3 [Composite]/[Ver.3]/[Ver.5] trait Digimon cards w/different levels
//! ```
//!
//! # DCGO C# reference
//! C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO/Assets/Scripts/CardEffect/P/Red/P_220.cs
//! (read from the base-repo absolute path per rule 29 — DCGO is not
//! initialized in this worktree)
//!
//! Key DCGO behaviors this test suite pins:
//! - The shared On Play / When Digivolving effect is a single MANDATORY
//!   activation (`ActivateClassesForSharedEffects(..., optional: false, ...)`)
//!   containing two internal, independently-gated steps:
//!     1. De-Digivolve 2 exactly one **opponent** Digimon — only offered if
//!        the opponent has a Digimon (`HasMatchConditionPermanent`); the pick
//!        itself is mandatory once offered (`canNoSelect: false`).
//!     2. "You may delete 1 Digimon" — targets **any** Digimon on **either**
//!        battle area (`IsPermanentExistsOnBattleAreaDigimon`, no owner
//!        filter), and is optional (`canNoSelect: true`, `mode: Destroy`).
//! - `AddSelfDigivolutionRequirementStaticEffect`: printed alt digivolve
//!   `[Digivolve] [Kimeramon]: Cost 6` — gated on the source's top card
//!   being named "Kimeramon".
//! - `AddJogressConditionClass` / `GetJogress`: printed DNA digivolve
//!   `[Kimeramon] + [Machinedramon]: Cost 0`, stacks unsuspended.
//! - `AddAssemblyConditionClass`: printed Assembly -6 special play — 3
//!   Composite/Ver.3/Ver.5 trait Digimon cards with pairwise-different
//!   levels, reducing play cost by 6 (14 - 6 = 8).
//! - `OnDestroyedAnyone` (On Deletion): cost-gated by
//!   `card.Owner.TrashCards.Count(CardSelectCondition) >= 3` where
//!   `CardSelectCondition` matches Composite / Wicked God / any DM-family
//!   trait. Selecting fewer than 3 aborts the entire effect (DCGO requires
//!   `selectedCards.Count == 3`). After the 3-card return, IF the owner has
//!   any level<=6 Composite/Ver.3/Ver.5 Digimon in trash, offers an optional
//!   up-to-2 pick with pairwise-different-level enforcement, then plays each
//!   picked card free (`payCost: false`, `activateETB: true` — their own On
//!   Play DOES fire, no suppression).
//!
//! # Patterns this test covers
//! - Printed keywords: Reboot, Blocker (declarative GrantKeyword, face-up).
//! - Shared On Play / When Digivolving timing (both fire the same process).
//! - `de_digivolve` (amount 2, stop_at_level 3) targeting exactly 1 opponent
//!   Digimon (mandatory pick when eligible; no prompt when no opponent
//!   Digimon exists).
//! - `select_any_permanent` (cross-owner target pool, SelectionKind::AnyField)
//!   for the optional "delete 1 Digimon" step.
//! - On Deletion cost-gated effect: exact-3 multi-select
//!   (`select_count_capped_multi`, min 3 / max 3) over trash with a trait
//!   `any_of` filter, feeding `return_to_deck_from_reveal` (bottom).
//! - Nested optional up-to-2 multi-select with `distinct_by: level` over
//!   trash, feeding `per_selected` + `play_from_trash_free` (their own On
//!   Play fires — no suppress_on_play).
//! - `alt_paths`: `kind: digivolve` (Kimeramon-named source, cost 6),
//!   `kind: dna_digivolve` (Kimeramon + Machinedramon materials, cost 0,
//!   `stacks_unsuspended: true`), `kind: assembly` (3 materials,
//!   `distinct_by: level`, cost 6 -> reduces printed cost 14 -> 8).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A generic Composite-trait Digimon usable as an opponent target / filler.
fn composite_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.traits = vec!["Composite".to_string()];
    c
}

/// Kimeramon (all real printings: BT2-077 / BT8-084 / BT18-015 / BT19-070 /
/// EX9-074) is Lv.5, never Lv.6 — deliberately kept off P-220's `level_eq: 6`
/// standard-box filter so this fixture exercises ONLY the name-gated
/// `[Digivolve] [Kimeramon]: Cost 6` alt path (no distinct-cost collision
/// with the standard route, which the DSL would otherwise surface as a
/// cost-choice prompt rather than an immediate commit).
fn kimeramon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Kimeramon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(13000);
    c.play_cost = 12;
    c.traits = vec!["Composite".to_string()];
    c
}

fn machinedramon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Machinedramon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(15000);
    c.play_cost = 13;
    c.traits = vec!["Composite".to_string()];
    c
}

fn p220_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("P-220")
        .expect("P-220 must be in embedded DSL pack")
        .add_card(composite_digimon("FILLER-L3", "Filler", 3, 4000))
        .memory(20)
        .start()
}

/// Push a CardSource referencing `card_id` into player `p`'s hand.
fn push_to_hand(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].hand.push(card);
}

/// Push a CardSource referencing `card_id` directly into player `p`'s trash.
fn push_to_trash(runner: &mut DebugRunner, p: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, p, next_idx);
    runner.game.players[p as usize].trash.push(card);
}

fn hand_index_of(runner: &DebugRunner, p: PlayerId, card_id: &str) -> usize {
    runner.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("card {} not found in player {} hand", card_id, p))
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p_220_compiles_in_embedded_pack() {
    let runner = p220_runner();
    assert!(
        runner.compiled_card("P-220").is_some(),
        "P-220 must be present in embedded DSL pack"
    );
}

#[test]
fn p_220_card_metadata_matches_print() {
    let runner = p220_runner();
    let card = runner.compiled_card("P-220").expect("P-220 compiles");

    assert_eq!(card.level, Some(7), "Millenniummon is Lv.7");
    assert_eq!(card.dp, Some(14000), "Millenniummon is DP 14000");
    assert_eq!(card.cost, Some(14), "Millenniummon costs 14 to play");
    assert!(
        card.traits.iter().any(|t| t == "Composite"),
        "must carry the Composite trait"
    );
}

#[test]
fn p_220_has_face_up_reboot_grant_keyword() {
    let runner = p220_runner();
    let card = runner.compiled_card("P-220").expect("compiled");

    let reboot = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Reboot"
        )
    });
    assert!(reboot, "P-220 must declare a face-up GrantKeyword(Reboot) clause");
}

#[test]
fn p_220_has_face_up_blocker_grant_keyword() {
    let runner = p220_runner();
    let card = runner.compiled_card("P-220").expect("compiled");

    let blocker = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::FaceUp,
                ..
            }) if keyword == "Blocker"
        )
    });
    assert!(blocker, "P-220 must declare a face-up GrantKeyword(Blocker) clause");
}

#[test]
fn p_220_has_on_play_when_digivolving_mandatory_shared_clause() {
    let runner = p220_runner();
    let card = runner.compiled_card("P-220").expect("compiled");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay) && t.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .expect("P-220 must have a triggered clause covering OnPlay + WhenDigivolving");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        !clause.optional,
        "shared OnPlay/WhenDigivolving activation is MANDATORY \
         (DCGO ActivateClassesForSharedEffects passes optional: false); \
         the optionality lives on the inner delete step only"
    );
}

#[test]
fn p_220_on_play_clause_contains_de_digivolve_two_step() {
    let runner = p220_runner();
    let card = runner.compiled_card("P-220").expect("compiled");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay) && t.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .expect("shared clause exists");

    assert!(
        clause.process.iter().any(|s| matches!(
            s,
            CompiledStep::DeDigivolve {
                amount: Some(2),
                ..
            }
        )),
        "shared clause must De-Digivolve 2; got {:?}",
        clause.process
    );
    // The "you may delete 1 Digimon" step is nested inside an `if
    // binding_exists` guard (per §11.7 we don't dig into nested process
    // trees structurally) — its presence and behavior are covered by the
    // Section 3 integrated behavioral tests instead.
}

#[test]
fn p_220_has_on_deletion_clause() {
    let runner = p220_runner();
    let card = runner.compiled_card("P-220").expect("compiled");

    let has_on_deletion = card.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::OnDeletion),
        _ => false,
    });
    assert!(has_on_deletion, "expected an On Deletion triggered clause");
}

#[test]
fn p_220_alt_paths_cover_digivolve_dna_and_assembly() {
    let runner = p220_runner();
    let card = runner.compiled_card("P-220").expect("compiled");

    // P-220 declares THREE `Digivolve` alt-paths: the standard Red Lv.6 box,
    // the standard Black Lv.6 box (each colour is its own alt_path entry —
    // they do not collapse), and the Kimeramon-named special route (cost 6).
    // Find the Kimeramon-gated one specifically by its `from: { name_is:
    // "Kimeramon" }` predicate rather than the first `Digivolve`-kind match.
    let digivolve_paths: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(
        digivolve_paths.len(),
        3,
        "expected standard Red Lv.6 + standard Black Lv.6 + Kimeramon special \
         route; got {}",
        digivolve_paths.len()
    );
    let kimeramon_route = digivolve_paths
        .iter()
        .find(|p| {
            p.from
                .as_ref()
                .map(|f| f.name_is.as_deref() == Some("Kimeramon"))
                .unwrap_or(false)
        })
        .expect("Kimeramon-gated Digivolve alt path (from: { name_is: Kimeramon })");
    assert_eq!(
        kimeramon_route.cost,
        Some(CompiledCost::Literal(6)),
        "[Digivolve] [Kimeramon]: Cost 6"
    );

    let dna = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .expect("Kimeramon + Machinedramon DNA alt path");
    assert_eq!(dna.materials.len(), 2, "DNA needs exactly 2 materials");
    assert!(dna.stacks_unsuspended, "DNA digivolve stacks unsuspended");
    assert_eq!(dna.cost, Some(CompiledCost::Literal(0)), "DNA Digivolve cost 0");

    let assembly = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly -6 alt path");
    assert_eq!(assembly.materials.len(), 3, "Assembly needs 3 materials");
    assert_eq!(
        assembly.cost,
        Some(CompiledCost::Literal(6)),
        "Assembly reduces cost by 6 (14 -> 8)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating: De-Digivolve step (positive / negative)
// ═══════════════════════════════════════════════════════════════════════════

/// Positive: with an opponent Digimon on the field, playing P-220 installs
/// an OppField prompt for De-Digivolve 2.
#[test]
fn p_220_on_play_offers_de_digivolve_when_opponent_has_a_digimon() {
    let mut runner = p220_runner();
    push_to_hand(&mut runner, 0, "P-220");
    runner.place_on_field(1, "FILLER-L3", Some(0));

    let idx = hand_index_of(&runner, 0, "P-220");
    runner.play(0, idx);

    let view = runner
        .pending_selection_view()
        .expect("De-Digivolve target prompt must install");
    assert_eq!(
        view.kind,
        SelectionKind::OppField,
        "select_opponent_permanent installs OppField selection"
    );
    assert!(
        !runner.pending_is_optional(),
        "the De-Digivolve target pick is mandatory once an opponent Digimon exists \
         (DCGO canNoSelect: false)"
    );
}

/// Negative: with no opponent Digimon at all, no OppField prompt installs.
#[test]
fn p_220_on_play_skips_de_digivolve_when_opponent_has_no_digimon() {
    let mut runner = p220_runner();
    push_to_hand(&mut runner, 0, "P-220");

    let idx = hand_index_of(&runner, 0, "P-220");
    runner.play(0, idx);

    let kind = runner.pending_kind();
    assert_ne!(
        kind,
        Some(SelectionKind::OppField),
        "must not prompt De-Digivolve 2 with no opponent Digimon on the field"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral outcome: De-Digivolve + optional delete
// ═══════════════════════════════════════════════════════════════════════════

/// De-Digivolve 2 pops 2 digivolution sources off the selected opponent
/// Digimon.
#[test]
fn p_220_on_play_de_digivolves_selected_opponent_digimon_by_two() {
    let mut runner = p220_runner();
    push_to_hand(&mut runner, 0, "P-220");
    let bottom = runner.place_on_field(1, "FILLER-L3", None);
    runner.push_source(bottom, "FILLER-L3");
    runner.push_source(bottom, "FILLER-L3");
    let sources_before = runner.game.players[1].battle_area[bottom.index as usize]
        .card_sources
        .len();

    let idx = hand_index_of(&runner, 0, "P-220");
    runner.play(0, idx);

    let v1 = runner
        .pending_selection_view()
        .expect("De-Digivolve prompt");
    // Field-selection action IDs always encode with slot 0 as the "attacker"
    // half (`install_field_selection` calls `encode_attack(0, field_index)`
    // regardless of which player's field is targeted).
    let pick = encode_attack(0, bottom.index as u16);
    assert!(v1.valid_action_ids.contains(&pick));
    runner
        .execute_action(v1.selecting_player, pick)
        .expect("pick the opponent Digimon");
    runner.game.drain_effect_queue();

    // Decline the follow-up optional delete so the De-Digivolve delta is
    // isolated.
    if runner.pending_is_optional() {
        let view = runner.pending_selection_view().expect("delete prompt");
        runner
            .execute_action(view.selecting_player, PASS)
            .expect("decline the optional delete");
    }
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.game.players[1].battle_area[bottom.index as usize]
            .card_sources
            .len(),
        sources_before - 2,
        "De-Digivolve 2 must trash exactly 2 digivolution sources"
    );
}

/// The optional "delete 1 Digimon" step must offer BOTH players' Digimon
/// (DCGO PermanentSelectCondition1 has no owner filter) via
/// SelectionKind::AnyField, and declining leaves the board untouched.
#[test]
fn p_220_on_play_optional_delete_offers_any_field_and_decline_is_a_no_op() {
    let mut runner = p220_runner();
    push_to_hand(&mut runner, 0, "P-220");
    runner.place_on_field(1, "FILLER-L3", Some(0));
    let own = runner.place_on_field(0, "FILLER-L3", Some(0));

    let idx = hand_index_of(&runner, 0, "P-220");
    runner.play(0, idx);

    // Resolve the mandatory De-Digivolve pick first.
    let v1 = runner.pending_selection_view().expect("De-Digivolve prompt");
    assert_eq!(v1.kind, SelectionKind::OppField);
    let de_digivolve_target = v1.valid_action_ids[0];
    runner
        .execute_action(v1.selecting_player, de_digivolve_target)
        .expect("resolve De-Digivolve target");
    runner.game.drain_effect_queue();

    let v2 = runner
        .pending_selection_view()
        .expect("optional delete prompt must install (own + opp Digimon both present)");
    assert_eq!(
        v2.kind,
        SelectionKind::AnyField,
        "select_any_permanent must install SelectionKind::AnyField (cross-owner pool)"
    );
    assert!(
        runner.pending_is_optional(),
        "'you may delete 1 Digimon' must be optional"
    );
    let own_action = encode_attack(0, own.index as u16);
    assert!(
        v2.valid_action_ids.contains(&own_action),
        "the controller's OWN Digimon must be a legal delete target too"
    );

    let field_before = runner.battle_area_size(1) + runner.battle_area_size(0);
    runner
        .execute_action(v2.selecting_player, PASS)
        .expect("decline the optional delete");
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.battle_area_size(1) + runner.battle_area_size(0),
        field_before,
        "declining the optional delete must not remove any Digimon"
    );
}

/// Positive: the optional delete CAN target the controller's own Digimon,
/// not just the opponent's.
#[test]
fn p_220_on_play_optional_delete_can_target_own_digimon() {
    let mut runner = p220_runner();
    push_to_hand(&mut runner, 0, "P-220");
    runner.place_on_field(1, "FILLER-L3", Some(0));
    let own = runner.place_on_field(0, "FILLER-L3", Some(0));

    let idx = hand_index_of(&runner, 0, "P-220");
    runner.play(0, idx);

    let v1 = runner.pending_selection_view().expect("De-Digivolve prompt");
    runner
        .execute_action(v1.selecting_player, v1.valid_action_ids[0])
        .expect("resolve De-Digivolve target");
    runner.game.drain_effect_queue();

    let v2 = runner.pending_selection_view().expect("optional delete prompt");
    let own_action = encode_attack(0, own.index as u16);
    let own_field_before = runner.battle_area_size(0);
    runner
        .execute_action(v2.selecting_player, own_action)
        .expect("delete the controller's own Digimon");
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.battle_area_size(0),
        own_field_before - 1,
        "the controller's own Digimon must be deletable, not just the opponent's"
    );
}

/// [When Digivolving] fires the same shared body as [On Play].
#[test]
fn p_220_when_digivolving_fires_same_shared_body() {
    let mut runner = p220_runner();
    let opp = runner.place_on_field(1, "FILLER-L3", Some(0));
    let millen = runner.place_on_field(0, "P-220", None);

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(millen));
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "[When Digivolving] must fire the same De-Digivolve 2 prompt as [On Play]"
    );
    let _ = opp;
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — On Deletion: cost-gated return + optional free-play
// ═══════════════════════════════════════════════════════════════════════════

/// Negative: fewer than 3 eligible trash cards → no On Deletion prompt.
#[test]
fn p_220_on_deletion_no_prompt_with_fewer_than_three_eligible_trash_cards() {
    let mut runner = p220_runner();
    let perm = runner.place_on_field(0, "P-220", None);
    // Trash has 0 eligible Composite/Wicked God/DM cards.
    runner
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "On Deletion must not prompt with fewer than 3 eligible trash cards \
         (the deleted P-220 itself only supplies 1)"
    );
}

/// Positive: with 3 eligible Composite-trait cards already in trash (plus the
/// deleted P-220 itself = 4 total), the cost-gated effect installs an OUTER
/// accept/decline prompt first (G-OUTER-OPTIONAL-NOT-INSTALLED: the clause is
/// `optional: true` but its first body step, the exact-3 multi-select, is not
/// itself declinable once payable — `min=3, optional_zero=false` — so the
/// lowering pass installs an explicit outer "you may" confirm, faithfully
/// matching DCGO's `canNoSelect: () => true` decline surface). Accepting it
/// then surfaces the exact-3 multi-select.
#[test]
fn p_220_on_deletion_offers_outer_accept_then_exact_three_return_selection() {
    let mut runner = p220_runner();
    push_to_trash(&mut runner, 0, "FILLER-L3");
    push_to_trash(&mut runner, 0, "FILLER-L3");
    push_to_trash(&mut runner, 0, "FILLER-L3");
    let perm = runner.place_on_field(0, "P-220", None);

    runner
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "the optional On Deletion effect installs an outer accept/decline prompt"
    );
    assert!(runner.pending_is_optional(), "the outer prompt must be declinable");

    runner
        .accept_optional_trigger()
        .expect("accept the outer On Deletion prompt");

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::CountCappedMultiSelect { min: 3, max: 3, picked: 0, .. })
        ),
        "after accepting, must prompt an exact-3 trash return selection; got {:?}",
        runner.pending_kind()
    );
}

/// Declining the outer accept/decline prompt must not return any cards.
#[test]
fn p_220_on_deletion_declining_the_outer_prompt_returns_nothing() {
    let mut runner = p220_runner();
    push_to_trash(&mut runner, 0, "FILLER-L3");
    push_to_trash(&mut runner, 0, "FILLER-L3");
    push_to_trash(&mut runner, 0, "FILLER-L3");
    let perm = runner.place_on_field(0, "P-220", None);
    let deck_before = runner.deck_size(0);

    runner
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    runner
        .decline_optional_trigger()
        .expect("decline the outer On Deletion prompt");
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "declining the outer prompt must not return any cards to the deck bottom"
    );
}

/// Once the outer prompt is accepted, the exact-3 return selection itself is
/// mandatory (no PASS mid-pick — `min=3, optional_zero=false`).
#[test]
fn p_220_on_deletion_return_selection_is_mandatory_after_accepting() {
    let mut runner = p220_runner();
    push_to_trash(&mut runner, 0, "FILLER-L3");
    push_to_trash(&mut runner, 0, "FILLER-L3");
    push_to_trash(&mut runner, 0, "FILLER-L3");
    let perm = runner.place_on_field(0, "P-220", None);

    runner
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    runner
        .accept_optional_trigger()
        .expect("accept the outer On Deletion prompt");

    assert!(
        !runner.pending_is_optional(),
        "the exact-3 return selection has no PASS available at picked=0 \
         (min=3, optional_zero=false — mandatory once accepted and payable)"
    );
    let view = runner.pending_selection_view().expect("return prompt");
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "PASS must not be a legal action before reaching 3 picks"
    );
}

/// Paying the 3-card cost returns exactly 3 cards to the deck bottom.
#[test]
fn p_220_on_deletion_returns_three_cards_to_deck_bottom() {
    let mut runner = p220_runner();
    push_to_trash(&mut runner, 0, "FILLER-L3");
    push_to_trash(&mut runner, 0, "FILLER-L3");
    push_to_trash(&mut runner, 0, "FILLER-L3");
    let perm = runner.place_on_field(0, "P-220", None);

    let deck_before = runner.deck_size(0);

    runner
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    runner
        .accept_optional_trigger()
        .expect("accept the outer On Deletion prompt");

    for _ in 0..3 {
        let view = runner
            .pending_selection_view()
            .expect("return-selection re-arms for each of the 3 picks");
        let pick = view.valid_action_ids[0];
        runner
            .execute_action(view.selecting_player, pick)
            .expect("pick a trash card to return");
    }
    runner.game.drain_effect_queue();
    // No level<=6 Composite/Ver.3/Ver.5 candidates remain in trash (all 3
    // FILLER-L3 cards were just returned), so the nested play prompt should
    // not install; auto_resolve is safe as a tail-drain here.
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.deck_size(0),
        deck_before + 3,
        "exactly 3 cards must land at the deck bottom"
    );
}

/// After paying the return cost, an optional up-to-2 different-level play
/// prompt installs when eligible level<=6 Composite/Ver.3/Ver.5 cards remain
/// in trash, and playing one fires that card's own On Play.
#[test]
fn p_220_on_deletion_plays_a_different_level_card_free_and_fires_its_on_play() {
    let mut runner = p220_runner();
    // 3 cost cards (consumed by the return).
    push_to_trash(&mut runner, 0, "FILLER-L3");
    push_to_trash(&mut runner, 0, "FILLER-L3");
    push_to_trash(&mut runner, 0, "FILLER-L3");
    // A 4th, distinct-level Composite Digimon left over to be played free.
    let level4 = composite_digimon("P220-L4", "Level4Composite", 4, 5000);
    runner.game.card_data.push(level4);
    push_to_trash(&mut runner, 0, "P220-L4");

    let perm = runner.place_on_field(0, "P-220", None);
    let field_before = runner.battle_area_size(0);

    runner
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    runner
        .accept_optional_trigger()
        .expect("accept the outer On Deletion prompt");

    // Pay the 3-card return cost (must avoid picking the level-4 filler so it
    // remains available for the free-play step).
    for _ in 0..3 {
        let view = runner.pending_selection_view().expect("return prompt");
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| a != PASS)
            .expect("a non-PASS return pick is available");
        runner
            .execute_action(view.selecting_player, pick)
            .expect("pick a cost card to return");
    }
    runner.game.drain_effect_queue();

    // Nested optional up-to-2 multi-select over remaining eligible trash.
    let free_play_view = runner
        .pending_selection_view()
        .expect("nested free-play selection must install with an eligible card remaining");
    assert!(
        matches!(
            free_play_view.kind,
            SelectionKind::CountCappedMultiSelect { max: 2, .. }
        ),
        "nested free-play prompt must be an up-to-2 multi-select; got {:?}",
        free_play_view.kind
    );
    let pick = free_play_view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("the level-4 Composite card is a legal pick");
    runner
        .execute_action(free_play_view.selecting_player, pick)
        .expect("pick the level-4 Composite card");

    // Decline further picks.
    if let Some(v) = runner.pending_selection_view() {
        if matches!(v.kind, SelectionKind::CountCappedMultiSelect { .. }) {
            let _ = runner.execute_action(v.selecting_player, PASS);
        }
    }
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert!(
        runner.battle_area_size(0) >= field_before,
        "at least one free play must land a Digimon back on the field"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "P220-L4"),
        "the freely-played card must be the level-4 Composite Digimon"
    );
}

/// This effect can't play cards of the same level: with only same-level
/// (level 4) Composite Digimon remaining in trash after the cost, the nested
/// multi-select allows at most ONE pick (no second same-level pick).
#[test]
fn p_220_on_deletion_cannot_play_two_cards_of_the_same_level() {
    let mut runner = p220_runner();
    push_to_trash(&mut runner, 0, "FILLER-L3");
    push_to_trash(&mut runner, 0, "FILLER-L3");
    push_to_trash(&mut runner, 0, "FILLER-L3");
    // Two MORE level-4 Composite Digimon (same level, both eligible for the
    // level<=6 reward filter) sit in trash — same level, so at most one can
    // ever be picked (distinct_by: level).
    let level4a = composite_digimon("P220-DUP-A", "DupCompositeA", 4, 5000);
    let level4b = composite_digimon("P220-DUP-B", "DupCompositeB", 4, 5000);
    runner.game.card_data.push(level4a);
    runner.game.card_data.push(level4b);
    push_to_trash(&mut runner, 0, "P220-DUP-A");
    push_to_trash(&mut runner, 0, "P220-DUP-B");

    let perm = runner.place_on_field(0, "P-220", None);

    runner
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();
    runner
        .accept_optional_trigger()
        .expect("accept the outer On Deletion prompt");

    // Pick specifically the 3 FILLER-L3 cards for the cost, leaving the two
    // level-4 duplicates untouched for the reward step. Cross-reference each
    // currently-legal action id against the live trash by card_id (rather
    // than hand-computing TRASH_EFFECT_START + index) to stay faithful to
    // whatever index scheme the engine's candidate list actually uses.
    use digimon_engine::action::space::TRASH_EFFECT_START;
    for _ in 0..3 {
        let view = runner.pending_selection_view().expect("return prompt");
        let pick = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&a| {
                let idx = a.wrapping_sub(TRASH_EFFECT_START) as usize;
                runner.game.players[0]
                    .trash
                    .get(idx)
                    .map(|c| c.card_id(&runner.game.card_data) == "FILLER-L3")
                    .unwrap_or(false)
            })
            .expect("a FILLER-L3 cost card is a legal pick");
        runner
            .execute_action(view.selecting_player, pick)
            .expect("pick a FILLER-L3 cost card to return");
    }
    runner.game.drain_effect_queue();

    if let Some(view) = runner.pending_selection_view() {
        if matches!(view.kind, SelectionKind::CountCappedMultiSelect { .. }) {
            let first_pick = view
                .valid_action_ids
                .iter()
                .copied()
                .find(|&a| a != PASS)
                .expect("one level-4 Composite pick is legal");
            runner
                .execute_action(view.selecting_player, first_pick)
                .expect("pick the first level-4 duplicate");

            // After one level-4 pick, no further pick of a DIFFERENT
            // level-4 card should be legal — only PASS (distinct_by: level).
            if let Some(v2) = runner.pending_selection_view() {
                let non_pass: Vec<u16> =
                    v2.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
                assert!(
                    non_pass.is_empty(),
                    "no second same-level pick should be legal; got {non_pass:?}"
                );
            }
        }
    }
    let _ = runner.auto_resolve();
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Alt-path behavioral: Kimeramon digivolve (Main-window reachable)
// ═══════════════════════════════════════════════════════════════════════════
//
// The regular (single-material) `Digivolve` alt-path IS wired for the
// user-action Main-phase digivolve-from-hand flow via
// `Game::collect_dsl_alt_digivolve_routes` (confirmed against
// `tests/alt_path_reachability.rs`, the production reachability guard for
// exactly this alt-path shape). `Game::digivolve_from_hand` is exercised
// directly here, matching that guard's idiom.

/// P-220 can digivolve from a Kimeramon-named base via the printed alt path
/// (`[Digivolve] [Kimeramon]: Cost 6`), reachable through the same
/// `alt_path_registry` route real gameplay uses.
#[test]
fn p_220_can_digivolve_from_kimeramon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-220")
        .expect("P-220 loads")
        .add_card(kimeramon("KIM-BASE"))
        .hand(0, &["P-220"])
        .memory(20)
        .start();
    runner
        .game
        .set_logger(Box::new(digimon_engine::logger::VerboseLogger::new()));
    runner.game.turn_count = 1;
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;
    let base = runner.place_on_field(0, "KIM-BASE", Some(0));

    let proceeded = runner.game.digivolve_from_hand(
        0,
        0,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByHand,
    );
    assert!(
        proceeded,
        "P-220 must be able to digivolve from a Kimeramon-named base via the \
         [Digivolve] [Kimeramon]: Cost 6 alt path; logs: {:?}",
        runner.game.logger.get_logs()
    );
    let on_top = runner.game.players[0].battle_area[base.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        == "P-220";
    assert!(on_top, "P-220 must be the new top card of the stack");
}

/// P-220 CANNOT digivolve from a non-Kimeramon, non-standard-color base —
/// proving the Kimeramon alt path is gated on the source's top-card NAME,
/// not merely on being a valid Digimon.
#[test]
fn p_220_cannot_digivolve_from_unrelated_base_with_no_standard_route() {
    // Level 5 (NOT 6) so this base misses BOTH the standard Red/Black Lv.6
    // box AND the Kimeramon name gate — no alt path should apply at all.
    let mut runner = DebugRunner::builder()
        .dsl_card("P-220")
        .expect("P-220 loads")
        .add_card(composite_digimon("NOT-KIM", "SomeOtherDigimon", 5, 8000))
        .hand(0, &["P-220"])
        .memory(20)
        .start();
    runner.game.turn_count = 1;
    runner.game.current_phase = digimon_engine::enums::GamePhase::Main;
    let base = runner.place_on_field(0, "NOT-KIM", Some(0));

    let proceeded = runner.game.digivolve_from_hand(
        0,
        0,
        base.index as usize,
        digimon_engine::enums::PlaySource::ByHand,
    );
    assert!(
        !proceeded,
        "P-220 must NOT digivolve from a Lv.5, non-Kimeramon-named base \
         (misses the Lv.6 standard box AND the Kimeramon name gate)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 6 — DNA digivolve: structural-only (Main-window test-fixture gap)
// ═══════════════════════════════════════════════════════════════════════════
//
// UNLIKE the regular `Digivolve` alt-path above, the Main-phase user-action
// DNA route (`Game::initiate_dna_digivolve` -> `dna_route_for_hand_card` ->
// `get_dna_stacking_order`) reads ONLY `CardData.dna_costs` — it does NOT
// consult `alt_path_registry` / `CompiledAltPathKind::DnaDigivolve` for the
// `DnaRouteWindow::Main` window (registered_dna_route_match is wired only
// for `DnaRouteWindow::EndOfTurnAction`, per `dna_digivolve.rs`). In
// PRODUCTION this is a non-issue: `CardData` for a real card ID is loaded
// from `data/cards.json` (which DOES carry P-220's printed `dna_costs`
// entry — `name_contains: "Kimeramon"` / `"Machinedramon"`, cost 0), so the
// Main-window DNA route resolves correctly at runtime. It is ONLY the
// synthetic `DebugRunner::builder().dsl_card(id)` TEST fixture that leaves
// `CardData.dna_costs` empty (`card_data_from_compiled` derives CardData
// purely from the compiled DSL spec, which carries no `dna_costs` field —
// see `qa/dsl-vocab-gaps.md` "DebugRunner empty evo_costs" precedent, which
// documents the identical situation for `evo_costs`). The structural
// assertion above (`p_220_alt_paths_cover_digivolve_dna_and_assembly`)
// already pins the DNA alt-path's shape (2 materials, cost 0,
// `stacks_unsuspended: true`) against the compiled card the SAME production
// pack ships. A behavioral Main-window test would require patching a
// synthetic `CardData.dna_costs` entry via `.add_card(...)` BEFORE
// `.dsl_card("P-220")` (the `.entry().or_insert_with()` merge order in
// `register_compiled_card` preserves a pre-seeded entry) — that exercises
// the LEGACY `dna_costs` lookup path, not the DSL `alt_paths` the card
// actually authors, so it would not add faithfulness signal beyond the
// structural test already above. No engine/DSL vocab gap is filed for this:
// production DNA digivolve for P-220 is unaffected; it is a test-fixture
// coverage note only, not a card-authoring defect.
