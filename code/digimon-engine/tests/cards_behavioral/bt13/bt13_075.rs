//! BT13-075 Alphamon — Digimon, Lv.6, Black, DP 12000, Cost 12.
//! Traits: Holy Warrior / Royal Knight / X Antibody. Form: Mega. Vaccine.
//!
//! # Card text (cards.json / card image)
//!
//! ```text
//! [On Play] [When Digivolving] By placing 1 Digimon card with the [X Antibody]
//!   trait or the [Royal Knight] trait from your trash under this Digimon as
//!   its bottom digivolution card, all of your opponent's Digimon with a play
//!   cost of 10 or higher can't attack players until the end of your
//!   opponent's turn.
//! [All Turns] [Once Per Turn] When this Digimon would leave the battle area by
//!   an effect, by returning 1 Digimon card with the [X Antibody] trait or the
//!   [Royal Knight] trait from this Digimon's digivolution cards to the bottom
//!   of your deck, prevent it from leaving play.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Black/BT13_075.cs
//!
//! # Implementation
//! - Clause 1 ([On Play][When Digivolving]): a `select_trash` placement cost
//!   (`optional: true, cost: true`) places an [X Antibody]/[Royal Knight]
//!   Digimon as a bottom digivolution source, then installs a CONTINUOUS
//!   floating mass modifier (`add_modifier{continuous: true}`) granting
//!   `CannotAttackPlayer` to every opponent Digimon with `play_cost_gte: 10`,
//!   expiring `end_of_opponents_turn`. Continuous so it also binds opponent
//!   Digimon that enter after resolution (DCGO `GainCanNotAttackPlayerEffect`
//!   with a per-attack `AttackerCondition`).
//! - Clause 2 ([All Turns][OPT] would-leave): a `kind: replacement`
//!   leave-prevention. Its cost returns 1 [X Antibody]/[Royal Knight]
//!   digivolution source to the BOTTOM of the owner's deck via the
//!   `return_selected_sources_to_deck` verb (G-RETURN-SELECTED-SOURCE-TO-DECK-
//!   BOTTOM), then `cancel_replacement`. Gated to removal BY AN EFFECT
//!   (`none_of: [replacement_cause: battle]`) and to a payable state
//!   (`self_digivolution_sources_trait_has` X Antibody / Royal Knight).

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A Digimon with explicit traits and play cost. `play_cost` drives the
/// `play_cost_gte: 10` aura filter; `traits` drives the [X Antibody]/[Royal
/// Knight] cost/source filters.
fn make_digimon(id: &str, dp: i32, play_cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card.play_cost = play_cost;
    card.level = Some(6);
    card.colors = vec![CardColor::Black];
    card.traits = traits.iter().map(|t| (*t).to_string()).collect();
    card
}

/// Drop a card straight into `player`'s trash.
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let src = CardSource::new(data_idx, player, runner.game.next_card_index());
    runner.game.players[player as usize].trash.push(src);
}

fn fire_on_play(runner: &mut DebugRunner, source: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn permanent_exists(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == card_id)
}

fn alpha_runner() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT13-075")
        .expect("BT13-075 YAML loads")
}

// ─── Section 1: metadata / structure ─────────────────────────────────────────

#[test]
fn bt13_075_loads_with_both_clauses() {
    use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};

    let runner = alpha_runner().start();
    let compiled = runner.compiled_card("BT13-075").expect("compiled card");
    assert_eq!(compiled.name, "Alphamon");
    assert_eq!(compiled.level, Some(6));
    assert_eq!(compiled.dp, Some(12000));
    assert_eq!(compiled.cost, Some(12));
    assert!(compiled.traits.iter().any(|t| t == "Royal Knight"));
    assert!(compiled.traits.iter().any(|t| t == "X Antibody"));

    let has_replacement = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
        )
    });
    assert!(
        has_replacement,
        "the would-leave self-protection compiles as a replacement clause"
    );
    let has_triggered = compiled
        .effects
        .iter()
        .any(|c| matches!(c, CompiledClause::Triggered(_)));
    assert!(
        has_triggered,
        "the [On Play]/[When Digivolving] aura compiles as a triggered clause"
    );
}

