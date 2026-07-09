//! P-193 The Wicked God Emerges! — Option, Cost 3, Purple, [Wicked God] trait.
//!
//! # Card text (data/card_bundles/P-193.md — official Bandai DB)
//!
//! [Main] By trashing 1 card with the [Composite] or [Wicked God] trait from
//! your hand, ＜Draw 2＞ (Draw 2 cards from your deck.) Then, place this card
//! in the battle area.
//! [End of All Turns] ＜Delay＞ (By trashing this card after the placing
//! turn, activate the effect below.)
//! ・By deleting 1 of your [Millenniummon], you may play 1 [Wicked God]
//! trait Digimon card from your hand or trash without paying the cost.
//!
//! Security Effect [Security] Activate this card's [Main] effects.
//!
//! Official Q&A: declining/failing the hand-trash cost means NEITHER the
//! draw NOR the battle-area placement happens (Clause A is a hard cost gate).
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Purple/P_193.cs
//!
//! # Patterns this test file covers
//!
//! - Clause A (Main): `select_hand { cost: true }` trash-gated Draw 2, then
//!   `place_self_as_delay_option`. The whole clause is skipped (no prompt)
//!   when no [Composite]/[Wicked God] card is in hand (`condition: count_gte`).
//! - Clause B (End of All Turns <Delay>): standard `trigger: delayed`
//!   (`activate_delayed_option_main` performs the outer "trash this card"
//!   activation cost automatically). The inner "By deleting 1 Millenniummon,
//!   you may play..." is modelled as `select_own_permanent { optional: true,
//!   then: [...] }` — the DSL's default `continue_on_decline: false` means
//!   declining (or a zero-Millenniummon no-op) drops the ENTIRE `then:` tail,
//!   so the free-play `select_union_zone` prompt is never installed unless a
//!   Millenniummon is actually deleted.
//! - Clause C (Security, mirrors the Main clause verbatim per DCGO
//!   `AddActivateMainOptionSecurityEffect`).

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause, CompiledScope,
    CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::SelectionKind;

const P_193_YAML: &str = include_str!("../../../cards/p/P-193.yaml");

// ─── Helper cards ────────────────────────────────────────────────────────────

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// A hand/trash card carrying the [Composite] or [Wicked God] trait — the
/// Clause A / Clause C trash-cost fodder.
fn make_traited_digimon(id: &str, name: &str, trait_name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.traits = vec![trait_name.to_string()];
    c
}

/// A Digimon named exactly "Millenniummon" (Clause B's delete-cost target).
fn make_millenniummon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Millenniummon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(6);
    c.dp = Some(13000);
    c.play_cost = 15;
    c
}

/// A non-Millenniummon Digimon (negative control for the delete-cost filter).
fn make_other_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c
}

/// A [Wicked God] trait Digimon card (the free-play reward target).
fn make_wicked_god_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = 14;
    c.traits = vec!["Wicked God".to_string()];
    c
}

fn p193_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(P_193_YAML)
        .expect("P-193 YAML must parse")
        .memory(10)
        .start()
}

/// Push a card straight into a player's trash zone (bypassing hand/discard
/// flow) — mirrors the seeding helper used by BT17-095's tests.
fn seed_trash(runner: &mut DebugRunner, player: usize, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|d| d.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} card data registered"));
    let card_index = runner.game.next_card_index();
    runner.game.players[player]
        .trash
        .push(CardSource::new(data_index, player as u8, card_index));
}

fn find_delayed_p193(runner: &DebugRunner, player: usize) -> Option<usize> {
    runner.game.players[player]
        .battle_area
        .iter()
        .position(|perm| {
            perm.top_card().card_id(&runner.game.card_data) == "P-193"
                && matches!(perm.option_state, OptionState::Delayed { .. })
        })
}

