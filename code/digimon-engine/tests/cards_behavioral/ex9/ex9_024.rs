//! EX9-024 Hanimon — Digimon, Lv.3, Yellow+Black, DP 1000, Cost 3.
//! Traits: Puppet, LIBERATOR. Attribute: Data.
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] By trashing 1 card in your hand, you may return 1 Digimon card
//! with the [Puppet] trait from your trash to the hand.
//!
//! Inherited Effect [Opponent's Turn] [Once Per Turn]
//! When one of your opponent's Digimon attacks, by deleting 1 of your other
//! Digimon, end that attack.
//!
//! Security: (none)
//!
//! Alt digivolve: [Digivolve] [Kyaromon]: Cost 0 — digivolve from a
//! [Kyaromon] for 0 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX9/Yellow/EX9_024.cs
//!
//! # Patterns this test covers
//! - A4 trash -> hand recursion gated by a hand-discard cost (On Play).
//! - E2 OPT + optional decline (the inherited attack-cancel clause).
//! - F-adjacent: opponent-attack observer (`on_opponent_attack`) that pays a
//!   "delete 1 of your OTHER Digimon" cost to end the attack.
//! - Alt-path metadata: standard Lv.2 Yellow digivolve + the printed
//!   [Kyaromon] cost-0 alt-digivolve path.
//!
//! # Faithfulness note (audit 2026-05-21)
//!
//! The inherited clause as previously shipped used `select_effect_choice`
//! followed by `delete_permanent: { target: this }` — i.e. it deleted the
//! EX9-024 CARRIER itself. The printed text says "by deleting 1 of your
//! **other** Digimon" and DCGO's `CanSelectPermanentCondition` requires
//! `permanent != card.PermanentOfThisCard()`. The corrected YAML uses
//! `select_own_permanent` with an `other: true` + `kind: digimon` filter,
//! matching the proven sibling EX11-020 Hanimon. `install_select_own_permanent`
//! pre-filters candidates: a lone carrier (no OTHER Digimon) short-circuits
//! with no prompt — so the attack does NOT end for free — and it attaches no
//! decline tail, so declining the optional pick also leaves the attack intact.

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::selection::SelectionKind;

// ─── Fixtures ────────────────────────────────────────────────────────────────

/// A Digimon card carrying the [Puppet] trait — eligible for the On Play
/// trash recursion.
fn make_puppet(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.traits = vec!["Puppet".to_string()];
    card
}

/// A plain test Digimon with no traits — `make_test_card` already defaults to
/// `CardKind::Digimon`, so this is also a valid `other Digimon` deletion cost.
fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Drop a card straight into player 0's trash for the recursion fixtures.
fn push_to_trash(runner: &mut DebugRunner, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("card exists");
    let src = CardSource::new(data_idx, 0, runner.game.next_card_index());
    runner.game.players[0].trash.push(src);
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn ex9_024_has_on_play_and_inherited_attack_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .build();
    let compiled = runner.compiled_card("EX9-024").expect("compiled card");

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
        "EX9-024 has exactly two triggered clauses: [On Play] + inherited attack-cancel"
    );

    let on_play = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("an [On Play] clause exists");
    assert_eq!(on_play.scope, CompiledScope::FaceUp);
    assert!(
        on_play.optional,
        "[On Play] return is optional ('you may return')"
    );
    assert!(!on_play.once_per_turn, "[On Play] is not Once Per Turn");
}

#[test]
fn ex9_024_inherited_clause_is_opponent_attack_opt() {
    let runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .build();
    let compiled = runner.compiled_card("EX9-024").expect("compiled card");

    let inherited = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
            _ => None,
        })
        .expect("an inherited clause exists");

    assert_eq!(
        inherited.when,
        vec![CompiledTiming::OnOpponentAttack],
        "inherited clause fires when an opponent's Digimon attacks"
    );
    assert!(
        inherited.once_per_turn,
        "inherited clause is [Once Per Turn]"
    );
}

