//! BT24-059 Sharkmon — Digimon, Lv.5, Black+Blue, DP 7000, Cost 7.
//! Traits: [Cyborg], [Titan], [TS]. Attribute: Virus.
//!
//! # Card text (data/cards.json — verbatim)
//!
//! [On Play] [When Digivolving] ＜De-Digivolve 1＞ 1 of your opponent's Digimon.
//!   (Trash the top card. You can't trash past level 3 cards.)
//! [On Deletion] Reveal the top 3 cards of your deck. You may play 1 play cost 7
//!   or lower [TS] trait card suspended among them without paying the cost.
//!   Trash the rest.
//!
//! Inherited Effect:
//! [When Attacking] [Once Per Turn] By placing 1 of your other Digimon as this
//!   Digimon's bottom digivolution card, it unsuspends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Black/BT24_059.cs
//!
//! # Patterns covered
//! - Shared [On Play]/[When Digivolving] body → De-Digivolve 1 opponent Digimon
//!   (clause A — `de_digivolve { amount: 1, stop_at_level: 3 }`)
//! - [On Deletion] reveal top 3, optional play 1 [TS] cost<=7 card SUSPENDED for
//!   free, trash remainder (clause B — reveal-bucket + play_from_revealed_free +
//!   suspend the played permanent + trash-rest loop)
//! - Inherited [When Attacking][OPT] place another own Digimon (whole permanent)
//!   as this Digimon's bottom source → this Digimon unsuspends (clause C)
//! - Q12: a placed permanent (incl. a token / Petrification source) counts as a
//!   digivolution source → the unsuspend fires
//!
//! # play-from-reveal SUSPENDED resolution
//! `PlayFromRevealedFreeArgs` exposes `bind_as` but NO `suspended`/`tapped`
//! flag. The faithful path is: `play_from_revealed_free { bind_as: ts_play }`
//! then `suspend { target: ts_play }` on the just-played permanent. No
//! stack-changing event intervenes between the play and the suspend, so the
//! provenance-bound handle resolves cleanly.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS, SEL_REVEAL_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};
use digimon_engine::{CardData, PermanentHandle};

const CARD_ID: &str = "BT24-059";

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn sharkmon_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-059 YAML loads")
}

fn make_digimon(id: &str, traits: &[&str]) -> CardData {
    make_digimon_with_cost(id, traits, 3)
}