// ─── Section 2: Clause 1 — On Play play-cost-10+ can't-attack-players aura ────

/// After placing a source from trash, opponent Digimon with play cost 10+
/// cannot attack the player (including ones that ENTER after resolution), while
/// a sub-10-cost opponent Digimon still can; the restriction lifts at the end
/// of the opponent's turn.
#[test]
fn bt13_075_places_source_then_blocks_high_play_cost_attacks() {
    let mut runner = alpha_runner()
        .add_card(make_digimon("RK-SOURCE", 3000, 4, &["Royal Knight"]))
        .add_card(make_digimon("OPP-BIG", 12000, 12, &[]))
        .add_card(make_digimon("OPP-SMALL", 4000, 5, &[]))
        .add_card(make_digimon("OPP-LATE", 13000, 11, &[]))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .security(0, &["FILL"; 3])
        .start();
    push_to_trash(&mut runner, 0, "RK-SOURCE");

    let alpha = runner.place_on_field(0, "BT13-075", Some(0));
    let opp_big = runner.place_on_field(1, "OPP-BIG", Some(0));
    let opp_small = runner.place_on_field(1, "OPP-SMALL", Some(0));

    // Pay the placement cost (the only legal trash pick).
    fire_on_play(&mut runner, alpha);
    let view = runner
        .pending_selection_view()
        .expect("placement-cost trash prompt installs");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Royal Knight Digimon is a legal placement cost"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("place Royal Knight source from trash");
    runner.auto_resolve().expect("finish aura install");

    assert!(
        runner.game.players[0].battle_area[alpha.index as usize]
            .card_sources
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == "RK-SOURCE"),
        "the placed card became a digivolution source of Alphamon"
    );

    let _ = opp_small; // the sub-10-cost "may attack" case lives in its own test

    // Rotate to the opponent's (P1's) turn so they can declare attacks.
    runner.game.set_memory(3);
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1, "rotated to the opponent's turn");

    // (c) LATE ENTRANT: a NEW 11-cost opponent Digimon played after the aura
    // resolved is ALSO bound (continuous re-evaluation). In real play, entering
    // the field runs a declarative recompute; `place_on_field` is a raw test
    // helper, so tick the floating-mass re-scan explicitly to model that.
    let opp_late = runner.place_on_field(1, "OPP-LATE", Some(0));
    runner.game.tick_declarative_effects();

    // (b) the 12-cost opponent Digimon already on field cannot attack the player,
    // and (c) the late-entrant 11-cost Digimon is also blocked. `attack_player`
    // returns `Invalid` for a `CannotAttackPlayer` gate BEFORE mutating any
    // state (combat/mod.rs), so these probes neither resolve nor cycle the turn.
    assert_eq!(
        runner.attack_player(opp_big, 0, false),
        AttackResult::Invalid,
        "(b) play-cost-12 opponent Digimon already on field cannot attack the player"
    );
    assert_eq!(
        runner.attack_player(opp_late, 0, false),
        AttackResult::Invalid,
        "(c) an opponent Digimon entering AFTER resolution is also blocked (continuous aura)"
    );

    // (e) after the opponent's turn ends, the restriction is gone — a 12-cost
    // opponent Digimon may attack the player again.
    runner.game.set_memory(3);
    runner.end_turn(); // P1 (opponent) ends -> back to P0
    runner.game.set_memory(3);
    runner.end_turn(); // P0 ends -> P1's turn again
    assert_eq!(runner.game.turn_player(), 1);
    assert_ne!(
        runner.attack_player(opp_big, 0, false),
        AttackResult::Invalid,
        "(e) the restriction expired at the end of the opponent's turn"
    );
}