#[test]
fn ex9_024_has_kyaromon_cost_zero_alt_digivolve() {
    let runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .build();
    let compiled = runner.compiled_card("EX9-024").expect("compiled card");

    let kyaromon_path = compiled
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve
                && p.from.as_ref().and_then(|f| f.name_contains.as_deref()) == Some("Kyaromon")
        })
        .expect("a [Kyaromon] alt-digivolve path exists");

    assert!(
        matches!(
            kyaromon_path.cost,
            Some(digimon_dsl::compiled::CompiledCost::Literal(0))
        ),
        "[Kyaromon] alt-digivolve costs 0 memory"
    );
}

// ─── Section 2: [On Play] condition gating ───────────────────────────────────

/// Positive: with cards in hand, playing Hanimon installs the discard-cost
/// prompt for the [On Play] clause.
#[test]
fn ex9_024_on_play_with_hand_card_installs_discard_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_filler("DISCARD"))
        .hand(0, &["EX9-024", "DISCARD"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Hanimon");

    let view = runner
        .pending_selection_view()
        .expect("discard-cost prompt installs when a hand card exists");
    assert_eq!(view.kind, SelectionKind::Hand);
}

/// Negative: with an empty hand (after Hanimon itself is played there is no
/// other card to trash), the [On Play] clause cannot pay its cost, so no
/// recursion prompt is offered.
#[test]
fn ex9_024_on_play_with_empty_hand_does_not_install_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_puppet("PUPPET-TRASH"))
        .hand(0, &["EX9-024"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, "PUPPET-TRASH");

    runner.play(0, 0).expect("play Hanimon");

    assert!(
        runner.pending_selection().is_none(),
        "no discardable hand card -> the [On Play] clause does not fire"
    );
    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert_eq!(
        trash_ids,
        vec!["PUPPET-TRASH".to_string()],
        "the Puppet card stays in trash with no recursion"
    );
}

// ─── Section 3: [On Play] behavioral — discard cost / Puppet recursion ───────

/// Positive: discard a hand card, then return a [Puppet]-trait Digimon from
/// trash to hand.
#[test]
fn ex9_024_on_play_trashes_hand_card_and_returns_puppet_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_puppet("PUPPET-TRASH"))
        .add_card(make_filler("DISCARD"))
        .hand(0, &["EX9-024", "DISCARD"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, "PUPPET-TRASH");

    runner.play(0, 0).expect("play Hanimon");

    let discard_view = runner.pending_selection_view().expect("discard prompt");
    assert_eq!(discard_view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "the discard cost payment can be declined"
    );
    assert_eq!(
        discard_view.valid_action_ids,
        vec![PLAY_HAND_START],
        "only the remaining hand card is discardable"
    );
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("trash hand card");

    let trash_view = runner
        .pending_selection_view()
        .expect("Puppet trash recursion prompt");
    assert_eq!(trash_view.kind, SelectionKind::Trash);
    assert!(
        runner.pending_is_optional(),
        "the return-to-hand selection is optional"
    );
    assert_eq!(
        trash_view.valid_action_ids.len(),
        1,
        "only the [Puppet]-trait Digimon is an eligible recursion target"
    );
    runner
        .execute_action(0, trash_view.valid_action_ids[0])
        .expect("return Puppet");
    runner.auto_resolve().expect("finish effect");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        hand_ids.contains(&"PUPPET-TRASH".to_string()),
        "the Puppet Digimon returned to hand"
    );
    assert!(
        !hand_ids.contains(&"DISCARD".to_string()),
        "the discarded card did not come back"
    );

    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"DISCARD".to_string()),
        "the discard cost landed the hand card in trash"
    );
    assert!(
        !trash_ids.contains(&"PUPPET-TRASH".to_string()),
        "the recurred Puppet left the trash"
    );
}

/// Negative (decline cost): declining the discard cost means the optional
/// return never happens — the Puppet stays in trash.
#[test]
fn ex9_024_decline_discard_does_not_return_trash_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_puppet("PUPPET-TRASH"))
        .add_card(make_filler("DISCARD"))
        .hand(0, &["EX9-024", "DISCARD"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, "PUPPET-TRASH");

    runner.play(0, 0).expect("play Hanimon");
    runner.execute_action(0, PASS).expect("decline discard");
    runner.auto_resolve().expect("finish decline");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert_eq!(
        hand_ids,
        vec!["DISCARD".to_string()],
        "declining the cost leaves the hand untouched"
    );
    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert_eq!(
        trash_ids,
        vec!["PUPPET-TRASH".to_string()],
        "no return happened — the Puppet stays in trash"
    );
}

