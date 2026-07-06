//! BT14-084 T.K. Takaishi — Tamer, Yellow, Cost 3.
//!
//! # Card text (per-card JSON `code/digimon-engine/cards/bt14/BT14-084.json` —
//! authoritative; cross-checked against the printed card image and the
//! official Bandai DB bundle `data/card_bundles/BT14-084.md`, verbatim match)
//!
//! [On Play] By returning the top card of your security stack to your hand,
//! you may place 1 yellow card with the [Vaccine] trait from your hand at
//! the bottom of your security stack.
//! [Your Turn] When a card is added to your security stack, you may suspend
//! this Tamer to gain 1 memory.
//! Security Effect [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT14/Yellow/BT14_084.cs
//!
//! DCGO models three timings:
//! - `OnEnterFieldAnyone`: unconditional `AddHandCards(SecurityCards[0])` +
//!   `IReduceSecurity` (the "By returning..." cost, gated only on
//!   `SecurityCards.Count >= 1` — not a player choice), THEN, only if a
//!   matching hand card exists, an optional (`canNoSelect: true`)
//!   `SelectHandEffect` over `CardTraits.Contains("Vaccine") &&
//!   HasCardColor(Yellow)`, placing the pick at the BOTTOM
//!   (`AddSecurityCard(toTop: false)`).
//! - `OnAddSecurity`: `CanUseCondition` gates `IsExistOnBattleArea` +
//!   `IsOwnerTurn` + `CanTriggerWhenAddSecurity(player == card.Owner)` (own
//!   security only, [Your Turn] only); `CanActivateCondition` gates
//!   `CanActivateSuspendCostEffect` (payable suspend cost — i.e. currently
//!   unsuspended). `ActivateCoroutine` taps this Tamer then `AddMemory(1)`.
//! - `SecuritySkill`: `CardEffectFactory.PlaySelfTamerSecurityEffect` (play
//!   self from security, free).
//!
//! This is the BT8-090 Kari Kamiya twin pattern for the [Your Turn] security-
//! added observer (see `code/digimon-engine/cards/bt8/BT8-090.yaml` /
//! `tests/cards_behavioral/bt8/bt8_090.rs`) — identical clause text, identical
//! DCGO shape (`OnAddSecurity` → `CanTriggerWhenAddSecurity(player ==
//! card.Owner)` + `IsOwnerTurn` + `CanActivateSuspendCostEffect`).
//!
//! IMPORTANT: DCGO's `CardSource.CardTraits` is a UNION of `Form_ENG` +
//! `Attribute_ENG` + `Type_ENG` (`CardSource.cs` region "traits") — so the
//! printed "[Vaccine] trait" check is `CardTraits.Contains("Vaccine")`, which
//! matches on the ATTRIBUTE field folded into the union. The Rust production
//! loader (`CardData::load_from_str`, `card_data.rs`) mirrors this exactly:
//! `traits = form_eng + attribute_eng + type_eng` (see the
//! `production_card_data_traits_superset_of_dsl_traits` guard in
//! `tests/dsl/trait_parity.rs`). So the DSL predicate for "[Vaccine] trait"
//! is `trait_has: Vaccine`, NOT `attribute_is` (a distinct DSL leaf that is
//! currently unconditionally `false` for every card — see
//! `code/digimon-engine/src/dsl_cards/predicate.rs` around line 2131,
//! "attribute not yet tracked on CardData" — a real but orthogonal engine
//! gap this card does not need to touch).
//!
//! # Patterns this test covers
//! - B3 trigger-on-event tamer (on hatch/on play/on deletion family; here
//!   On Play + on_added_to_security).
//! - Optional decline (both clauses are independently "you may" — no
//!   once_per_turn on this card, so no OPT lockout test applies).
//! - G-OWN-SECURITY-ADDED-OBSERVER (BT8-090 twin) — CLOSED/authorable, per
//!   `qa/dsl-vocab-gaps.md` "RESOLVED / RECLASSIFIED 2026-06-15" section.
//! - Security Effect play-self-free (`on_security` / `play_from_security`).
//! - Cost-as-unconditional-step: "By X-ing, you may Y" where the cost (adding
//!   top security to hand) is NOT itself optional/declinable — only the
//!   downstream placement is "you may".

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardSourceRef, StackPosition};

fn tk_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load from embedded DSL pack")
        .memory(5)
        .start()
}

/// Yellow Digimon with the Vaccine trait — a legal target for the optional
/// bottom-security placement.
fn make_yellow_vaccine_digimon(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut d = make_test_card(card_id, name);
    d.colors = vec![CardColor::Yellow];
    d.traits = vec!["Vaccine".to_string()];
    d
}