/// Advance turns until the Delay option seated at `delay_idx` is legally
/// activatable (`turn_count > placed_on_turn`), mirroring BT13-110's loop.
fn advance_past_placing_turn(runner: &mut DebugRunner, player: usize, delay_idx: usize) {
    let placed_on_turn = match runner.game.players[player].battle_area[delay_idx].option_state {
        OptionState::Delayed { placed_on_turn, .. } => placed_on_turn,
        _ => panic!("expected a Delay option at index {delay_idx}"),
    };
    for _ in 0..4 {
        if runner.game.turn_count > placed_on_turn {
            break;
        }
        runner.end_turn();
        runner.game.enter_main_phase();
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — structural / metadata
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p193_has_printed_metadata() {
    let runner = p193_runner();
    let card = runner.compiled_card("P-193").expect("P-193 present");
    assert_eq!(card.name, "The Wicked God Emerges!");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.color, vec![CompiledColor::Purple]);
    assert!(card.traits.iter().any(|t| t == "Wicked God"));
}

#[test]
fn p193_main_clause_is_optional_face_up_and_gated_on_hand_fodder() {
    let runner = p193_runner();
    let card = runner.compiled_card("P-193").expect("P-193 present");
    let main = card
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(triggered)
                if triggered.when.contains(&CompiledTiming::MainFromHand) =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("[Main] clause exists");
    assert!(
        main.optional,
        "printed 'By trashing X, <Draw 2>' is a cost-gated effect (declinable)"
    );
    assert_eq!(main.scope, CompiledScope::FaceUp);
    assert!(
        main.condition.is_some(),
        "Main clause must gate on hand having an eligible Composite/Wicked God card"
    );
    assert!(main
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)));
}

#[test]
fn p193_has_delay_clause_with_main_phase_trigger() {
    let runner = p193_runner();
    let card = runner.compiled_card("P-193").expect("P-193 present");
    let has_delay = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Delay {
                trigger: CompiledTiming::Delayed,
                ..
            })
        )
    });
    assert!(
        has_delay,
        "P-193 must have a `kind: delay` clause with `trigger: delayed` \
         (standard printed <Delay>, matching DCGO's CanDeclareOptionDelayEffect gate)"
    );
}

#[test]
fn p193_security_clause_mirrors_main_and_is_optional() {
    let runner = p193_runner();
    let card = runner.compiled_card("P-193").expect("P-193 present");
    let sec = card
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("[Security] clause exists");
    assert!(
        sec.optional,
        "Security mirrors the cost-gated Main effect (declinable)"
    );
    assert!(sec
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlaceSelfAsDelayOption)));
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Clause A: [Main] trash-cost-gated Draw 2 + place self
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn p193_main_trashing_fodder_draws_two_and_places_self() {
    let composite = make_traited_digimon("P193-COMP", "CompositeGuy", "Composite");
    let filler = make_filler("FILL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_193_YAML)
        .expect("P-193 YAML parses")
        .add_card(composite.clone())
        .add_card(filler.clone())
        .memory(10)
        .hand(0, &["P-193", "P193-COMP"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "P-193 [Main] must activate from hand");

    let view = runner
        .pending_selection_view()
        .expect("the trash-cost pick must install");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        view.is_optional,
        "the trash pick is declinable (cost: true)"
    );
    // Two eligible cards: P-193 itself (hand index 0 — it carries the
    // printed [Wicked God] trait and is still IN HAND at cost-payment time,
    // matching DCGO's CanSelectHandCondition which scans the whole hand) and
    // CompositeGuy (hand index 1). Explicitly target the Composite card via
    // its PLAY_HAND_START-relative action id — do not rely on array order.
    use digimon_engine::action::space::PLAY_HAND_START;
    let composite_action = PLAY_HAND_START + 1;
    assert!(
        view.valid_action_ids.contains(&composite_action),
        "CompositeGuy (hand index 1) must be a valid trash-cost target"
    );
    runner
        .game
        .resolve_selection(view.selecting_player, composite_action)
        .expect("accept the trash cost by discarding CompositeGuy");

    // Drain: draw 2 + place self should follow automatically.
    runner.game.drain_effect_queue();

    // Net: -1 (CompositeGuy trashed) +2 (drawn) -1 (P-193 itself leaves hand
    // to the battle area via place_self_as_delay_option) = net 0 vs before.
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "hand size unchanged overall: -1 trash, +2 draw, -1 P-193 leaves to battle area"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "2 cards drawn from deck"
    );
    assert_eq!(runner.trash_size(0), 1, "the Composite card was trashed");
    assert!(
        find_delayed_p193(&runner, 0).is_some(),
        "P-193 must be placed in the battle area as a Delay option"
    );
}

