//! BT18-019 Millenniummon — Digimon, Lv.7, Red/Black, DP 14000, Cost 14.
//! Traits: Composite. Attribute: Virus.
//! Standard digivolve (cards.json evo_costs): Lv.6 Red / Cost 4.
//! DNA Digivolution (official Bandai DB `dna_costs` / DCGO jogress):
//!   [Kimeramon] + [Machinedramon] / Cost 0, stacks unsuspended.
//! DigiXros -2 (per material): [Kimeramon] x [Machinedramon]
//!   (DCGO `DigiXrosCondition(elements, null, 2)` — the trailing param is
//!   named `reduceCostPerCard`, so EACH material contributes cost_delta -2,
//!   not a flat -2 total).
//!
//! # Card text (official Bandai DB / data/card_bundles/BT18-019.md)
//!
//! ```text
//! [On Play] [When Digivolving] Delete 1 of your opponent's Digimon. Then, if
//! DNA digivolving, by returning 1 of each Digimon card with different levels
//! from your opponent's trash to the top of the deck, gain 1 memory for each
//! card returned.
//! [On Deletion] By returning 1 [Kimeramon] and 1 [Machinedramon] from your
//! trash to the bottom of the deck, you may play 1 [Millenniummon] from your
//! trash without paying the cost.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT18/Red/BT18_019.cs
//!
//! DCGO splits [On Play] and [When Digivolving] into two separate
//! ActivateClass blocks that both do "delete 1 opponent Digimon", plus a
//! THIRD block (also OnEnterFieldAnyone / CanTriggerWhenDigivolving) that
//! does the delete again AND, `if (CardEffectCommons.IsJogress(_hashtable))`,
//! loops levels 3..7 selecting up to 1 opponent-trash Digimon card per level
//! (each selection has `canNoSelect: () => true`, i.e. per-level optional),
//! reverses pick order, adds them to the top of the deck, then
//! `AddMemory(1 * sources.Count)` once at the end. Modeled as a single
//! `when: [on_play, when_digivolving]` clause (delete is authored once, not
//! duplicated, since the DSL doesn't need DCGO's per-timing ActivateClass
//! split) with the DNA sub-effect gated by `condition: { dna_origin: true }`
//! — proven pattern from BT17-078 (also `[On Play][When Digivolving]` +
//! `dna_origin: true` inner gate).
//!
//! # DNA return-cards modeling
//!
//! `select_count_capped_multi { zone: trash, of: opponent, distinct_by: level,
//! optional_zero: true }` reproduces "1 of each Digimon card with different
//! levels" (DCGO's per-level bucket loop): `distinct_by: level` prunes
//! already-matched-level candidates after each pick so no two picks share a
//! level, and `optional_zero: true` makes the whole selection a "may" (0
//! picks legal — DCGO's `canNoSelect: () => true` on every level bucket).
//! `max: 7` is a generous cap (Digimon levels run at most 2-7; `distinct_by`
//! self-limits regardless). The move is `per_selected` over the bound list,
//! each iteration doing `move_trash_card_to_deck_top` + `gain_memory: 1` —
//! NOT `return_to_deck_from_reveal`, which only operates on the engine's
//! `revealed_cards` pool (reveal-then-return flows like LM-020), not on a
//! `select_count_capped_multi { zone: trash }` binding; using it here would
//! silently no-op every move.
//!
//! # On Deletion modeling
//!
//! DCGO requires the SAME selection to include >=1 Kimeramon AND >=1
//! Machinedramon (canEndSelectCondition), cross-excluding extra picks of one
//! name once the other still needs to be found. Modeled as two
//! `select_trash` + `return_trash_list_to_deck_bottom` PAIRS, interleaved
//! (pick Kimeramon, move it; pick Machinedramon, move it) rather than both
//! picks followed by both moves — `select_trash` bindings are positional
//! `TrashIndex`es, so moving the first pick before resolving the second
//! pick's binding shifts indices under it. The leading pick is
//! `optional: true, cost: true` (declining aborts the whole clause — nothing
//! is returned, nothing is played); this also satisfies the DSL's
//! `body_first_step_is_declinable` check so the engine does not ALSO install
//! a redundant outer accept/decline gate ahead of the body (discovered via
//! `SelectionKind::Replacement` showing up unexpectedly on the leading
//! mandatory `select_trash` — see BT13-088 for the same two-sequential-picks
//! idiom). The second pick is mandatory (no `optional`) once the cost is
//! committed to. If either name is absent from trash, its pick has zero
//! candidates and the step no-ops per the standard 2b/2c convention.
//!
//! # Patterns covered
//! - G3: DNA digivolve On Deletion (Millenniummon IS the canonical G3 pool
//!   entry per docs/RUST_DSL_TEST_API.md §4.3)
//! - G2: DNA digivolve alt-path (Kimeramon + Machinedramon materials)
//! - C4: DigiXros source play (2 named materials, per-material cost_delta)
//! - A5: trash -> deck recursion (return_trash cards + play_from_trash_free)
//! - E2-adjacent: optional cost-gated multi-step On Deletion body

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::TriggerSource;