/// Negative (non-Puppet trash card excluded): with only a non-Puppet Digimon
/// in trash, paying the discard cost yields no recursion — the [Puppet]
/// filter excludes the card, the candidate pool is empty, and the optional
/// recursion step silently short-circuits with no prompt and no return.
#[test]
fn ex9_024_on_play_non_puppet_trash_card_is_not_recursion_eligible() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_filler("NON-PUPPET-TRASH"))
        .add_card(make_filler("DISCARD"))
        .hand(0, &["EX9-024", "DISCARD"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, "NON-PUPPET-TRASH");

    runner.play(0, 0).expect("play Hanimon");
    runner
        .execute_action(0, PLAY_HAND_START)
        .expect("trash hand card");

    // No [Puppet]-trait Digimon in trash -> the optional recursion has zero
    // eligible candidates and resolves without offering a pick.
    assert!(
        runner.pending_selection().is_none(),
        "a non-Puppet trash card yields no eligible recursion target"
    );

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(
        !hand_ids.contains(&"NON-PUPPET-TRASH".to_string()),
        "the non-Puppet card was not pulled back to hand"
    );
    let trash_ids = zone_ids(&runner.game.players[0].trash, &runner.game.card_data);
    assert!(
        trash_ids.contains(&"NON-PUPPET-TRASH".to_string()),
        "the non-Puppet Digimon stays in trash"
    );
    assert!(
        trash_ids.contains(&"DISCARD".to_string()),
        "the discard cost was still paid"
    );
}

// ─── Section 4: inherited attack-cancel behavioral ───────────────────────────

/// Positive: on the opponent's turn, when their Digimon attacks, EX9-024's
/// inherited clause offers a selection over the controller's OTHER Digimon.
/// Choosing one deletes THAT Digimon (not the EX9-024 carrier) and ends the
/// attack with no security check.
#[test]
fn ex9_024_inherited_deletes_other_digimon_and_ends_opponent_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("OTHER"))
        .add_card(make_filler("ATTACKER"))
        .add_card(make_filler("SECURITY"))
        .security(0, &["SECURITY"])
        .start();
    // EX9-024 inherited under CARRIER — the carrier is the live Digimon.
    runner.place_stack(0, &["EX9-024", "CARRIER"]);
    // A genuinely OTHER Digimon for player 0 — the legal deletion cost.
    runner.place_on_field(0, "OTHER", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);

    let view = runner
        .pending_selection_view()
        .expect("other-Digimon deletion-cost selection installs");
    assert_eq!(
        view.kind,
        SelectionKind::OwnField,
        "the cost is a pick over the controller's own field"
    );
    assert!(
        runner.pending_is_optional(),
        "the attack cancel is declinable; the cost cannot auto-delete"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the OTHER Digimon is eligible — the EX9-024 carrier is excluded"
    );

    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("delete the other Digimon and end the attack");
    runner.auto_resolve().expect("finish attack cancel");

    assert_eq!(
        runner.security_count(0),
        1,
        "the attack ended before any security check"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "CARRIER"),
        "the EX9-024 carrier survives — the printed cost is an OTHER Digimon"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|perm| perm.top_card().card_id(&runner.game.card_data) != "OTHER"),
        "the chosen OTHER Digimon was deleted as the cost"
    );
    assert!(
        runner.game.pending_attack.is_none(),
        "the attack state is fully cleared"
    );
}

/// Negative (decline): declining the optional cost selection deletes nothing
/// and does NOT end the attack — the security check proceeds.
#[test]
fn ex9_024_inherited_decline_keeps_digimon_and_lets_attack_resolve() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("OTHER"))
        .add_card(make_filler("ATTACKER"))
        .add_card(make_filler("SECURITY"))
        .security(0, &["SECURITY"])
        .start();
    runner.place_stack(0, &["EX9-024", "CARRIER"]);
    runner.place_on_field(0, "OTHER", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);
    runner
        .execute_action(0, PASS)
        .expect("decline the attack cancel");

    assert_eq!(
        runner.battle_area_size(0),
        2,
        "declining deletes neither the carrier nor the other Digimon"
    );
    assert_eq!(
        runner.security_count(0),
        0,
        "declining does not end the attack — the security check happens"
    );
}