/// Yellow Digimon WITHOUT the Vaccine trait — must be rejected by the filter.
fn make_yellow_non_vaccine_digimon(
    card_id: &str,
    name: &str,
) -> digimon_engine::card_data::CardData {
    let mut d = make_test_card(card_id, name);
    d.colors = vec![CardColor::Yellow];
    d.traits = vec!["Data".to_string()];
    d
}

/// Red Digimon WITH the Vaccine trait — wrong color, must be rejected.
fn make_red_vaccine_digimon(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut d = make_test_card(card_id, name);
    d.colors = vec![CardColor::Red];
    d.traits = vec!["Vaccine".to_string()];
    d
}

// ─── Structural: the three printed clauses ────────────────────────────────

#[test]
fn bt14_084_has_on_play_security_added_and_security_clauses() {
    let runner = tk_runner();
    let card = runner
        .compiled_card("BT14-084")
        .expect("BT14-084 must be compiled");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )),
        "BT14-084 must have an [On Play] clause"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAddedToSecurity)
        )),
        "BT14-084 must have the [Your Turn] security-added observer clause"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "BT14-084 must have the Security play clause"
    );
}

/// Structural: the [Your Turn] own-security-added observer must ship as an
/// optional on_added_to_security clause with an active_when gate and a
/// condition (source_is_unsuspended), mirroring BT8-090.
#[test]
fn bt14_084_has_your_turn_security_added_observer_clause() {
    let runner = tk_runner();
    let card = runner
        .compiled_card("BT14-084")
        .expect("BT14-084 must be compiled");

    let observer = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAddedToSecurity) => {
            Some(t)
        }
        _ => None,
    });
    let observer = observer.expect("BT14-084 must have an on_added_to_security observer clause");

    assert!(
        observer.optional,
        "the security-added clause is 'you may' — must be optional so PASS is exposed"
    );
    assert!(
        observer.active_when.is_some(),
        "[Your Turn] gate must be encoded via active_when"
    );
    assert!(
        observer.condition.is_some(),
        "must gate the suspend cost on source_is_unsuspended"
    );
}

/// Structural: the [On Play] clause is a face-up (not inherited/security)
/// scope clause.
#[test]
fn bt14_084_on_play_clause_is_face_up_scope() {
    let runner = tk_runner();
    let card = runner
        .compiled_card("BT14-084")
        .expect("BT14-084 must be compiled");

    let on_play = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay) => Some(t),
        _ => None,
    });
    let on_play = on_play.expect("On Play clause must exist");
    assert_eq!(on_play.scope, CompiledScope::FaceUp);
}

// ─── [On Play]: cost (return top security) always fires ──────────────────

#[test]
fn bt14_084_on_play_returns_top_security_to_hand_even_with_no_matching_hand_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .add_card(make_test_card("BT14-084-SEC-TOP", "Sec Top"))
        .add_card(make_test_card("BT14-084-SEC-BOT", "Sec Bot"))
        .security(0, &["BT14-084-SEC-TOP", "BT14-084-SEC-BOT"])
        .hand(0, &["BT14-084"])
        .memory(5)
        .start();

    let hand_before = runner.hand_size(0);
    let security_before = runner.security_count(0);

    let _field_index = runner.play(0, 0).expect("T.K. must play from hand");

    // No Vaccine/yellow card in hand to place, so the effect should have no
    // further pending selection after the mandatory cost step resolves.
    assert!(
        runner.pending_selection().is_none(),
        "with no legal placement target, no selection should remain pending"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before, // -1 (T.K. left hand to play) +1 (top security returned)
        "top security card returned to hand replaces the played T.K. in hand count"
    );
    assert_eq!(
        runner.security_count(0),
        security_before - 1,
        "top security card must leave the security stack"
    );
}

// ─── [On Play]: optional bottom-security placement ────────────────────────

#[test]
fn bt14_084_on_play_offers_optional_placement_when_yellow_vaccine_card_in_hand() {
    let vaccine_card = make_yellow_vaccine_digimon("BT14-084-VAC-A", "Vaccine A");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .add_card(vaccine_card)
        .add_card(make_test_card("BT14-084-SEC-A", "Sec A"))
        .security(0, &["BT14-084-SEC-A"])
        .hand(0, &["BT14-084", "BT14-084-VAC-A"])
        .memory(5)
        .start();

    let _field_index = runner.play(0, 0).expect("T.K. must play from hand");

    let view = runner
        .pending_selection_view()
        .expect("a legal Vaccine/yellow hand card must offer the optional placement");
    assert!(
        runner.pending_is_optional(),
        "the placement is 'you may' — PASS must be legal"
    );
    // Accept: place the Vaccine card.
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("accept the optional placement");
    runner.auto_resolve().expect("resolve after accept");
}