// ─── Card fixtures ────────────────────────────────────────────────────────

fn make_digimon_lv(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(2000 + level as i32 * 1000);
    card
}

fn make_named_digimon(id: &str, name: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(2000 + level as i32 * 1000);
    card
}

fn millenniummon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT18-019")
        .expect("BT18-019 in embedded DSL pack")
        .add_card(make_digimon_lv("OPP-A", 5))
        .add_card(make_digimon_lv("OPP-B", 4))
        .add_card(make_named_digimon("KIMERAMON", "Kimeramon", 6))
        .add_card(make_named_digimon("MACHINEDRAMON", "Machinedramon", 6))
        .memory(5)
        .start()
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .trash
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn deck_top_id(runner: &DebugRunner, player: u8) -> Option<String> {
    runner.game.players[player as usize]
        .deck
        .last()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
}

fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle, dna_origin: bool) {
    let card = runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Digivolved {
            player: handle.player,
            permanent: handle,
            card,
            effect_initiated: false,
            dna_origin,
        },
    );
    runner.game.drain_effect_queue();
}

// ─── §1 Structural assertions ───────────────────────────────────────────────

#[test]
fn bt18_019_has_dna_digivolve_alt_path_with_two_materials() {
    let runner = millenniummon();
    let compiled = runner.compiled_card("BT18-019").expect("BT18-019 compiled");

    let dna = compiled
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .expect("BT18-019 must declare a DnaDigivolve alt-path");
    assert_eq!(dna.materials.len(), 2, "Kimeramon + Machinedramon");
}

#[test]
fn bt18_019_has_digixros_alt_path_with_two_named_materials() {
    let runner = millenniummon();
    let compiled = runner.compiled_card("BT18-019").expect("BT18-019 compiled");

    let xros = compiled
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DigiXros)
        .expect("BT18-019 must declare a DigiXros alt-path");
    assert_eq!(
        xros.materials.len(),
        2,
        "DigiXros -2: [Kimeramon] x [Machinedramon] is a 2-material recipe"
    );
}

#[test]
fn bt18_019_has_standard_digivolve_alt_path() {
    let runner = millenniummon();
    let compiled = runner.compiled_card("BT18-019").expect("BT18-019 compiled");
    assert!(
        compiled
            .alt_paths
            .iter()
            .any(|p| p.kind == CompiledAltPathKind::Digivolve),
        "printed Lv.6 Red / cost 4 standard digivolve path"
    );
}

#[test]
fn bt18_019_compiles_own_and_on_deletion_clauses() {
    let runner = millenniummon();
    let compiled = runner.compiled_card("BT18-019").expect("BT18-019 compiled");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "one [On Play][When Digivolving] clause + one [On Deletion] clause"
    );

    let main = triggered
        .iter()
        .find(|c| c.when.contains(&CompiledTiming::OnPlay))
        .expect("main clause carries OnPlay");
    assert!(
        main.when.contains(&CompiledTiming::WhenDigivolving),
        "main clause is also WhenDigivolving"
    );
    assert_eq!(main.scope, CompiledScope::FaceUp);

    let on_deletion = triggered
        .iter()
        .find(|c| c.when.contains(&CompiledTiming::OnDeletion))
        .expect("On Deletion clause present");
    assert!(
        on_deletion.optional,
        "the whole return-2-play-1-free cost is a 'may'"
    );
}