/// Negative (lone carrier — free-attack-end guard): with NO other Digimon on
/// the field, the inherited clause has no payable cost. The pre-filter
/// short-circuits with no prompt, and crucially the attack is NOT ended for
/// free — the security check proceeds.
#[test]
fn ex9_024_inherited_lone_carrier_does_not_end_attack_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("ATTACKER"))
        .add_card(make_filler("SECURITY"))
        .security(0, &["SECURITY"])
        .start();
    // Only the EX9-024 carrier stack — no OTHER Digimon to delete.
    runner.place_stack(0, &["EX9-024", "CARRIER"]);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);

    assert!(
        runner.pending_selection().is_none(),
        "no OTHER Digimon -> no deletion-cost prompt installs"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "the lone carrier was not deleted"
    );
    runner.auto_resolve().expect("resolve the attack");
    assert_eq!(
        runner.security_count(0),
        0,
        "the attack was NOT ended for free — the security check happened"
    );
}

/// OPT lockout: a second opponent attack in the SAME turn (by a different
/// opponent Digimon) does not re-offer the attack-cancel prompt — the
/// inherited clause's [Once Per Turn] lockout is keyed to EX9-024's clause,
/// not to the attacker.
#[test]
fn ex9_024_inherited_opt_locks_second_attack_same_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("OTHER1"))
        .add_card(make_filler("OTHER2"))
        .add_card(make_filler("ATTACKER1"))
        .add_card(make_filler("ATTACKER2"))
        .add_card(make_filler("SECURITY1"))
        .add_card(make_filler("SECURITY2"))
        .security(0, &["SECURITY1", "SECURITY2"])
        .start();
    runner.place_stack(0, &["EX9-024", "CARRIER"]);
    runner.place_on_field(0, "OTHER1", Some(0));
    runner.place_on_field(0, "OTHER2", Some(0));
    // Two separate opponent attackers so both attack declarations are legal
    // (a Digimon suspends when it attacks and cannot attack again).
    let attacker1 = runner.place_on_field(1, "ATTACKER1", Some(0));
    let attacker2 = runner.place_on_field(1, "ATTACKER2", Some(0));
    runner.end_turn();

    // First opponent attack — clause fires, pay the cost, end the attack.
    runner.attack_player(attacker1, 0, false);
    let view = runner
        .pending_selection_view()
        .expect("first attack offers the cancel prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("delete an other Digimon and end the attack");
    runner.auto_resolve().expect("finish first cancel");
    assert_eq!(
        runner.security_count(0),
        2,
        "first attack was cancelled — no security checked"
    );

    // Second opponent attack, same turn, by the other attacker — OPT must
    // lock the clause out.
    runner.attack_player(attacker2, 0, false);
    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn] blocks a second activation in the same turn"
    );
    runner.auto_resolve().expect("resolve the second attack");
    assert_eq!(
        runner.security_count(0),
        1,
        "the second attack was not cancelled — it checked one security"
    );
    assert_eq!(
        runner.battle_area_size(0),
        2,
        "OPT lockout means no second OTHER Digimon was deleted"
    );
}

/// Negative (not the opponent's turn): the inherited clause is gated by
/// `active_when: { opponents_turn: true }`. When the EX9-024 controller is
/// the attacker on their OWN turn, the clause must not fire.
#[test]
fn ex9_024_inherited_does_not_fire_on_controllers_own_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX9-024")
        .expect("EX9-024 YAML loads")
        .add_card(make_filler("CARRIER"))
        .add_card(make_filler("OTHER"))
        .add_card(make_filler("DEFENDER-SEC"))
        .security(1, &["DEFENDER-SEC"])
        .start();
    // EX9-024's controller (player 0) is the active player and the attacker.
    let attacker = runner.place_stack(0, &["EX9-024", "CARRIER"]);
    runner.place_on_field(0, "OTHER", Some(0));

    runner.attack_player(attacker, 1, false);

    assert!(
        runner.pending_selection().is_none(),
        "on the controller's own turn the inherited clause must not fire"
    );
}
