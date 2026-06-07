//! BT13-088 Belphemon: Sleep Mode — Digimon, Lv.6, Purple, DP 11000, Cost 11.
//! Traits: Demon Lord / Seven Great Demon Lords. Attribute: Virus.
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] [When Digivolving] By placing 1 [Belphemon: Rage Mode] from your
//! trash on top of this Digimon's digivolution cards, this Digimon can't attack
//! and isn't affected by your opponent's effects until the end of your
//! opponent's turn.
//! [Opponent's Turn] [Once Per Turn] When an opponent's Digimon attacks, you
//! may trash 2 cards in your hand to end the attack.
//!
//! Alt digivolve: [Digivolve] [Belphemon: Rage Mode]: Cost 1.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Purple/BT13_088.cs
//!   - EffectTiming.None: AddSelfDigivolutionRequirementStaticEffect, topCard
//!     name contains "Belphemon: Rage Mode", digivolutionCost 1.
//!   - EffectTiming.OnEnterFieldAnyone × 2 (On Play / When Digivolving, shared
//!     body): SelectCard from Trash filtered to "Belphemon: Rage Mode",
//!     canNoSelect false, AddDigivolutionCardsTop, then GainCanNotAttack
//!     (UntilOpponentTurnEnd) + CanNotAffectedClass (opponent skills,
//!     UntilOpponentTurnEnd).
//!   - EffectTiming.OnAllyAttack ([Opponent's Turn][OPT]): PermanentCondition
//!     IsOpponentPermanent, CanActivateCondition HandCards.Count >= 2,
//!     SelectHandEffect Mode.Discard maxCount 2 canEndNotMax false, then
//!     attackProcess.EndAttack().
//!
//! # Patterns this test covers
//! - E1/A6 [On Play]/[When Digivolving] dual-timing clause that pays a
//!   trash-card-as-digivolution-source cost (Belphemon: Rage Mode) and gains
//!   CannotAttack + the 4-source-kind opponent-effect immunity bundle, both
//!   until end of opponent's turn.
//! - F-adjacent: face-up [Opponent's Turn][OPT] attack-cancel that pays a
//!   "trash 2 cards in hand" cost to end the attack.
//! - Alt-path metadata: the printed [Belphemon: Rage Mode] cost-1 alt-digivolve.
//!
//! # place_as_top_source resolution (PARTIAL note)
//!
//! DCGO places Rage Mode via `AddDigivolutionCardsTop` (topmost source, just
//! below the face card). The engine/DSL has only `place_as_bottom_source`. The
//! placement POSITION is behaviorally inert for this card: Belphemon's own
//! effect only requires Rage Mode to be IN the digivolution stack so its
//! inherited effect (BT13-091/EX10-022 "[End of Opp Turn] trash top card if
//! [Belphemon: Sleep Mode]") becomes active — inherited effects fire from any
//! source position. No mechanic on this card references the top/bottom source
//! position. The YAML uses `place_as_bottom_source` with this documented
//! cosmetic divergence; the tests assert Rage Mode is present as a source, not
//! a specific stack index.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectSourceKind, EffectTiming, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const RAGE_MODE: &str = "Belphemon: Rage Mode";

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A stand-in [Belphemon: Rage Mode] card — only the NAME matters for the
/// `select_trash` filter and the digivolution-source placement.
fn make_rage_mode(id: &str) -> CardData {
    let mut card = make_test_card(id, RAGE_MODE);
    card.level = Some(6);
    card.dp = Some(11000);
    card
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
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

fn fire_on_play(runner: &mut DebugRunner, source: digimon_engine::permanent::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
}

fn fire_when_digivolving(
    runner: &mut DebugRunner,
    source: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn rage_is_source(runner: &DebugRunner, handle: digimon_engine::permanent::PermanentHandle) -> bool {
    runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .card_sources
        .iter()
        .any(|src| src.card_id(&runner.game.card_data) == "RAGE")
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn bt13_088_metadata_matches_printed_card() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .build();
    let card = runner.compiled_card("BT13-088").expect("compiled card");

    assert_eq!(card.card, "BT13-088");
    assert_eq!(card.name, "Belphemon: Sleep Mode");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(11));
    assert_eq!(card.dp, Some(11000));
    assert!(card.traits.contains(&"Demon Lord".to_string()));
    assert!(card
        .traits
        .contains(&"Seven Great Demon Lords".to_string()));
}

#[test]
fn bt13_088_has_dual_timing_grant_and_opponent_attack_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .build();
    let compiled = runner.compiled_card("BT13-088").expect("compiled card");

    let triggered: Vec<&_> = compiled
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
        "BT13-088 has exactly two triggered clauses: dual-timing grant + opp-attack cancel"
    );

    let grant = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("an [On Play] clause exists");
    assert_eq!(grant.scope, CompiledScope::FaceUp);
    assert!(
        grant.when.contains(&CompiledTiming::WhenDigivolving),
        "the grant clause is shared [On Play] / [When Digivolving]"
    );
}