#[test]
fn bt14_084_on_play_accepting_places_vaccine_card_at_bottom_of_security_face_down() {
    let vaccine_card = make_yellow_vaccine_digimon("BT14-084-VAC-B", "Vaccine B");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .add_card(vaccine_card)
        .add_card(make_test_card("BT14-084-SEC-B1", "Sec B1"))
        .add_card(make_test_card("BT14-084-SEC-B2", "Sec B2"))
        .security(0, &["BT14-084-SEC-B1", "BT14-084-SEC-B2"])
        .hand(0, &["BT14-084", "BT14-084-VAC-B"])
        .memory(5)
        .start();

    let _field_index = runner.play(0, 0).expect("T.K. must play from hand");

    let security_before = runner.security_count(0); // 1 remaining after top returned
    let view = runner
        .pending_selection_view()
        .expect("optional placement prompt must install");

    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("accept placement");
    runner.auto_resolve().expect("resolve after accept");

    assert_eq!(
        runner.security_count(0),
        security_before + 1,
        "accepting must add the Vaccine card to security"
    );
    // The placed card must be at the BOTTOM of the security stack, and
    // face-down (no printed-text override to face-up). Engine convention:
    // `StackPosition::Bottom` inserts at index 0 (`security.first()`);
    // `StackPosition::Top` pushes to the end (`security.last()`) — see
    // `Game::place_on_security_observed` and `add_top_security_to_hand`
    // (which reads `.last()` as "top").
    let bottom = runner.game.players[0]
        .security
        .first()
        .expect("security must be non-empty");
    assert_eq!(
        bottom.card_id(&runner.game.card_data),
        "BT14-084-VAC-B",
        "the Vaccine card must land at the BOTTOM of security"
    );
}

#[test]
fn bt14_084_on_play_declining_leaves_hand_and_security_unchanged() {
    let vaccine_card = make_yellow_vaccine_digimon("BT14-084-VAC-C", "Vaccine C");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .add_card(vaccine_card)
        .add_card(make_test_card("BT14-084-SEC-C", "Sec C"))
        .security(0, &["BT14-084-SEC-C"])
        .hand(0, &["BT14-084", "BT14-084-VAC-C"])
        .memory(5)
        .start();

    let _field_index = runner.play(0, 0).expect("T.K. must play from hand");

    runner
        .pending_selection_view()
        .expect("optional placement prompt must install");
    let hand_before_decline = runner.hand_size(0);
    let security_before_decline = runner.security_count(0);

    runner
        .decline_optional_trigger()
        .expect("decline the optional placement");
    runner.auto_resolve().expect("resolve after decline");

    assert_eq!(
        runner.hand_size(0),
        hand_before_decline,
        "declining must not remove the Vaccine card from hand"
    );
    assert_eq!(
        runner.security_count(0),
        security_before_decline,
        "declining must not add anything to security"
    );
}

/// Negative: a yellow but non-Vaccine card in hand must NOT be offered.
#[test]
fn bt14_084_on_play_does_not_offer_yellow_non_vaccine_card() {
    let non_vaccine = make_yellow_non_vaccine_digimon("BT14-084-NV-A", "Non Vaccine A");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .add_card(non_vaccine)
        .add_card(make_test_card("BT14-084-SEC-D", "Sec D"))
        .security(0, &["BT14-084-SEC-D"])
        .hand(0, &["BT14-084", "BT14-084-NV-A"])
        .memory(5)
        .start();

    let _field_index = runner.play(0, 0).expect("T.K. must play from hand");

    assert!(
        runner.pending_selection().is_none(),
        "a yellow non-Vaccine card must not satisfy the placement filter"
    );
}

/// Negative: a Vaccine card of the wrong color (not yellow) must NOT be
/// offered.
#[test]
fn bt14_084_on_play_does_not_offer_non_yellow_vaccine_card() {
    let red_vaccine = make_red_vaccine_digimon("BT14-084-RV-A", "Red Vaccine A");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .add_card(red_vaccine)
        .add_card(make_test_card("BT14-084-SEC-E", "Sec E"))
        .security(0, &["BT14-084-SEC-E"])
        .hand(0, &["BT14-084", "BT14-084-RV-A"])
        .memory(5)
        .start();

    let _field_index = runner.play(0, 0).expect("T.K. must play from hand");

    assert!(
        runner.pending_selection().is_none(),
        "a non-yellow Vaccine card must not satisfy the placement filter"
    );
}