/// Faithfulness edge case: P-193 itself carries the printed [Wicked God]
/// trait, so while it is still in hand (before the "then place this card in
/// the battle area" tail resolves) it is itself a legal — if unusual — target
/// for its own "trashing 1 [Composite]/[Wicked God] card" cost, matching
/// DCGO's `CanSelectHandCondition` (which scans the whole hand, including the
/// source card). Trashing itself pays the cost and Draws 2; the "then place
/// this card in the battle area" tail then finds P-193 in the trash (its
/// only remaining zone) and moves it onto the field as a Delay option —
/// `place_self_as_delay_option`'s controller-zone fallback
/// (`remove_source_option_from_controller_zones`) searches hand THEN trash
/// unconditionally, so it recovers the just-self-discarded card. No panic,
/// no illegal state; P-193 ends up seated as a Delay option, having paid its
/// "cost" from its own printed traits.
#[test]
fn p193_main_can_trash_itself_as_its_own_cost_and_still_draws() {
    let filler = make_filler("FILL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_193_YAML)
        .expect("P-193 YAML parses")
        .add_card(filler.clone())
        .memory(10)
        .hand(0, &["P-193"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let deck_before = runner.deck_size(0);

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "P-193 [Main] must activate from hand");

    let view = runner
        .pending_selection_view()
        .expect("the trash-cost pick must install");
    assert_eq!(view.kind, SelectionKind::Hand);
    use digimon_engine::action::space::PLAY_HAND_START;
    assert!(
        view.valid_action_ids.contains(&PLAY_HAND_START),
        "P-193 itself (hand index 0) is a legal trash-cost target — it \
         carries the printed [Wicked God] trait and is still in hand"
    );
    runner
        .game
        .resolve_selection(view.selecting_player, PLAY_HAND_START)
        .expect("trash P-193 itself as the cost");

    runner.game.drain_effect_queue();

    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "Draw 2 still resolves even though the cost card was P-193 itself"
    );
    assert_eq!(
        runner.trash_size(0),
        0,
        "P-193 does not remain in the trash — place_self_as_delay_option \
         recovers it from trash and seats it on the field"
    );
    assert!(
        find_delayed_p193(&runner, 0).is_some(),
        "P-193 ends up seated as a Delay option on the field, recovered \
         from the trash it briefly occupied as its own cost payment"
    );
}

/// FALSE-GREEN FIX (reviewer directive): the previous version of this test
/// staged NO Purple Digimon/Tamer anywhere for player 0, so the mask bit was
/// forced to `0.0` by the independent §4.2 Option color requirement
/// (`option_use_requirement_or_color_available` in `action/mask.rs`, checked
/// immediately after — and regardless of — `option_has_active_main_effect`'s
/// `condition: count_gte` fodder gate). The old assertion was therefore
/// vacuously true: it passed even though the fodder-count condition was never
/// actually exercised. Proven by direct experiment: staging a Purple anchor
/// on the field while keeping the exact same "no external fodder" hand
/// (`["P-193", "FILL"]`) flips the mask bit to `1.0` — i.e. the ONLY thing
/// keeping the old test green was the missing color anchor, not the fodder
/// gate it claimed to prove.
///
/// That experiment also surfaces the reason this card's `condition: count_gte
/// { zone: [hand], n: 1 }` can never be observed blocking its OWN
/// `PLAY_HAND_START` mask bit: P-193 prints the `[Wicked God]` trait itself
/// (`traits: ["Wicked God"]` in P-193.yaml), and `count_matching` /
/// `count_card_sources` (dsl_cards/predicate.rs) scan the ENTIRE hand zone
/// with no self-exclusion for the card whose effect is being evaluated —
/// exactly mirroring DCGO's `CanSelectHandCondition`, which the YAML's own
/// doc comment notes "scans the whole hand, including the source card" (see
/// also `p193_main_can_trash_itself_as_its_own_cost_and_still_draws` above,
/// which exercises this same self-qualification for the trash-cost pick
/// itself). So whenever P-193 sits in the hand being scanned, it always
/// satisfies its own `n: 1` fodder floor — the Main-from-hand mask bit can
/// never be legitimately forced to `0.0` by the fodder gate while P-193 is
/// the very card in that hand slot.
///
/// This test instead genuinely isolates BOTH gates for the Main/mask path:
/// - Positive control: color satisfied (Purple anchor on field) + hand
///   containing P-193 alone (self-qualifying fodder, zero external fodder
///   needed) -> mask bit legal. This proves the color gate is satisfied
///   independently of any external Composite/Wicked-God card.
/// - The genuine "zero eligible fodder" case for `count_gte` is proven below
///   in Section 4 via the Security path (`p193_security_no_eligible_fodder_...`),
///   where P-193 sits in the security zone rather than hand, so it cannot
///   self-qualify and the fodder gate is cleanly isolated from the color gate
///   (the §4.2 color requirement only gates hand-play legality, not security
///   reveals — confirmed by `p193_security_trashing_fodder_draws_two_and_places_self`
///   above, which never stages a color anchor and still fires from security).
#[test]
fn p193_main_color_satisfied_self_qualifying_fodder_offers_activation() {
    let purple_anchor = make_other_digimon("P193-ANCHOR", "PurpleAnchor");
    let filler = make_filler("FILL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_193_YAML)
        .expect("P-193 YAML parses")
        .add_card(purple_anchor.clone())
        .add_card(filler.clone())
        .memory(10)
        .hand(0, &["P-193"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    // Satisfy the §4.2 Option color requirement independently of the printed
    // trash-cost fodder: a Purple Digimon on the battle area.
    runner.place_on_field(0, "P193-ANCHOR", Some(0));

    let mask = digimon_engine::action::mask::build_action_mask(&runner.game, 0);
    let p193_hand_index = runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == "P-193")
        .expect("P-193 in hand");
    use digimon_engine::action::space::PLAY_HAND_START;
    assert_eq!(
        mask[(PLAY_HAND_START as usize) + p193_hand_index],
        1.0,
        "with the color requirement satisfied, P-193's [Main] must be legal: \
         it self-qualifies as its own [Wicked God] trait fodder even with no \
         external Composite/Wicked-God card in hand"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Clause B: [End of All Turns] <Delay> — delete-gated free play
// ═══════════════════════════════════════════════════════════════════════════

/// Place P-193 directly as a Delay option on the field (bypassing the Main
/// clause) so Clause B tests can focus on the delay body alone.
fn seat_p193_as_delay(runner: &mut DebugRunner, player: usize) -> usize {
    let handle = runner.place_on_field(player as u8, "P-193", Some(0));
    let perm = &mut runner.game.players[player].battle_area[handle.index as usize];
    perm.option_state = OptionState::Delayed {
        owner: player as u8,
        trash_on_turn: u16::MAX,
        trigger: DelayTrigger::MainPhaseActivated,
        placed_on_turn: 0,
    };
    handle.index as usize
}

#[test]
fn p193_delay_deleting_millenniummon_offers_free_wicked_god_play_from_hand() {
    let millenniummon = make_millenniummon("P193-MILL");
    let wicked_god = make_wicked_god_digimon("P193-WG", "SomeWickedGod");
    let filler = make_filler("FILL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_193_YAML)
        .expect("P-193 YAML parses")
        .add_card(millenniummon.clone())
        .add_card(wicked_god.clone())
        .add_card(filler.clone())
        .memory(10)
        .hand(0, &["P193-WG"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let delay_idx = seat_p193_as_delay(&mut runner, 0);
    runner.place_on_field(0, "P193-MILL", Some(0));
    advance_past_placing_turn(&mut runner, 0, delay_idx);

    let delay_handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: delay_idx as u8,
    };
    assert!(
        runner.game.activate_delayed_option_main(delay_handle),
        "the <Delay> must be activatable after the placing turn"
    );

    // The body installs a selection (the Millenniummon delete pick), so the
    // outer "trash this card" activation cost is DEFERRED until the whole
    // body resolves (`activate_delayed_option_main`'s documented
    // MainPhaseActivation resume behavior) — P-193 is still physically on
    // the field at this point; it trashes once the body (including the
    // nested free-play tail) finishes.
    let view = runner
        .pending_selection_view()
        .expect("Millenniummon delete pick must install");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.is_optional, "the delete pick is declinable");
    let (action, player) = (view.valid_action_ids[0], view.selecting_player);
    runner
        .game
        .resolve_selection(player, action)
        .expect("delete the Millenniummon");

    // Millenniummon deletion succeeded -> the free-play union prompt installs.
    let view2 = runner
        .pending_selection_view()
        .expect("free-play union-zone prompt must install after a successful delete");
    assert_eq!(
        view2.kind,
        SelectionKind::UnionZone {
            zones: digimon_engine::selection::UnionZoneSet::HAND
                | digimon_engine::selection::UnionZoneSet::TRASH
        }
    );
    assert!(view2.is_optional, "the free play itself is 'you may'");
    let (action2, player2) = (view2.valid_action_ids[0], view2.selecting_player);
    runner
        .game
        .resolve_selection(player2, action2)
        .expect("play the Wicked God card free");

    runner.game.drain_effect_queue();

    // Now that the body has fully resolved, the deferred trash-self
    // activation cost has been paid: P-193 is no longer a Delay permanent
    // on the field.
    assert!(
        find_delayed_p193(&runner, 0).is_none(),
        "P-193 must have been trashed as the <Delay> activation cost"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "P193-WG"),
        "the Wicked God Digimon must be on the battle area, played for free"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "P193-MILL"),
        "Millenniummon must have been deleted"
    );
}

/// UNFAITHFUL-GATING REGRESSION GUARD: declining the Millenniummon delete
/// pick must NOT expose the free-play prompt, even though an eligible
/// [Wicked God] card sits in hand. This is the exact bug the reviewer's fix
/// directive closes — the earlier flat (ungated) body let the union-zone
/// prompt install unconditionally.
#[test]
fn p193_delay_declining_deletion_skips_free_play() {
    let millenniummon = make_millenniummon("P193-MILL");
    let wicked_god = make_wicked_god_digimon("P193-WG-DECLINE", "DeclinedWickedGod");
    let filler = make_filler("FILL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_193_YAML)
        .expect("P-193 YAML parses")
        .add_card(millenniummon.clone())
        .add_card(wicked_god.clone())
        .add_card(filler.clone())
        .memory(10)
        .hand(0, &["P193-WG-DECLINE"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let delay_idx = seat_p193_as_delay(&mut runner, 0);
    runner.place_on_field(0, "P193-MILL", Some(0));
    advance_past_placing_turn(&mut runner, 0, delay_idx);

    let delay_handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: delay_idx as u8,
    };
    assert!(runner.game.activate_delayed_option_main(delay_handle));

    let view = runner
        .pending_selection_view()
        .expect("Millenniummon delete pick must install");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.is_optional);
    runner
        .game
        .resolve_selection(view.selecting_player, PASS)
        .expect("decline the Millenniummon delete pick");

    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "declining the delete cost must abort the whole free-play tail — \
         no union-zone prompt may install even though an eligible [Wicked \
         God] card sits in hand"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "P193-MILL"),
        "Millenniummon must NOT have been deleted after a decline"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "P193-WG-DECLINE"),
        "the Wicked God card must remain unplayed in hand"
    );
}

/// With no Millenniummon on the field, the delete pick itself must be a
/// no-op (no eligible candidates) and the free-play tail must not run.
#[test]
fn p193_delay_no_millenniummon_on_field_skips_delete_and_free_play() {
    let wicked_god = make_wicked_god_digimon("P193-WG-NOMILL", "NoMillWickedGod");
    let filler = make_filler("FILL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_193_YAML)
        .expect("P-193 YAML parses")
        .add_card(wicked_god.clone())
        .add_card(filler.clone())
        .memory(10)
        .hand(0, &["P193-WG-NOMILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let delay_idx = seat_p193_as_delay(&mut runner, 0);
    // No Millenniummon anywhere on the field.
    advance_past_placing_turn(&mut runner, 0, delay_idx);

    let delay_handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: delay_idx as u8,
    };
    assert!(runner.game.activate_delayed_option_main(delay_handle));

    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "with zero Millenniummon candidates, neither the delete pick nor \
         the free-play prompt may install (no panic, no over-exposed choice)"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "P193-WG-NOMILL"),
        "the Wicked God card must remain unplayed in hand"
    );
}

/// Deletion succeeds but the player may still decline the free play itself
/// (the "you may play" leg is independently optional).
#[test]
fn p193_delay_deletion_succeeds_but_free_play_is_declinable() {
    let millenniummon = make_millenniummon("P193-MILL");
    let wicked_god = make_wicked_god_digimon("P193-WG2", "DeclineAfterDelete");
    let filler = make_filler("FILL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_193_YAML)
        .expect("P-193 YAML parses")
        .add_card(millenniummon.clone())
        .add_card(wicked_god.clone())
        .add_card(filler.clone())
        .memory(10)
        .hand(0, &["P193-WG2"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let delay_idx = seat_p193_as_delay(&mut runner, 0);
    runner.place_on_field(0, "P193-MILL", Some(0));
    advance_past_placing_turn(&mut runner, 0, delay_idx);

    let delay_handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: delay_idx as u8,
    };
    assert!(runner.game.activate_delayed_option_main(delay_handle));

    let view = runner
        .pending_selection_view()
        .expect("Millenniummon delete pick must install");
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("delete the Millenniummon");

    let view2 = runner
        .pending_selection_view()
        .expect("free-play prompt installs after a successful delete");
    assert!(
        view2.is_optional,
        "the free play is independently 'you may'"
    );
    runner
        .game
        .resolve_selection(view2.selecting_player, PASS)
        .expect("decline the free play");

    runner.game.drain_effect_queue();

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "P193-MILL"),
        "Millenniummon was still deleted (that cost was already paid)"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "P193-WG2"),
        "the Wicked God card remains unplayed after declining the free play"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|p| p.top_card().card_id(&runner.game.card_data) != "P193-WG2"),
        "the Wicked God card must not be on the battle area"
    );
}