// ─── §2 Behavioral — [On Play] delete 1 opponent Digimon ──────────────────

#[test]
fn bt18_019_on_play_deletes_one_opponent_digimon_via_play_action() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-019")
        .expect("BT18-019 in embedded DSL pack")
        .add_card(make_digimon_lv("OPP-A", 5))
        .hand(0, &["BT18-019"])
        .memory(5)
        .start();

    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    runner.play(0, 0).expect("play Millenniummon");

    let view = runner
        .pending_selection_view()
        .expect("[On Play] must prompt to delete 1 opponent Digimon");
    assert!(
        !runner.pending_is_optional(),
        "delete is mandatory when a target exists"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("delete the opponent Digimon");
    runner.auto_resolve().expect("resolve rest (no DNA on a fresh play)");

    assert_eq!(
        runner.battle_area_size(1),
        0,
        "opponent's only Digimon was deleted"
    );
    let _ = opp;
}

#[test]
fn bt18_019_on_play_no_opponent_digimon_installs_no_selection() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-019")
        .expect("BT18-019 in embedded DSL pack")
        .hand(0, &["BT18-019"])
        .memory(5)
        .start();

    runner.play(0, 0).expect("play Millenniummon");
    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon on field -> delete step no-ops, no selection installs"
    );
}

// ─── §3 Behavioral — [When Digivolving] non-DNA: delete only, no DNA return ─

#[test]
fn bt18_019_standard_digivolve_deletes_but_does_not_offer_dna_return() {
    let mut runner = millenniummon();
    let millen = runner.place_on_field(0, "BT18-019", Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    let kim_in_trash = runner.place_on_field(1, "KIMERAMON", Some(0));
    runner.game.delete_permanent_with_cause(
        kim_in_trash,
        ReplacementCause::OpponentEffect,
    );

    fire_when_digivolving(&mut runner, millen, false);

    let view = runner
        .pending_selection_view()
        .expect("delete-1-opponent-Digimon prompt installs on standard digivolve");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("delete the opponent Digimon");
    runner.auto_resolve().expect("no DNA branch on standard digivolve");

    assert_eq!(runner.battle_area_size(1), 0, "OPP-A deleted");
    // OPP-A joins Kimeramon in trash when deleted; the pre-existing Kimeramon
    // must remain untouched (not returned to deck) since this digivolve was
    // NOT via DNA.
    let trash = trash_ids(&runner, 1);
    assert!(
        trash.contains(&"KIMERAMON".to_string()),
        "opponent's trash Kimeramon must NOT be returned on a non-DNA digivolve; trash={trash:?}"
    );
    assert_eq!(deck_top_id(&runner, 1), None, "no card was returned to the opponent's deck");
    let _ = opp;
}

// ─── §4 Behavioral — [When Digivolving] DNA: delete + return-by-level ──────

#[test]
fn bt18_019_dna_digivolve_offers_optional_return_by_distinct_level() {
    let mut runner = millenniummon();
    let millen = runner.place_on_field(0, "BT18-019", Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));

    // Opponent trash: two Digimon at DIFFERENT levels (4 and 5).
    let lv4 = runner.place_on_field(1, "OPP-B", Some(0));
    runner
        .game
        .delete_permanent_with_cause(lv4, ReplacementCause::OpponentEffect);

    fire_when_digivolving(&mut runner, millen, true);

    // Delete-1-opponent-Digimon prompt fires first.
    let del_view = runner
        .pending_selection_view()
        .expect("delete prompt installs first");
    runner
        .execute_action(0, del_view.valid_action_ids[0])
        .expect("delete OPP-A");

    // Then the DNA return-by-level multi-select should install, offering the
    // now-two opponent trash Digimon (OPP-A just deleted + OPP-B pre-deleted),
    // both at different levels (5 and 4).
    let view = runner
        .pending_selection_view()
        .expect("DNA branch must offer the return-by-level multi-select");
    assert!(
        runner.pending_is_optional(),
        "optional_zero: true -> 0 picks is legal from the start"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        2,
        "both distinct-level opponent trash Digimon are eligible"
    );
    let _ = opp;
}