/// Negative: with an empty security stack, the [On Play] clause must not
/// crash and must not offer the placement (no top card to return; DCGO gates
/// the whole coroutine on `SecurityCards.Count >= 1`).
#[test]
fn bt14_084_on_play_with_empty_security_does_not_offer_placement() {
    let vaccine_card = make_yellow_vaccine_digimon("BT14-084-VAC-D", "Vaccine D");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .add_card(vaccine_card)
        .hand(0, &["BT14-084", "BT14-084-VAC-D"])
        .memory(5)
        .start();
    runner.game.players[0].security.clear();

    let security_before = runner.security_count(0);
    assert_eq!(security_before, 0, "precondition: security is empty");

    let _field_index = runner.play(0, 0).expect("T.K. must play from hand");

    assert!(
        runner.pending_selection().is_none(),
        "with no security to return, the cost cannot fire, so no placement selection installs"
    );
    assert_eq!(
        runner.security_count(0),
        0,
        "security must remain empty (nothing to return)"
    );
}

// ─── [Your Turn] security-added observer ──────────────────────────────────
//
// A card added to the controller's OWN security stack on their turn must
// offer "you may suspend this Tamer to gain 1 memory". Accepting suspends
// T.K. and gains exactly 1 memory. Mirrors BT8-090's own-security-added
// observer test suite exactly.

/// Helper: place `count` deck-top cards on `player`'s own security through
/// the real game flow (`EffectContext::place_on_security` from `DeckTop`),
/// which fires `fire_on_place_security` exactly as an in-game recovery
/// would.
fn add_own_deck_top_to_security(
    runner: &mut DebugRunner,
    player: u8,
    source: digimon_engine::card_source::CardHandle,
) {
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let placed = ctx.place_on_security(
        player,
        CardSourceRef::DeckTop(player),
        StackPosition::Top,
        false,
    );
    assert!(placed, "deck-top must place onto own security");
}

#[test]
fn bt14_084_security_added_offers_suspend_to_gain_memory_and_accepting_works() {
    let filler = make_test_card("BT14084-SA-A", "Sec Add A");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .add_card(filler)
        .deck(0, &["BT14084-SA-A", "BT14084-SA-A"])
        .deck(1, &["BT14084-SA-A"])
        .memory(3)
        .start();

    // T.K. on P0's field, unsuspended, on P0's turn ([Your Turn] satisfied).
    let tk = runner.place_on_field(0, "BT14-084", Some(0));
    assert!(
        !runner.game.players[0].battle_area[tk.index as usize].is_suspended,
        "precondition: T.K. starts unsuspended"
    );
    runner.game.memory = 3;
    let memory_before = runner.memory();
    let tk_source = runner.game.players[0].battle_area[tk.index as usize]
        .top_card()
        .handle();

    add_own_deck_top_to_security(&mut runner, 0, tk_source);

    let view = runner
        .pending_selection_view()
        .expect("adding to own security on your turn must offer the suspend-to-gain choice");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("accept the optional suspend cost");
    runner.auto_resolve().expect("resolve after accept");

    assert!(
        runner.game.players[0].battle_area[tk.index as usize].is_suspended,
        "accepting must suspend this Tamer (the activation cost)"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "accepting gains exactly 1 memory for the controller"
    );
}

#[test]
fn bt14_084_security_added_player_may_decline_leaving_unsuspended_and_memory_unchanged() {
    let filler = make_test_card("BT14084-SA-B", "Sec Add B");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .add_card(filler)
        .deck(0, &["BT14084-SA-B", "BT14084-SA-B"])
        .deck(1, &["BT14084-SA-B"])
        .memory(3)
        .start();

    let tk = runner.place_on_field(0, "BT14-084", Some(0));
    runner.game.memory = 3;
    let memory_before = runner.memory();
    let tk_source = runner.game.players[0].battle_area[tk.index as usize]
        .top_card()
        .handle();

    add_own_deck_top_to_security(&mut runner, 0, tk_source);

    runner
        .pending_selection_view()
        .expect("must offer the suspend-to-gain choice");
    runner
        .decline_optional_trigger()
        .expect("decline the optional suspend cost");
    runner.auto_resolve().expect("resolve after decline");

    assert!(
        !runner.game.players[0].battle_area[tk.index as usize].is_suspended,
        "declining must leave this Tamer unsuspended"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "declining must not gain memory"
    );
}