/// A non-Millenniummon Digimon on the field must not be offered as a delete
/// target (exact-name filter, not a substring/trait match).
#[test]
fn p193_delay_non_millenniummon_digimon_is_not_a_valid_delete_target() {
    let other = make_other_digimon("P193-OTHER", "NotMillenniummon");
    let filler = make_filler("FILL");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_193_YAML)
        .expect("P-193 YAML parses")
        .add_card(other.clone())
        .add_card(filler.clone())
        .memory(10)
        .hand(0, &["FILL"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let delay_idx = seat_p193_as_delay(&mut runner, 0);
    runner.place_on_field(0, "P193-OTHER", Some(0));
    advance_past_placing_turn(&mut runner, 0, delay_idx);

    let delay_handle = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: delay_idx as u8,
    };
    assert!(runner.game.activate_delayed_option_main(delay_handle));

    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "a non-Millenniummon Digimon must not be a valid delete target — \
         the clause must no-op cleanly"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "P193-OTHER"),
        "the non-Millenniummon Digimon must remain on the field, untouched"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Clause C: [Security] Activate [Main] effects
// ═══════════════════════════════════════════════════════════════════════════

/// Drives a REAL attack into P-193 seated as the top security card, so the
/// engine's actual security-reveal machinery (`pending_security` +
/// `SecuritySkill` dispatch) exercises Clause C end-to-end — mirroring the
/// BT15-092 `run_security_effect` idiom (`attack_player` + `auto_resolve`).
/// A manually-placed-on-field + `enqueue_triggered(SecuritySkill, ...)`
/// simulation is NOT faithful here: `place_self_as_delay_option_permanent`'s
/// already-on-field branch (`remove_source_card_from_permanent`) refuses to
/// remove a permanent's own sole top card, so that shortcut silently no-ops
/// instead of reproducing the real hand/trash-origin `pending_security` path.
#[test]
fn p193_security_trashing_fodder_draws_two_and_places_self() {
    let composite = make_traited_digimon("P193-SEC-COMP", "SecCompositeGuy", "Wicked God");
    let attacker_dgm = make_other_digimon("P193-ATK", "Attacker");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_193_YAML)
        .expect("P-193 YAML parses")
        .add_card(composite.clone())
        .add_card(attacker_dgm.clone())
        .memory(10)
        .hand(0, &["P193-SEC-COMP"])
        .security(0, &["P-193"])
        .start();

    let attacker = runner.place_on_field(1, "P193-ATK", Some(0));

    let _ = runner.attack_player(attacker, 0, false);

    // Drive up to (and including) the trash-cost pick manually, so the test
    // asserts on the exact selection surfaced rather than blindly trusting
    // auto_resolve's `first()` pick.
    let view = runner
        .pending_selection_view()
        .expect("the trash-cost pick must install from the real security reveal");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        view.is_optional,
        "the trash pick is declinable (cost: true)"
    );
    runner
        .game
        .resolve_selection(view.selecting_player, view.valid_action_ids[0])
        .expect("accept the trash cost");

    // Let the rest of the attack (draw 2, place self, security resolution
    // wrap-up) auto-resolve.
    runner.auto_resolve().ok();

    assert_eq!(runner.trash_size(0), 1, "the fodder card was trashed");
    assert!(
        find_delayed_p193(&runner, 0).is_some(),
        "P-193 must be placed in the battle area as a Delay option from security"
    );
}