fn make_digimon_with_cost(id: &str, traits: &[&str], play_cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.play_cost = play_cost;
    card.level = Some(4);
    card.dp = Some(5000);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn make_ts_tamer(id: &str, play_cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Black];
    card.play_cost = play_cost;
    card.level = None;
    card.dp = None;
    card.traits = vec!["TS".to_string()];
    card
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn fire_timing(runner: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn target_action(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

fn battle_area_contains(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt24_059_yaml_compiles_and_has_printed_metadata() {
    let runner = sharkmon_runner().start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT24-059 present in embedded DSL pack");
    assert_eq!(compiled.card, "BT24-059");
    assert_eq!(compiled.level, Some(5));
    assert_eq!(compiled.cost, Some(7));
    assert_eq!(compiled.dp, Some(7000));
    for trait_name in ["Cyborg", "Titan", "TS"] {
        assert!(
            compiled.traits.iter().any(|t| t == trait_name),
            "missing trait {trait_name}"
        );
    }
}

/// Clause A — a shared On Play + When Digivolving triggered clause exists with
/// a De-Digivolve 1 (stop_at_level 3) step.
#[test]
fn bt24_059_has_shared_on_play_when_digivolving_de_digivolve_clause() {
    let runner = sharkmon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let clause = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("shared On Play / When Digivolving clause");
    assert!(
        clause.process.iter().any(|s| matches!(
            s,
            CompiledStep::DeDigivolve {
                amount: Some(1),
                stop_at_level: Some(3),
                ..
            }
        )),
        "shared clause must De-Digivolve 1 with stop_at_level 3"
    );
}

/// Clause A must NOT also carry On Deletion (the On Deletion body is the
/// reveal, a different effect).
#[test]
fn bt24_059_shared_clause_is_not_on_deletion() {
    let runner = sharkmon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let shared = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("shared clause");
    assert!(
        !shared.when.contains(&CompiledTiming::OnDeletion),
        "the De-Digivolve clause must not fire On Deletion"
    );
}

/// Clause B — an OnDeletion triggered clause exists and is mandatory at the
/// clause level (reveal + trash are mandatory; only the play step is optional).
#[test]
fn bt24_059_has_mandatory_on_deletion_reveal_clause() {
    let runner = sharkmon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let od = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnDeletion)
                    && !t.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OnDeletion clause");
    assert!(!od.optional, "On Deletion clause must not be optional");
    assert!(
        od.process
            .iter()
            .any(|s| matches!(s, CompiledStep::RevealTopDeck { count: 3, .. })),
        "On Deletion must reveal top 3"
    );
}

/// Clause B — the reveal bucket filters for [TS] trait cards costing <=7 and is
/// optional (min:0, max:1), consumed by PlayFromRevealedFree, and the played
/// permanent is suspended.
#[test]
fn bt24_059_on_deletion_plays_ts_card_suspended_for_free() {
    let runner = sharkmon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let od = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnDeletion)
                    && !t.when.contains(&CompiledTiming::OnPlay) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OnDeletion clause");

    // Optional [TS] cost<=7 bucket.
    let bucket_ok = od.process.iter().any(|s| {
        matches!(s, CompiledStep::SelectRevealBuckets { buckets, .. }
            if buckets.len() == 1
                && buckets[0].min == 0
                && buckets[0].max == 1)
    });
    assert!(bucket_ok, "must offer an optional single-pick reveal bucket");

    // Play the picked card for free, binding the played permanent.
    let play_binding = od
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::PlayFromRevealedFree {
                bind_as: Some(bound),
                ..
            } => Some(bound.clone()),
            _ => None,
        })
        .expect("play_from_revealed_free must bind the played permanent");

    // Suspend the just-played permanent (faithful "play suspended").
    assert!(
        od.process.iter().any(|s| matches!(
            s,
            CompiledStep::Suspend {
                target: CompiledBindingRef::Named(name),
            } if *name == play_binding
        )),
        "the played reveal card must be suspended via its bound handle"
    );

    // Trash the rest.
    let has_trash_rest = od.process.iter().any(|s| match s {
        CompiledStep::PerSelected { body, .. } => body
            .iter()
            .any(|inner| matches!(inner, CompiledStep::TrashFromReveal { .. })),
        _ => false,
    });
    assert!(has_trash_rest, "On Deletion must trash the remaining revealed cards");
}