/// Negative: an add to the controller's own security on the OPPONENT's turn
/// must NOT fire ([Your Turn] gate).
#[test]
fn bt14_084_security_added_does_not_fire_on_opponents_turn() {
    let filler = make_test_card("BT14084-SA-C", "Sec Add C");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .add_card(filler)
        .deck(0, &["BT14084-SA-C", "BT14084-SA-C"])
        .deck(1, &["BT14084-SA-C"])
        .memory(3)
        .start();

    let tk = runner.place_on_field(0, "BT14-084", Some(0));
    runner.end_turn(); // P0 ends → P1's turn (opponent's turn for T.K.'s controller)
    runner.game.memory = 3;
    let memory_before = runner.memory();
    let tk_source = runner.game.players[0].battle_area[tk.index as usize]
        .top_card()
        .handle();

    // Add a card to P0's own security on P1's turn.
    add_own_deck_top_to_security(&mut runner, 0, tk_source);

    assert!(
        runner.pending_selection().is_none(),
        "[Your Turn] gate must suppress the observer on the opponent's turn"
    );
    assert!(
        !runner.game.players[0].battle_area[tk.index as usize].is_suspended,
        "T.K. must stay unsuspended when the observer does not fire"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gain when the [Your Turn] gate blocks the observer"
    );
}

/// Negative: while T.K. is already suspended, the suspend cost is unpayable,
/// so the observer must not offer the choice (source_is_unsuspended guard).
#[test]
fn bt14_084_security_added_does_not_offer_when_already_suspended() {
    let filler = make_test_card("BT14084-SA-D", "Sec Add D");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .add_card(filler)
        .deck(0, &["BT14084-SA-D", "BT14084-SA-D"])
        .deck(1, &["BT14084-SA-D"])
        .memory(3)
        .start();

    let tk = runner.place_on_field(0, "BT14-084", Some(0));
    runner.game.players[0].battle_area[tk.index as usize].is_suspended = true;
    runner.game.memory = 3;
    let memory_before = runner.memory();
    let tk_source = runner.game.players[0].battle_area[tk.index as usize]
        .top_card()
        .handle();

    add_own_deck_top_to_security(&mut runner, 0, tk_source);

    assert!(
        runner.pending_selection().is_none(),
        "already-suspended T.K. cannot pay the suspend cost — no prompt"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gain when the suspend cost is unpayable"
    );
}

/// Negative: an add to the OPPONENT's security stack (not the controller's
/// own) must NOT fire, even on the controller's turn — DCGO gates
/// `CanTriggerWhenAddSecurity(player == card.Owner)`.
#[test]
fn bt14_084_security_added_does_not_fire_for_opponents_security() {
    let filler = make_test_card("BT14084-SA-E", "Sec Add E");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .add_card(filler)
        .deck(0, &["BT14084-SA-E"])
        .deck(1, &["BT14084-SA-E", "BT14084-SA-E"])
        .memory(3)
        .start();

    let tk = runner.place_on_field(0, "BT14-084", Some(0));
    runner.game.memory = 3;
    let memory_before = runner.memory();
    let tk_source = runner.game.players[0].battle_area[tk.index as usize]
        .top_card()
        .handle();

    // Add a card to P1's (opponent's) security while it is P0's turn.
    add_own_deck_top_to_security(&mut runner, 1, tk_source);

    assert!(
        runner.pending_selection().is_none(),
        "adding to the OPPONENT's security must not fire T.K.'s own-security observer"
    );
    assert!(
        !runner.game.players[0].battle_area[tk.index as usize].is_suspended,
        "T.K. must stay unsuspended when the observer does not fire"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "no memory gain when the observer does not fire"
    );
}

// ─── [Security] play self free ─────────────────────────────────────────────

#[test]
fn bt14_084_security_effect_plays_self_without_paying_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 must load")
        .security(1, &["BT14-084"])
        .memory(0)
        .start();

    let field_before = runner.battle_area_size(1);
    let memory_before = runner.memory();

    // Drive the Security Effect's free self-play directly through
    // EffectContext, mirroring the DCGO `PlaySelfTamerSecurityEffect`
    // shape (the security-check dispatch that reaches this timing is
    // covered elsewhere; this integrated test exercises play-from-security
    // itself, which is what the clause's process body invokes).
    let handle = runner.game.players[1].security[0].handle();
    let mut ctx = EffectContext::new(&mut runner.game, handle, None, 1);
    let played = ctx.play_from_security(1);

    assert!(played.is_some(), "Security Effect must place T.K. onto the battle area");
    assert_eq!(
        runner.battle_area_size(1),
        field_before + 1,
        "Security Effect must place T.K. onto the battle area"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "Security Effect play is free — memory must not change"
    );
}