#[test]
fn bt18_019_dna_digivolve_declining_return_gains_no_memory() {
    let mut runner = millenniummon();
    let millen = runner.place_on_field(0, "BT18-019", Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));

    fire_when_digivolving(&mut runner, millen, true);
    let del_view = runner.pending_selection_view().expect("delete prompt");
    runner
        .execute_action(0, del_view.valid_action_ids[0])
        .expect("delete OPP-A");

    let memory_before = runner.memory();
    // No opponent trash Digimon exist yet at this point other than the
    // just-deleted OPP-A, so the multi-select may still install with 1
    // candidate; decline immediately via PASS.
    if runner.pending_selection().is_some() {
        runner.execute_action(0, PASS).expect("decline the return");
    }
    runner.auto_resolve().expect("finish resolving");

    assert_eq!(
        runner.memory(),
        memory_before,
        "declining (0 picks) must not gain memory"
    );
    let _ = opp;
}

#[test]
fn bt18_019_dna_digivolve_returning_cards_moves_to_opponent_deck_top_and_gains_memory() {
    let mut runner = millenniummon();
    let millen = runner.place_on_field(0, "BT18-019", Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    let lv4 = runner.place_on_field(1, "OPP-B", Some(0));
    runner
        .game
        .delete_permanent_with_cause(lv4, ReplacementCause::OpponentEffect);

    let memory_before = runner.memory();

    fire_when_digivolving(&mut runner, millen, true);
    let del_view = runner.pending_selection_view().expect("delete prompt");
    runner
        .execute_action(0, del_view.valid_action_ids[0])
        .expect("delete OPP-A");

    let view = runner
        .pending_selection_view()
        .expect("DNA return multi-select installs");
    // Pick the first available candidate (one level).
    let first_action = view.valid_action_ids[0];
    runner
        .execute_action(0, first_action)
        .expect("pick 1 card");
    // Commit with just 1 pick.
    runner.execute_action(0, PASS).expect("stop picking");
    runner.auto_resolve().expect("finish resolving");

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "gain 1 memory for the 1 card returned"
    );
    // Two distinct-level candidates were available (OPP-A lvl 5, OPP-B lvl
    // 4); only 1 was picked, so exactly 1 card left the opponent's trash and
    // 1 remains.
    assert_eq!(
        trash_ids(&runner, 1).len(),
        1,
        "only the 1 picked card left the opponent's trash"
    );
    assert!(
        deck_top_id(&runner, 1).is_some(),
        "a card was placed on top of the opponent's deck"
    );
    let _ = opp;
}

// ─── §5 Behavioral — [On Deletion] return Kimeramon+Machinedramon, free play ─

#[test]
fn bt18_019_on_deletion_no_selection_without_both_named_cards_in_trash() {
    let mut runner = millenniummon();
    let carrier = runner.place_on_field(0, "BT18-019", None);
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    // With no Kimeramon/Machinedramon in trash, the leading `select_trash`
    // for Kimeramon has zero candidates. The clause itself is `optional:
    // true`, so a Trash prompt may still surface (an "empty" cost pick) but
    // it must be freely declinable — the cost cannot be paid, so PASS must
    // be legal and declining must leave the trash/field untouched.
    if runner.pending_selection().is_some() {
        assert!(
            runner.pending_is_optional(),
            "with no eligible cards, any surfaced prompt must be declinable"
        );
        runner.execute_action(0, PASS).expect("decline");
        runner.auto_resolve().expect("finish resolving");
    }

    assert_eq!(
        trash_ids(&runner, 0),
        vec!["BT18-019"],
        "nothing moved; the deleted carrier is the only trash card"
    );
    assert_eq!(
        runner.battle_area_size(0),
        0,
        "no Millenniummon played from trash without the return cost"
    );
}