#[test]
fn bt13_088_opponent_attack_clause_is_opt_face_up() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .build();
    let compiled = runner.compiled_card("BT13-088").expect("compiled card");

    let cancel = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::OnOpponentAttack] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("an [Opponent's Turn] attack-cancel clause exists");

    assert_eq!(
        cancel.scope,
        CompiledScope::FaceUp,
        "the attack-cancel is BT13-088's OWN (face-up) effect, not inherited"
    );
    assert!(cancel.once_per_turn, "attack-cancel is [Once Per Turn]");
    assert!(
        cancel.optional,
        "attack-cancel is optional ('you may trash 2 cards')"
    );
}

#[test]
fn bt13_088_has_rage_mode_cost_one_alt_digivolve() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .build();
    let compiled = runner.compiled_card("BT13-088").expect("compiled card");

    let rage_path = compiled
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.from
                    .as_ref()
                    .and_then(|f| f.name_contains.as_deref())
                    .map(|n| n.contains("Belphemon: Rage Mode"))
                    .unwrap_or(false)
        })
        .expect("a [Belphemon: Rage Mode] alt-digivolve path exists");

    assert!(
        matches!(rage_path.cost, Some(CompiledCost::Literal(1))),
        "[Belphemon: Rage Mode] alt-digivolve costs 1 memory"
    );
}

// ─── Section 2: clause-0 condition gating (Rage Mode in trash) ───────────────

/// Positive: with [Belphemon: Rage Mode] in trash, the On Play grant clause
/// installs the trash-selection cost prompt.
#[test]
fn bt13_088_on_play_with_rage_in_trash_installs_trash_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .add_card(make_rage_mode("RAGE"))
        .start();
    push_to_trash(&mut runner, 0, "RAGE");

    let belphemon = runner.place_on_field(0, "BT13-088", Some(0));
    fire_on_play(&mut runner, belphemon);

    let view = runner
        .pending_selection_view()
        .expect("Rage Mode trash-selection cost prompt installs");
    assert_eq!(view.kind, SelectionKind::Trash);
}

/// Negative: with NO [Belphemon: Rage Mode] in trash, the grant clause cannot
/// pay its placement cost — no prompt installs and no modifier is gained.
#[test]
fn bt13_088_on_play_without_rage_in_trash_does_nothing() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .add_card(make_filler("OTHER"))
        .start();
    push_to_trash(&mut runner, 0, "OTHER");

    let belphemon = runner.place_on_field(0, "BT13-088", Some(0));
    fire_on_play(&mut runner, belphemon);

    assert!(
        runner.pending_selection().is_none(),
        "no Rage Mode in trash -> the grant clause does not fire"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(belphemon, ModifierType::CannotAttack),
        "no CannotAttack granted when the cost is unpayable"
    );
}

// ─── Section 3: clause-0 behavioral — grant + immunity ───────────────────────

/// Paying the cost places Rage Mode under Belphemon as a digivolution source,
/// then grants CannotAttack + opponent-effect immunity until end of opp turn.
#[test]
fn bt13_088_on_play_places_rage_and_grants_cannot_attack_plus_immunity() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .add_card(make_rage_mode("RAGE"))
        .start();
    push_to_trash(&mut runner, 0, "RAGE");

    let belphemon = runner.place_on_field(0, "BT13-088", Some(0));
    fire_on_play(&mut runner, belphemon);

    let view = runner
        .pending_selection_view()
        .expect("Rage Mode trash prompt");
    assert_eq!(view.kind, SelectionKind::Trash);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Belphemon: Rage Mode card is a legal placement cost"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("place Rage Mode from trash");
    runner.auto_resolve().expect("finish grant clause");

    assert!(
        rage_is_source(&runner, belphemon),
        "Belphemon: Rage Mode is now a digivolution source of Belphemon"
    );
    assert_eq!(
        runner.trash_size(0),
        0,
        "the Rage Mode card left the trash"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(belphemon, ModifierType::CannotAttack),
        "Belphemon gains CannotAttack"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(belphemon, ModifierType::CannotBeAffected),
        "Belphemon gains the opponent-effect immunity bundle"
    );
    for source_kind in [
        EffectSourceKind::Digimon,
        EffectSourceKind::Tamer,
        EffectSourceKind::Option,
        EffectSourceKind::Rule,
    ] {
        assert!(
            runner
                .game
                .permanent_is_unaffected_by_effect(belphemon, 1, source_kind),
            "opponent {source_kind:?} effects must not affect Belphemon"
        );
        assert!(
            !runner
                .game
                .permanent_is_unaffected_by_effect(belphemon, 0, source_kind),
            "own {source_kind:?} effects must still affect Belphemon"
        );
    }
}