/// GENUINE fodder-gate isolation (replaces the false-green
/// `p193_main_no_eligible_fodder_does_not_offer_activation`): here P-193 sits
/// in the SECURITY zone, not hand, so it cannot self-qualify as its own
/// `[Wicked God]` trait fodder the way it does when checked from hand (see
/// `p193_main_color_satisfied_self_qualifying_fodder_offers_activation` above
/// and `p193_main_can_trash_itself_as_its_own_cost_and_still_draws`). With
/// zero [Composite]/[Wicked God] cards anywhere in the defender's hand, the
/// `condition: count_gte` gate on the `[Security]` clause (which mirrors the
/// `[Main]` clause's condition verbatim) must prevent the effect from firing
/// at all: no trash-cost selection installs, no draw, no placement — P-193
/// just resolves as an ordinary broken security card. Note this path does
/// NOT depend on (and is not confounded by) the §4.2 Option color
/// requirement, which only gates hand-play mask legality — the existing
/// `p193_security_trashing_fodder_draws_two_and_places_self` positive control
/// above already demonstrates the security-reveal path fires with no color
/// anchor staged at all.
#[test]
fn p193_security_no_eligible_fodder_skips_activation() {
    let filler = make_filler("FILL");
    let attacker_dgm = make_other_digimon("P193-SEC-ATK", "SecAttacker");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_193_YAML)
        .expect("P-193 YAML parses")
        .add_card(filler.clone())
        .add_card(attacker_dgm.clone())
        .memory(10)
        .hand(0, &["FILL"])
        .security(0, &["P-193"])
        .start();

    let attacker = runner.place_on_field(1, "P193-SEC-ATK", Some(0));

    let _ = runner.attack_player(attacker, 0, false);
    runner.auto_resolve().ok();

    assert!(
        runner.pending_selection().is_none(),
        "no trash-cost selection may install when the defender's hand has \
         zero [Composite]/[Wicked God] cards"
    );
    assert_eq!(
        runner.trash_size(0),
        1,
        "P-193 must resolve as an ordinary broken security card (trashed), \
         with no Draw 2 and no battle-area placement"
    );
    assert!(
        find_delayed_p193(&runner, 0).is_none(),
        "P-193 must NOT be placed in the battle area as a Delay option when \
         the [Security] clause's fodder condition is not met"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "FILL"),
        "the filler hand card must remain untouched (no trash-cost pick fired)"
    );
}