/// Companion positive arm: a sub-10-cost opponent Digimon is NOT bound by the
/// aura and may attack the player (kept in its own test so resolving the attack
/// cannot perturb the blocking assertions above).
#[test]
fn bt13_075_aura_does_not_block_sub_ten_cost_attacker() {
    let mut runner = alpha_runner()
        .add_card(make_digimon("RK-SOURCE", 3000, 4, &["Royal Knight"]))
        .add_card(make_digimon("OPP-SMALL", 4000, 5, &[]))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 20])
        .deck(1, &["FILL"; 20])
        .security(0, &["FILL"; 3])
        .start();
    push_to_trash(&mut runner, 0, "RK-SOURCE");

    let alpha = runner.place_on_field(0, "BT13-075", Some(0));
    let opp_small = runner.place_on_field(1, "OPP-SMALL", Some(0));

    fire_on_play(&mut runner, alpha);
    let view = runner
        .pending_selection_view()
        .expect("placement-cost trash prompt installs");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("place Royal Knight source from trash");
    runner.auto_resolve().expect("finish aura install");

    runner.game.set_memory(3);
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1, "rotated to the opponent's turn");

    assert_ne!(
        runner.attack_player(opp_small, 0, false),
        AttackResult::Invalid,
        "play-cost-5 opponent Digimon is unaffected and may attack the player"
    );
}

/// Negative: with no [X Antibody]/[Royal Knight] Digimon in trash the clause
/// cannot pay its placement cost — no prompt installs and no aura is granted
/// (a sub-condition `count_gte` gate).
#[test]
fn bt13_075_on_play_without_eligible_trash_source_does_nothing() {
    let mut runner = alpha_runner()
        .add_card(make_digimon("PLAIN", 3000, 4, &["Beast"]))
        .add_card(make_digimon("OPP-BIG", 12000, 12, &[]))
        .deck(0, &["PLAIN"; 5])
        .deck(1, &["PLAIN"; 5])
        .start();
    push_to_trash(&mut runner, 0, "PLAIN"); // not X Antibody / Royal Knight

    let alpha = runner.place_on_field(0, "BT13-075", Some(0));
    let opp_big = runner.place_on_field(1, "OPP-BIG", Some(0));

    fire_on_play(&mut runner, alpha);
    assert!(
        runner.pending_selection().is_none(),
        "no eligible trash source -> the clause does not fire"
    );

    runner.game.set_memory(3);
    runner.end_turn();
    assert_eq!(runner.game.turn_player(), 1);
    assert_ne!(
        runner.attack_player(opp_big, 0, false),
        AttackResult::Invalid,
        "with no aura installed the 12-cost opponent Digimon may attack the player"
    );
}

// ─── Section 3: Clause 2 — would-leave self-protection (deck-bottom return) ───