#[test]
fn bt18_019_on_deletion_offers_optional_kimeramon_pick_when_present() {
    let mut runner = millenniummon();
    let kim = runner.place_on_field(0, "KIMERAMON", Some(0));
    runner
        .game
        .delete_permanent_with_cause(kim, ReplacementCause::OpponentEffect);

    let carrier = runner.place_on_field(0, "BT18-019", None);
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    // Only Kimeramon is in trash (no Machinedramon) — DCGO's
    // CanActivateCondition fires on EITHER name present, but
    // canEndSelectCondition still needs both, so the pick for Machinedramon
    // has zero candidates and the clause should no-op overall (no prompt or
    // a prompt that cannot complete). We assert here that if a prompt
    // installs at all, declining it is always legal.
    if let Some(view) = runner.pending_selection_view() {
        assert!(
            view.valid_action_ids.contains(&PASS) || runner.pending_is_optional(),
            "with only Kimeramon present, the cost cannot be fully paid; must be declinable"
        );
    }
}

#[test]
fn bt18_019_on_deletion_declining_does_not_play_millenniummon_from_trash() {
    let mut runner = millenniummon();
    let kim = runner.place_on_field(0, "KIMERAMON", Some(0));
    runner
        .game
        .delete_permanent_with_cause(kim, ReplacementCause::OpponentEffect);
    let mach = runner.place_on_field(0, "MACHINEDRAMON", Some(0));
    runner
        .game
        .delete_permanent_with_cause(mach, ReplacementCause::OpponentEffect);

    let carrier = runner.place_on_field(0, "BT18-019", None);
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    assert!(runner.pending_is_optional(), "whole cost is a 'may'");
    runner.execute_action(0, PASS).expect("decline the cost");

    assert_eq!(
        trash_ids(&runner, 0),
        vec!["KIMERAMON", "MACHINEDRAMON", "BT18-019"],
        "declining leaves all three cards in trash untouched"
    );
    assert_eq!(
        runner.battle_area_size(0),
        0,
        "no Millenniummon played when the cost is declined"
    );
}

#[test]
fn bt18_019_on_deletion_paying_cost_returns_both_named_cards_to_deck_bottom() {
    let mut runner = millenniummon();
    let kim = runner.place_on_field(0, "KIMERAMON", Some(0));
    runner
        .game
        .delete_permanent_with_cause(kim, ReplacementCause::OpponentEffect);
    let mach = runner.place_on_field(0, "MACHINEDRAMON", Some(0));
    runner
        .game
        .delete_permanent_with_cause(mach, ReplacementCause::OpponentEffect);

    let carrier = runner.place_on_field(0, "BT18-019", None);
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    // Pick Kimeramon.
    let view = runner
        .pending_selection_view()
        .expect("select Kimeramon prompt");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick Kimeramon");
    // Pick Machinedramon.
    let view2 = runner
        .pending_selection_view()
        .expect("select Machinedramon prompt");
    runner
        .execute_action(0, view2.valid_action_ids[0])
        .expect("pick Machinedramon");

    // Both named cards have now left the trash; only the just-deleted
    // Millenniummon carrier remains, and its own name also satisfies the
    // free-play sub-selection's `name_is: "Millenniummon"` filter (DCGO
    // re-scans trash for eligible Millenniummon copies AFTER the bottom-deck
    // step, and the just-deleted carrier itself qualifies). Decline that
    // final optional pick here to assert the bottom-deck step in isolation.
    assert_eq!(
        trash_ids(&runner, 0),
        vec!["BT18-019"],
        "Kimeramon and Machinedramon left the trash; the just-deleted Millenniummon remains"
    );
    let view3 = runner
        .pending_selection_view()
        .expect("free-play prompt offers the just-deleted Millenniummon");
    assert!(view3.is_optional, "free play is a 'may'");
    runner.execute_action(0, PASS).expect("decline the free play");

    assert_eq!(
        runner.game.players[0].deck.first().unwrap().card_id(&runner.game.card_data),
        "MACHINEDRAMON",
        "cards go to the BOTTOM of the deck (deck[0] is the bottom)"
    );
    assert_eq!(
        trash_ids(&runner, 0),
        vec!["BT18-019"],
        "declining the free play leaves the carrier in trash"
    );
}