/// The grant also fires on [When Digivolving] (shared body).
#[test]
fn bt13_088_when_digivolving_also_grants() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .add_card(make_rage_mode("RAGE"))
        .start();
    push_to_trash(&mut runner, 0, "RAGE");

    let belphemon = runner.place_on_field(0, "BT13-088", Some(0));
    fire_when_digivolving(&mut runner, belphemon);

    let view = runner
        .pending_selection_view()
        .expect("When Digivolving installs the same trash prompt");
    assert_eq!(view.kind, SelectionKind::Trash);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("place Rage Mode");
    runner.auto_resolve().expect("finish grant clause");

    assert!(
        runner
            .game
            .modifiers
            .has(belphemon, ModifierType::CannotAttack),
        "When Digivolving grants CannotAttack too"
    );
}

/// The CannotAttack + immunity grants expire at end of opponent's turn.
#[test]
fn bt13_088_grants_expire_at_end_of_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .add_card(make_rage_mode("RAGE"))
        .add_card(make_filler("DECK0"))
        .add_card(make_filler("DECK1"))
        // Both players need a deck so turn rotation does not deck-out (which
        // would end the game before the opponent's turn can end).
        .deck(0, &["DECK0"; 20])
        .deck(1, &["DECK1"; 20])
        .start();
    push_to_trash(&mut runner, 0, "RAGE");

    let belphemon = runner.place_on_field(0, "BT13-088", Some(0));
    fire_on_play(&mut runner, belphemon);
    let view = runner.pending_selection_view().expect("trash prompt");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("place Rage Mode");
    runner.auto_resolve().expect("finish grant clause");

    assert!(runner
        .game
        .modifiers
        .has(belphemon, ModifierType::CannotAttack));

    // Cycle a full turn rotation back to the controller. Set a positive
    // memory before each turn-end so the memory-swing-back guard in `end_turn`
    // (memory restored from negative -> parks in Main without rotating) does
    // not stall the rotation.
    runner.game.set_memory(3);
    runner.end_turn(); // P0 ends -> P1's (opponent's) turn begins
    assert_eq!(runner.game.turn_player(), 1, "rotated to the opponent's turn");
    assert!(
        runner
            .game
            .modifiers
            .has(belphemon, ModifierType::CannotAttack),
        "CannotAttack still active during the opponent's turn"
    );

    runner.game.enter_main_phase();
    runner.game.set_memory(3);
    runner.end_turn(); // P1's (opponent's) turn ends -> back to P0
    assert_eq!(runner.game.turn_player(), 0, "rotated back to the controller");

    assert!(
        !runner
            .game
            .modifiers
            .has(belphemon, ModifierType::CannotAttack),
        "CannotAttack expires at the end of the opponent's turn"
    );
    assert!(
        !runner
            .game
            .modifiers
            .has(belphemon, ModifierType::CannotBeAffected),
        "the opponent-effect immunity expires at the end of the opponent's turn"
    );
}

// ─── Section 4: clause-1 condition gating (hand >= 2) ────────────────────────

/// Negative: with fewer than 2 cards in hand, the [Opponent's Turn] cancel
/// cannot pay its 2-card trash cost — no prompt and the attack resolves.
#[test]
fn bt13_088_opp_attack_with_one_hand_card_does_not_install_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .add_card(make_filler("HAND1"))
        .add_card(make_filler("ATTACKER"))
        .add_card(make_filler("SECURITY"))
        .hand(0, &["HAND1"])
        .security(0, &["SECURITY"])
        .start();
    runner.place_on_field(0, "BT13-088", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);

    assert!(
        runner.pending_selection().is_none(),
        "fewer than 2 hand cards -> no attack-cancel prompt installs"
    );
    runner.auto_resolve().expect("resolve attack");
    assert_eq!(
        runner.security_count(0),
        0,
        "the attack was NOT cancelled for free — the security check happened"
    );
}

// ─── Section 5: clause-1 behavioral — trash 2 -> end attack ──────────────────