/// Paying the cost (return 1 [X Antibody]/[Royal Knight] digivolution source to
/// the BOTTOM of the deck) prevents the effect removal: Alphamon stays, the
/// chosen source is gone from its stack and sits at the bottom of the deck.
#[test]
fn bt13_075_would_leave_returns_source_to_deck_bottom_and_prevents() {
    let mut runner = alpha_runner()
        .add_card(make_digimon("RK-SRC", 3000, 4, &["Royal Knight"]))
        .start();
    // [RK-SRC, BT13-075] bottom→top: RK-SRC is a digivolution source.
    let alpha = runner.place_stack(0, &["RK-SRC", "BT13-075"]);

    let deck_before = runner.game.players[0].deck.len();

    runner
        .game
        .delete_permanent_with_cause(alpha, ReplacementCause::OpponentEffect);

    // Accept the optional replacement.
    let view = runner
        .pending_selection_view()
        .expect("would-leave replacement offers an accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(runner.pending_is_optional(), "replacement may be declined");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("accept the leave-prevention replacement");

    // Pay the source-return cost — every player choice surfaces (no auto-select).
    let cost_view = runner
        .pending_selection_view()
        .expect("source-return cost selection installs");
    assert!(
        matches!(cost_view.kind, SelectionKind::SourceMulti { .. }),
        "the cost is a digivolution-source pick, got {:?}",
        cost_view.kind
    );
    assert_eq!(
        cost_view.valid_action_ids.len(),
        1,
        "only the Royal Knight source is a legal cost"
    );
    runner
        .execute_action(0, cost_view.valid_action_ids[0])
        .expect("return the Royal Knight source");
    runner.auto_resolve().expect("finish replacement");

    // Alphamon stays on the field.
    assert!(
        permanent_exists(&runner, 0, "BT13-075"),
        "paying the cost prevents the removal — Alphamon stays"
    );
    // The source left the digivolution stack.
    let alpha_perm = &runner.game.players[0].battle_area[alpha.index as usize];
    assert!(
        alpha_perm
            .card_sources
            .iter()
            .all(|s| s.card_id(&runner.game.card_data) != "RK-SRC"),
        "the returned source is gone from Alphamon's digivolution cards"
    );
    // The source is at the BOTTOM of the owner's deck (index 0), not in trash.
    assert_eq!(
        runner.game.players[0].deck.len(),
        deck_before + 1,
        "deck grows by the returned source"
    );
    assert_eq!(
        runner.game.players[0].deck[0].card_id(&runner.game.card_data),
        "RK-SRC",
        "the source returns to the BOTTOM of the deck (index 0)"
    );
    assert_eq!(
        runner.trash_size(0),
        0,
        "the return-to-deck cost must NOT route the source to trash"
    );
}

/// Declining the optional replacement lets Alphamon leave the battle area, with
/// the [X Antibody]/[Royal Knight] source untouched.
#[test]
fn bt13_075_would_leave_decline_lets_it_leave() {
    let mut runner = alpha_runner()
        .add_card(make_digimon("RK-SRC", 3000, 4, &["Royal Knight"]))
        .start();
    let alpha = runner.place_stack(0, &["RK-SRC", "BT13-075"]);

    runner
        .game
        .delete_permanent_with_cause(alpha, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("would-leave replacement offers an accept prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    runner
        .execute_action(0, PASS)
        .expect("decline the leave-prevention");
    runner.auto_resolve().expect("finish declined removal");

    assert!(
        !permanent_exists(&runner, 0, "BT13-075"),
        "declining the replacement lets Alphamon leave the battle area"
    );
}

/// Negative gate: with NO [X Antibody]/[Royal Knight] source beneath Alphamon
/// the prevention cost is unpayable, so the replacement is never offered and the
/// removal goes through.
#[test]
fn bt13_075_would_leave_no_eligible_source_does_not_prevent() {
    let mut runner = alpha_runner()
        .add_card(make_digimon("PLAIN", 3000, 4, &["Beast"]))
        .start();
    let alpha = runner.place_stack(0, &["PLAIN", "BT13-075"]);

    runner
        .game
        .delete_permanent_with_cause(alpha, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_selection().is_none(),
        "no [X Antibody]/[Royal Knight] source -> the prevention is never offered"
    );
    assert!(
        !permanent_exists(&runner, 0, "BT13-075"),
        "with no payable cost Alphamon leaves the battle area"
    );
}

/// Cause gate: removal in BATTLE is not "by an effect", so the would-leave
/// self-protection does not trigger.
#[test]
fn bt13_075_would_leave_does_not_trigger_on_battle_removal() {
    let mut runner = alpha_runner()
        .add_card(make_digimon("RK-SRC", 3000, 4, &["Royal Knight"]))
        .start();
    let alpha = runner.place_stack(0, &["RK-SRC", "BT13-075"]);

    runner
        .game
        .delete_permanent_with_cause(alpha, ReplacementCause::Battle);

    assert!(
        runner.pending_selection().is_none(),
        "battle removal is not 'by an effect' -> no leave-prevention prompt"
    );
    assert!(
        !permanent_exists(&runner, 0, "BT13-075"),
        "Alphamon is removed by the battle deletion"
    );
}