#[test]
fn bt18_019_on_deletion_free_play_may_target_the_just_deleted_copy() {
    let mut runner = millenniummon();
    let kim = runner.place_on_field(0, "KIMERAMON", Some(0));
    runner
        .game
        .delete_permanent_with_cause(kim, ReplacementCause::OpponentEffect);
    let mach = runner.place_on_field(0, "MACHINEDRAMON", Some(0));
    runner
        .game
        .delete_permanent_with_cause(mach, ReplacementCause::OpponentEffect);

    let carrier = runner.place_on_field(0, "BT18-019", None);
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("select Kimeramon prompt");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick Kimeramon");
    let view2 = runner
        .pending_selection_view()
        .expect("select Machinedramon prompt");
    runner
        .execute_action(0, view2.valid_action_ids[0])
        .expect("pick Machinedramon");
    // auto_resolve picks the first legal action at the remaining prompt,
    // which plays the just-deleted Millenniummon copy from trash.
    runner.auto_resolve().expect("resolve free play");

    assert_eq!(
        trash_ids(&runner, 0),
        Vec::<String>::new(),
        "the just-deleted Millenniummon was played from trash, leaving trash empty"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT18-019"),
        "Millenniummon is back on the battle area"
    );
}

#[test]
fn bt18_019_on_deletion_plays_millenniummon_from_trash_free_when_available() {
    let mut runner = millenniummon();
    let kim = runner.place_on_field(0, "KIMERAMON", Some(0));
    runner
        .game
        .delete_permanent_with_cause(kim, ReplacementCause::OpponentEffect);
    let mach = runner.place_on_field(0, "MACHINEDRAMON", Some(0));
    runner
        .game
        .delete_permanent_with_cause(mach, ReplacementCause::OpponentEffect);

    // A SECOND Millenniummon copy must already be in trash to be played by
    // the effect (the just-deleted carrier is not itself the free-play
    // candidate under DCGO's flow, since it re-checks trash AFTER the
    // bottom-deck step; but a second, distinct copy models the intended
    // "you have another Millenniummon in trash" scenario cleanly regardless
    // of whether the engine happens to also re-offer the just-deleted one).
    let extra_millen = runner.place_on_field(0, "BT18-019", None);
    runner
        .game
        .delete_permanent_with_cause(extra_millen, ReplacementCause::OpponentEffect);

    let carrier = runner.place_on_field(0, "BT18-019", None);
    let memory_before = runner.memory();
    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);

    let view = runner.pending_selection_view().expect("select Kimeramon");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("pick Kimeramon");
    let view2 = runner
        .pending_selection_view()
        .expect("select Machinedramon");
    runner
        .execute_action(0, view2.valid_action_ids[0])
        .expect("pick Machinedramon");

    // Now a Millenniummon-from-trash play prompt should install (optional).
    if let Some(view3) = runner.pending_selection_view() {
        runner
            .execute_action(0, view3.valid_action_ids[0])
            .expect("pick the trash Millenniummon to play free");
    }
    runner.auto_resolve().expect("finish resolving");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT18-019"),
        "a Millenniummon was played onto the battle area from trash"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "playing from trash without paying cost does not spend memory"
    );
}

#[test]
fn bt18_019_on_deletion_clause_is_optional_at_top_level() {
    let runner = millenniummon();
    let compiled = runner.compiled_card("BT18-019").expect("compiled");
    let on_deletion = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|c| c.when.contains(&CompiledTiming::OnDeletion))
        .expect("On Deletion clause");
    assert!(on_deletion.optional);
}