/// Positive: on the opponent's turn, when their Digimon attacks, trashing 2
/// hand cards ends that attack before any security check.
#[test]
fn bt13_088_opp_attack_trash_two_ends_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .add_card(make_filler("HAND1"))
        .add_card(make_filler("HAND2"))
        .add_card(make_filler("ATTACKER"))
        .add_card(make_filler("SECURITY"))
        .hand(0, &["HAND1", "HAND2"])
        .security(0, &["SECURITY"])
        .start();
    runner.place_on_field(0, "BT13-088", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);

    let view = runner
        .pending_selection_view()
        .expect("first hand-trash cost prompt installs");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "the attack cancel is declinable"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("trash first hand card");

    let view2 = runner
        .pending_selection_view()
        .expect("second hand-trash cost prompt installs");
    assert_eq!(view2.kind, SelectionKind::Hand);
    runner
        .execute_action(0, view2.valid_action_ids[0])
        .expect("trash second hand card");
    runner.auto_resolve().expect("finish attack cancel");

    assert_eq!(
        runner.security_count(0),
        1,
        "the attack ended before any security check"
    );
    assert_eq!(
        runner.hand_size(0),
        0,
        "exactly 2 hand cards were trashed as the cost"
    );
    assert_eq!(
        runner.trash_size(0),
        2,
        "the 2 trashed cards landed in trash"
    );
    assert!(
        runner.game.pending_attack.is_none(),
        "the attack state is fully cleared"
    );
}

/// Negative (decline): declining the cost trashes nothing and lets the attack
/// resolve (the security check proceeds).
#[test]
fn bt13_088_opp_attack_decline_lets_attack_resolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .add_card(make_filler("HAND1"))
        .add_card(make_filler("HAND2"))
        .add_card(make_filler("ATTACKER"))
        .add_card(make_filler("SECURITY"))
        .hand(0, &["HAND1", "HAND2"])
        .security(0, &["SECURITY"])
        .start();
    runner.place_on_field(0, "BT13-088", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);
    runner.execute_action(0, PASS).expect("decline the cancel");
    runner.auto_resolve().expect("resolve attack");

    assert_eq!(
        runner.hand_size(0),
        2,
        "declining trashes no hand cards"
    );
    assert_eq!(
        runner.security_count(0),
        0,
        "declining does not end the attack — the security check happened"
    );
}

// ─── Section 6: clause-1 OPT + opponent-turn gating ──────────────────────────

/// OPT lockout: a second opponent attack the same turn does not re-offer the
/// cancel prompt; after the turn cycles the lockout clears.
#[test]
fn bt13_088_opp_attack_opt_locks_second_attack_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .add_card(make_filler("HAND1"))
        .add_card(make_filler("HAND2"))
        .add_card(make_filler("HAND3"))
        .add_card(make_filler("HAND4"))
        .add_card(make_filler("ATTACKER1"))
        .add_card(make_filler("ATTACKER2"))
        .add_card(make_filler("SEC1"))
        .add_card(make_filler("SEC2"))
        .hand(0, &["HAND1", "HAND2", "HAND3", "HAND4"])
        .security(0, &["SEC1", "SEC2"])
        .start();
    runner.place_on_field(0, "BT13-088", Some(0));
    let attacker1 = runner.place_on_field(1, "ATTACKER1", Some(0));
    let attacker2 = runner.place_on_field(1, "ATTACKER2", Some(0));
    runner.end_turn();

    // First opponent attack — pay the cost, end the attack.
    runner.attack_player(attacker1, 0, false);
    let view = runner
        .pending_selection_view()
        .expect("first attack offers the cancel prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("trash first card");
    let view2 = runner
        .pending_selection_view()
        .expect("second pick prompt");
    runner
        .execute_action(0, view2.valid_action_ids[0])
        .expect("trash second card");
    runner.auto_resolve().expect("finish first cancel");
    assert_eq!(
        runner.security_count(0),
        2,
        "first attack was cancelled — no security checked"
    );

    // Second opponent attack, same turn — OPT must lock the clause out.
    runner.attack_player(attacker2, 0, false);
    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] blocks a second activation in the same turn"
    );
    runner.auto_resolve().expect("resolve second attack");
    assert_eq!(
        runner.security_count(0),
        1,
        "the second attack was not cancelled — it checked one security"
    );
}

/// Negative (own turn): the cancel is gated to the opponent's turn. When the
/// BT13-088 controller is the attacker on their OWN turn, it must not fire.
#[test]
fn bt13_088_does_not_fire_on_controllers_own_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-088")
        .expect("BT13-088 YAML loads")
        .add_card(make_filler("HAND1"))
        .add_card(make_filler("HAND2"))
        .add_card(make_filler("DEFENDER-SEC"))
        .hand(0, &["HAND1", "HAND2"])
        .security(1, &["DEFENDER-SEC"])
        .start();
    // BT13-088's controller (player 0) is the active player and the attacker.
    let belphemon = runner.place_on_field(0, "BT13-088", Some(0));

    runner.attack_player(belphemon, 1, false);

    assert!(
        runner.pending_selection().is_none(),
        "on the controller's own turn the attack-cancel clause must not fire"
    );
}