/// Clause C — an inherited When Attacking clause exists, optional, once-per-turn.
#[test]
fn bt24_059_has_inherited_opt_when_attacking_clause() {
    let runner = sharkmon_runner().start();
    let compiled = runner.compiled_card(CARD_ID).expect("present");
    let inh = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited When Attacking clause");
    assert!(inh.optional, "inherited clause is 'By placing…' → optional");
    assert!(inh.once_per_turn, "inherited clause is [Once Per Turn]");

    // Places another permanent as bottom source, then unsuspends self. Both
    // are gated (inside an `If` on the placement binding) so the unsuspend only
    // follows a real placement; scan the whole step tree (top-level + If body).
    fn steps_flat(steps: &[CompiledStep]) -> Vec<&CompiledStep> {
        let mut out = Vec::new();
        for s in steps {
            out.push(s);
            if let CompiledStep::If {
                then, else_branch, ..
            } = s
            {
                out.extend(steps_flat(then));
                out.extend(steps_flat(else_branch));
            }
        }
        out
    }
    let flat = steps_flat(&inh.process);
    assert!(
        flat.iter()
            .any(|s| matches!(s, CompiledStep::PlaceAsBottomSource { .. })),
        "inherited clause must place a permanent as a bottom source"
    );
    assert!(
        flat.iter().any(|s| matches!(
            s,
            CompiledStep::Unsuspend {
                target: CompiledBindingRef::Source,
            }
        )),
        "inherited clause must unsuspend the source (this Digimon)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play] / [When Digivolving] De-Digivolve behavioral
// ═══════════════════════════════════════════════════════════════════════════════

/// [On Play] De-Digivolve 1 trashes the top source of the chosen opponent
/// Digimon and routes it to the opponent's trash.
#[test]
fn bt24_059_on_play_de_digivolves_one_opponent_digimon() {
    let mut runner = sharkmon_runner()
        .add_card(make_digimon("OPP-BASE", &[]))
        .add_card(make_digimon("OPP-TOP", &[]))
        .memory(10)
        .start();
    let shark = runner.place_on_field(0, CARD_ID, Some(0));
    let opponent = runner.place_stack(1, &["OPP-BASE", "OPP-TOP"]);
    let opp_trash_before = runner.trash_size(1);

    fire_timing(&mut runner, EffectTiming::OnPlay, shark);

    let prompt = runner
        .pending_selection_view()
        .expect("De-Digivolve target prompt");
    assert_eq!(prompt.kind, SelectionKind::OppField);
    assert_eq!(prompt.valid_action_ids, vec![target_action(opponent)]);

    runner
        .execute_action(0, target_action(opponent))
        .expect("select opponent Digimon");

    assert_eq!(
        runner.game.players[1].battle_area[opponent.index as usize].stack_size(),
        1,
        "De-Digivolve 1 must trash the top source"
    );
    assert_eq!(
        runner.trash_size(1),
        opp_trash_before + 1,
        "the trashed source goes to the opponent's trash"
    );
}

/// [When Digivolving] fires the same De-Digivolve body.
#[test]
fn bt24_059_when_digivolving_de_digivolves_one_opponent_digimon() {
    let mut runner = sharkmon_runner()
        .add_card(make_digimon("OPP-BASE", &[]))
        .add_card(make_digimon("OPP-TOP", &[]))
        .memory(10)
        .start();
    let shark = runner.place_on_field(0, CARD_ID, Some(0));
    let opponent = runner.place_stack(1, &["OPP-BASE", "OPP-TOP"]);

    fire_timing(&mut runner, EffectTiming::WhenDigivolving, shark);

    let prompt = runner
        .pending_selection_view()
        .expect("De-Digivolve target prompt on When Digivolving");
    assert_eq!(prompt.kind, SelectionKind::OppField);
    runner
        .execute_action(0, target_action(opponent))
        .expect("select opponent Digimon");
    assert_eq!(
        runner.game.players[1].battle_area[opponent.index as usize].stack_size(),
        1,
        "When Digivolving De-Digivolve must trash one source"
    );
}

/// Negative: De-Digivolve cannot trash past level-3 cards. A 2-card stack whose
/// base is level 3 (top is level 4) loses only the level-4 top; a stack already
/// at level 3 keeps its single card.
#[test]
fn bt24_059_de_digivolve_stops_at_level_three() {
    let mut runner = sharkmon_runner()
        .add_card(make_level_digimon("OPP-LV3", 3))
        .add_card(make_level_digimon("OPP-LV4", 4))
        .memory(10)
        .start();
    let shark = runner.place_on_field(0, CARD_ID, Some(0));
    // Stack: base LV3, top LV4 → De-Digivolve 1 trashes top (LV4), leaving LV3.
    let opponent = runner.place_stack(1, &["OPP-LV3", "OPP-LV4"]);

    fire_timing(&mut runner, EffectTiming::OnPlay, shark);
    runner
        .execute_action(0, target_action(opponent))
        .expect("select opponent Digimon");

    let perm = &runner.game.players[1].battle_area[opponent.index as usize];
    assert_eq!(perm.stack_size(), 1, "De-Digivolve removes the LV4 top");
    assert_eq!(perm.top_card().card_id(&runner.game.card_data), "OPP-LV3");
}

/// Negative: with no opponent Digimon, the shared clause installs no selection
/// (nothing to De-Digivolve).
#[test]
fn bt24_059_on_play_no_opponent_digimon_installs_no_selection() {
    let mut runner = sharkmon_runner().memory(10).start();
    let shark = runner.place_on_field(0, CARD_ID, Some(0));

    fire_timing(&mut runner, EffectTiming::OnPlay, shark);

    assert!(
        runner.pending_selection().is_none(),
        "no De-Digivolve target → no selection installs"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [On Deletion] reveal + play-suspended-from-reveal behavioral
// ═══════════════════════════════════════════════════════════════════════════════

/// Deleting Sharkmon with an eligible [TS] cost<=7 card in the top 3 installs an
/// optional reveal-bucket selection.
#[test]
fn bt24_059_on_deletion_installs_optional_ts_selection() {
    let ts = make_digimon_with_cost("TS-DIGI", &["TS"], 5);
    let mut runner = sharkmon_runner()
        .add_card(ts)
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        // TS-DIGI on top of deck (last in array) → revealed index 0.
        .deck(0, &["FILL-2", "FILL-1", "TS-DIGI"])
        .memory(0)
        .start();

    let shark = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .delete_permanent_with_cause(shark, digimon_engine::replacement::ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("reveal bucket selection installs");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { .. }));
    assert!(runner.pending_is_optional(), "'You may play' → optional");
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "the TS card at reveal index 0 must be a legal pick"
    );
}

/// Picking the [TS] card plays it onto the field SUSPENDED for free and trashes
/// the other 2 revealed cards.
#[test]
fn bt24_059_on_deletion_plays_ts_card_suspended_and_trashes_rest() {
    let ts = make_digimon_with_cost("TS-DIGI", &["TS"], 7);
    let mut runner = sharkmon_runner()
        .add_card(ts)
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "TS-DIGI"])
        .memory(0) // cannot afford cost-7 normally — must be free
        .start();

    let shark = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .delete_permanent_with_cause(shark, digimon_engine::replacement::ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    let field_before = runner.battle_area_size(0);
    let trash_before = runner.trash_size(0);
    let memory_before = runner.memory();

    runner
        .execute_action(0, SEL_REVEAL_START)
        .expect("pick the TS card");
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    assert!(
        battle_area_contains(&runner, 0, "TS-DIGI"),
        "the TS card must be played onto the field"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "exactly one card played"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "play_from_revealed_free must not spend memory"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 2,
        "the 2 remaining revealed cards are trashed"
    );

    let played = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == "TS-DIGI")
        .expect("TS-DIGI on field");
    assert!(
        played.is_suspended,
        "the played [TS] card must enter SUSPENDED (DCGO isTapped: true)"
    );
}

/// A [TS] Tamer costing <=7 is also an eligible reveal pick (DCGO:
/// IsDigimon || IsTamer).
#[test]
fn bt24_059_on_deletion_ts_tamer_is_eligible() {
    let tamer = make_ts_tamer("TS-TAMER", 4);
    let mut runner = sharkmon_runner()
        .add_card(tamer)
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "TS-TAMER"])
        .memory(0)
        .start();

    let shark = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .delete_permanent_with_cause(shark, digimon_engine::replacement::ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("reveal bucket selection installs for a TS Tamer");
    assert!(
        view.valid_action_ids.contains(&SEL_REVEAL_START),
        "TS Tamer at index 0 must be a legal pick"
    );
}

/// Negative: a non-[TS] card and a [TS] card costing 8 are NOT eligible.
#[test]
fn bt24_059_on_deletion_non_ts_and_overcost_not_eligible() {
    let non_ts = make_digimon_with_cost("NON-TS", &[], 3);
    let ts8 = make_digimon_with_cost("TS-8", &["TS"], 8);
    let mut runner = sharkmon_runner()
        .add_card(non_ts)
        .add_card(ts8)
        .add_card(make_filler("FILL-1"))
        .deck(0, &["FILL-1", "TS-8", "NON-TS"])
        .memory(0)
        .start();

    let shark = runner.place_on_field(0, CARD_ID, Some(0));
    runner
        .game
        .delete_permanent_with_cause(shark, digimon_engine::replacement::ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    // No eligible card → bucket auto-skips (no selection), or, if installed, the
    // ineligible cards are not legal picks.
    if let Some(view) = runner.pending_selection_view() {
        assert!(
            !view.valid_action_ids.iter().any(|a| *a >= SEL_REVEAL_START),
            "neither the non-TS card nor the cost-8 TS card may be picked: {:?}",
            view.valid_action_ids
        );
    }
}

/// Declining the optional play still trashes all 3 revealed cards.
#[test]
fn bt24_059_on_deletion_decline_trashes_all_three() {
    let ts = make_digimon_with_cost("TS-DIGI", &["TS"], 5);
    let mut runner = sharkmon_runner()
        .add_card(ts)
        .add_card(make_filler("FILL-1"))
        .add_card(make_filler("FILL-2"))
        .deck(0, &["FILL-2", "FILL-1", "TS-DIGI"])
        .memory(0)
        .start();

    let shark = runner.place_on_field(0, CARD_ID, Some(0));
    let trash_before = runner.trash_size(0);
    runner
        .game
        .delete_permanent_with_cause(shark, digimon_engine::replacement::ReplacementCause::OpponentEffect);
    runner.game.drain_effect_queue();

    if runner.pending_selection_view().is_some() {
        let _ = runner.execute_action(0, PASS);
        runner.game.drain_effect_queue();
    }
    let _ = runner.auto_resolve();

    assert!(
        runner.trash_size(0) >= trash_before + 3,
        "declining the play trashes all 3 revealed cards"
    );
    assert!(
        !battle_area_contains(&runner, 0, "TS-DIGI"),
        "nothing is played when the optional pick is declined"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Inherited [When Attacking][OPT] place-source → unsuspend behavioral
// ═══════════════════════════════════════════════════════════════════════════════
//
// The inherited effect comes from Sharkmon-as-a-digivolution-SOURCE under a
// live carrier Digimon. "this Digimon" = the carrier permanent. So the test
// stack is [BT24-059 (source), CARRIER (top/live)], and the inherited When
// Attacking fires on the carrier handle (mirrors bt24_020's setup).

/// Re-resolve the carrier permanent (the live Digimon hosting Sharkmon as a
/// digivolution source) by its top card id.
fn carrier_handle(runner: &DebugRunner) -> PermanentHandle {
    let idx = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "CARRIER")
        .expect("carrier on field");
    PermanentHandle {
        player: 0,
        index: idx as u8,
    }
}

/// Accepting the inherited trigger places another of your Digimon (whole
/// permanent) as this Digimon's bottom source, then unsuspends this Digimon.
#[test]
fn bt24_059_inherited_place_source_unsuspends_this_digimon() {
    let mut runner = sharkmon_runner()
        .add_card(make_digimon("CARRIER", &[]))
        .add_card(make_digimon("ALLY", &[]))
        .memory(10)
        .start();

    // Sharkmon under a live carrier Digimon; ALLY is the "other" Digimon.
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.place_on_field(0, "ALLY", Some(0));

    let carrier = carrier_handle(&runner);
    let stack_before = runner.game.players[0].battle_area[carrier.index as usize].stack_size();
    let area_before = runner.battle_area_size(0);

    // Suspend the carrier, then fire its inherited When Attacking.
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = true;
    fire_timing(&mut runner, EffectTiming::WhenAttacking, carrier);

    // Accept the optional "place 1 of your other Digimon" selection.
    let view = runner
        .pending_selection_view()
        .expect("place-source selection installs");
    assert!(runner.pending_is_optional(), "'By placing…' → optional");
    let pick = view
        .valid_action_ids
        .iter()
        .find(|a| **a != PASS)
        .copied()
        .expect("ALLY must be a legal placement target");
    runner.execute_action(0, pick).expect("place ALLY");
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    let carrier = carrier_handle(&runner);
    assert!(
        !runner.game.players[0].battle_area[carrier.index as usize].is_suspended,
        "placing a Digimon as bottom source must unsuspend this Digimon"
    );
    assert!(
        runner.game.players[0].battle_area[carrier.index as usize].stack_size() > stack_before,
        "the placed Digimon (whole permanent) joins the carrier's stack"
    );
    assert_eq!(
        runner.battle_area_size(0),
        area_before - 1,
        "the placed Digimon leaves the battle area as a source"
    );
}

/// Negative: with no OTHER own Digimon to place, the inherited clause installs
/// no selection.
#[test]
fn bt24_059_inherited_no_other_digimon_no_selection() {
    let mut runner = sharkmon_runner()
        .add_card(make_digimon("CARRIER", &[]))
        .memory(10)
        .start();
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let carrier = carrier_handle(&runner);
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = true;

    fire_timing(&mut runner, EffectTiming::WhenAttacking, carrier);

    assert!(
        runner.pending_selection().is_none(),
        "no other Digimon to place → no selection installs"
    );
    assert!(
        runner.game.players[0].battle_area[carrier.index as usize].is_suspended,
        "the carrier stays suspended when nothing is placed"
    );
}

/// OPT lockout: once the inherited clause fires this turn, a second fire does
/// not re-install the placement selection.
#[test]
fn bt24_059_inherited_is_once_per_turn() {
    let mut runner = sharkmon_runner()
        .add_card(make_digimon("CARRIER", &[]))
        .add_card(make_digimon("ALLY-A", &[]))
        .add_card(make_digimon("ALLY-B", &[]))
        .memory(10)
        .start();
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.place_on_field(0, "ALLY-A", Some(0));
    runner.place_on_field(0, "ALLY-B", Some(0));

    let carrier = carrier_handle(&runner);
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = true;
    fire_timing(&mut runner, EffectTiming::WhenAttacking, carrier);

    // Resolve the first activation.
    if let Some(view) = runner.pending_selection_view() {
        if let Some(pick) = view.valid_action_ids.iter().find(|a| **a != PASS).copied() {
            let _ = runner.execute_action(0, pick);
            runner.game.drain_effect_queue();
            let _ = runner.auto_resolve();
        }
    }

    // Fire again the same turn.
    let carrier = carrier_handle(&runner);
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = true;
    fire_timing(&mut runner, EffectTiming::WhenAttacking, carrier);

    if let Some(view) = runner.pending_selection_view() {
        assert!(
            !matches!(view.kind, SelectionKind::OwnField),
            "OPT must block a second placement selection on the same turn"
        );
    }
}

/// Q12: a placed permanent (token or any other Digimon) counts as a digivolution
/// source, so the unsuspend fires. We model the "token as source" case with a
/// 1-card permanent placed as the carrier's bottom source.
#[test]
fn bt24_059_inherited_q12_token_source_counts_and_unsuspends() {
    let mut runner = sharkmon_runner()
        .add_card(make_digimon("CARRIER", &[]))
        .add_card(make_digimon("TOKEN-LIKE", &[]))
        .memory(10)
        .start();
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    // A single-card permanent (stands in for a Petrification token permanent).
    runner.place_on_field(0, "TOKEN-LIKE", Some(0));

    let carrier = carrier_handle(&runner);
    runner.game.players[0].battle_area[carrier.index as usize].is_suspended = true;

    fire_timing(&mut runner, EffectTiming::WhenAttacking, carrier);
    let view = runner
        .pending_selection_view()
        .expect("placement selection installs");
    let pick = view
        .valid_action_ids
        .iter()
        .find(|a| **a != PASS)
        .copied()
        .expect("token-like permanent is a legal placement");
    runner.execute_action(0, pick).expect("place token-like source");
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();

    let carrier = carrier_handle(&runner);
    assert!(
        !runner.game.players[0].battle_area[carrier.index as usize].is_suspended,
        "Q12: a token/permanent placed as a source counts → this Digimon unsuspends"
    );
}

// ─── extra card builder used only by the level-stop test ────────────────────

fn make_level_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.play_cost = 3;
    card.level = Some(level);
    card.dp = Some(i32::from(level) * 1000);
    card
}
