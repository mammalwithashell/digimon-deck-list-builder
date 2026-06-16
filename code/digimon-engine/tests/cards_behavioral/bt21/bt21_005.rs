//! BT21-005 Swipemon — Digi-Egg (In-Training), Lv.2, Green, Cost 0, no DP.
//! Traits: Appmon, Social, Swipe. Attribute: Social.
//!
//! # Card text (card image — authoritative)
//!
//! ```text
//! Inherited Effect [Your Turn] [Once Per Turn]
//! When this Digimon gets linked, ＜Draw 1＞ (Draw 1 card from your deck.)
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Green/BT21_005.cs
//!   - EffectTiming.WhenLinked, SetIsInheritedEffect(true), maxUsableCount: 1.
//!   - CanUseCondition: CanTriggerWhenLinked(PermanentCondition: this host,
//!       CardCondition: host is on battle area).
//!   - CanActivateCondition: IsExistOnBattleAreaDigimon && IsOwnerTurn.
//!   - ActivateCoroutine: DrawClass(card.Owner, 1). isOptional: false.
//!
//! # Patterns covered (RUST_DSL_TEST_API §4.3)
//! - G4: Inherited Digi-Egg — once_per_turn OnLink trigger via `scope: inherited`
//!   + `when: when_card_linked_to_this` (the egg is a digivolution source under
//!   the host; inherited dispatch walks card_sources via Phase 2 Track D).
//! - B3: Board-wide link observer (host-side; host self-filter: the link attaches
//!   to THIS host, not a sibling).
//! - E2: Once Per Turn OPT lockout + [Your Turn] gate.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT21-005";

fn egg() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled(CARD_ID)
}

fn make_digimon(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(3000);
    card.level = Some(4);
    card.traits = vec!["Appmon".to_string()];
    card
}

/// Simulate linking `link_card_id` onto `host` by:
/// 1. Pushing the card into `host.linked_cards` directly.
/// 2. Firing `OnLink` (TriggerSource::Linked) globally.
/// 3. Draining the effect queue.
///
/// This bypasses `decode_action` (which is turn-player-gated) so we can test
/// the link event independently of whose turn it is — exercising the
/// `active_when: { your_turn: true }` gate cleanly.
fn simulate_link(
    r: &mut DebugRunner,
    host: PermanentHandle,
    link_card_id: &str,
) -> CardHandle {
    let linked_handle = r.push_linked_owned(host, link_card_id, host.player);
    let player_count = r.game.players.len();
    for pid in 0..player_count {
        r.game.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::Linked {
                player: pid as PlayerId,
                host,
                card: linked_handle,
            },
        );
    }
    r.game.drain_effect_queue();
    linked_handle
}

/// Place a stack: egg (BT21-005) as the bottom digivolution source under HOST.
/// Returns the PermanentHandle of the resulting host permanent.
fn host_with_egg(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_stack(player, &[CARD_ID, "HOST"])
}

fn advance_to_main(r: &mut DebugRunner) {
    r.game.enter_main_phase();
}

fn base_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-005 YAML parses and compiles")
        .add_card(make_digimon("HOST"))
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(make_test_card("LINK-CARD", "LinkCard"))
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start()
}

// ─── SECTION 1 — Structural assertions ──────────────────────────────────────

/// BT21-005 compiles with the expected metadata.
#[test]
fn bt21_005_metadata_is_green_lv2_egg_no_dp() {
    let card = egg();
    assert_eq!(card.card, "BT21-005");
    assert_eq!(card.name, "Swipemon");
    assert_eq!(card.level, Some(2));
    assert_eq!(card.dp, None, "a Digi-Egg has no DP");
    assert!(
        card.color
            .contains(&digimon_dsl::compiled::CompiledColor::Green),
        "color must be green"
    );
    assert_eq!(
        card.kind,
        digimon_dsl::compiled::CompiledCardKind::DigiEgg,
        "kind must be digi_egg"
    );
}

/// Exactly one triggered clause (the inherited draw); no main-deck or security
/// effect. The clause must be Inherited scope + WhenCardLinkedToThis timing +
/// once_per_turn + mandatory (no "may").
#[test]
fn bt21_005_has_single_inherited_when_linked_clause_no_other_clauses() {
    let card = egg();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "exactly one triggered clause");

    let clause = triggered[0];
    assert_eq!(
        clause.scope,
        CompiledScope::Inherited,
        "the draw clause must be scope: inherited"
    );
    assert!(
        clause
            .when
            .contains(&CompiledTiming::WhenCardLinkedToThis),
        "must fire on WhenCardLinkedToThis; when={:?}",
        clause.when
    );
    assert!(
        clause.once_per_turn,
        "[Once Per Turn] → once_per_turn must be true"
    );
    assert!(
        !clause.optional,
        "draw is mandatory (no 'may') → optional must be false"
    );
}

/// The clause's process contains a draw step targeting the card's owner.
#[test]
fn bt21_005_process_contains_draw_step() {
    use digimon_dsl::compiled::{CompiledPlayerRef, CompiledStep};
    let card = egg();
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("triggered clause present");
    assert!(
        clause.process.iter().any(|s| matches!(
            s,
            CompiledStep::Draw {
                of: CompiledPlayerRef::You,
                count: 1
            }
        )),
        "process must have draw {{ of: you, count: 1 }}"
    );
}

// ─── SECTION 2 — Behavioral: inherited dispatch via place_stack ─────────────

/// Positive: host carries the egg as a source. A card links onto the host on
/// P0's turn. The inherited draw fires: hand increases by 1, deck decreases by 1.
#[test]
fn bt21_005_draws_when_card_linked_to_host_your_turn() {
    let mut r = base_runner();
    let host = host_with_egg(&mut r, 0);
    advance_to_main(&mut r);

    let hand_before = r.hand_size(0);
    let deck_before = r.game.players[0].deck.len();

    simulate_link(&mut r, host, "LINK-CARD");
    let _ = r.auto_resolve();

    // The inherited draw fires: +1 hand, -1 deck.
    assert_eq!(
        r.hand_size(0),
        hand_before + 1,
        "inherited draw must add 1 card to hand when a card links to the host on your turn"
    );
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before - 1,
        "deck must decrease by 1 from the inherited draw"
    );
}

// ─── SECTION 2b — Negative: opponent's turn gate ────────────────────────────

/// Negative: if it is the opponent's turn when the link fires, the [Your Turn]
/// gate blocks the inherited draw.
#[test]
fn bt21_005_does_not_draw_on_opponents_turn() {
    let mut r = base_runner();
    let host = host_with_egg(&mut r, 0);
    advance_to_main(&mut r);

    // Switch to opponent's turn.
    r.game.turn_player_idx = 1;

    let hand_before = r.hand_size(0);

    simulate_link(&mut r, host, "LINK-CARD");
    let _ = r.auto_resolve();

    // The draw must NOT fire because [Your Turn] gate blocks it on the opponent's turn.
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "[Your Turn] gate must block the draw when it is the opponent's turn"
    );
}

// ─── SECTION 3 — OPT lockout ────────────────────────────────────────────────

/// OPT: a second link event in the same turn does not trigger a second draw.
///
/// Two separate cards link onto the host. First link fires the draw;
/// second link on the same turn must be blocked by the OPT counter.
#[test]
fn bt21_005_opt_blocks_second_link_same_turn() {
    let mut r = base_runner();
    let host = host_with_egg(&mut r, 0);
    advance_to_main(&mut r);

    let hand_before = r.hand_size(0);

    // First link: fires the OPT draw.
    simulate_link(&mut r, host, "LINK-CARD");
    let _ = r.auto_resolve();

    let hand_after_first = r.hand_size(0);
    assert_eq!(
        hand_after_first,
        hand_before + 1,
        "first link must trigger the inherited draw"
    );

    // Second link on the same turn: OPT must block the draw.
    simulate_link(&mut r, host, "FILLER");
    let _ = r.auto_resolve();

    assert_eq!(
        r.hand_size(0),
        hand_after_first,
        "[Once Per Turn] must block a second inherited draw in the same turn"
    );
}

/// OPT clears after end_turn: on P0's next turn the draw fires again.
///
/// We snapshot hand size right AFTER the turn rotation completes (so we
/// account for draw-phase cards drawn during end_turn/start_turn), then
/// check that exactly one MORE card arrives after the second link.
#[test]
fn bt21_005_opt_clears_after_end_turn() {
    let mut r = base_runner();
    let host = host_with_egg(&mut r, 0);
    advance_to_main(&mut r);

    // Turn 1: fire the OPT draw.
    simulate_link(&mut r, host, "LINK-CARD");
    let _ = r.auto_resolve();

    // Advance to P0's next turn (end P0 turn → P1 turn → P0 turn).
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.game.turn_player(), 0, "must be P0's turn again");

    // Snapshot AFTER the rotation so draw-phase cards are accounted for.
    let hand_at_start_of_turn2 = r.hand_size(0);

    // Fire a link on P0's second turn — OPT must have cleared.
    simulate_link(&mut r, host, "FILLER");
    let _ = r.auto_resolve();

    assert_eq!(
        r.hand_size(0),
        hand_at_start_of_turn2 + 1,
        "OPT must clear after end_turn; draw fires again on the next turn"
    );
}

// ─── SECTION 4 — Negative: egg not under a host does nothing ─────────────────

/// When a link fires on a host that does NOT have the egg as a source,
/// no draw is triggered (the inherited effect is host-scoped).
#[test]
fn bt21_005_no_draw_when_egg_not_under_host() {
    let mut r = base_runner();
    // Place HOST without the egg as a source.
    let standalone_host = r.place_on_field(0, "HOST", Some(0));
    advance_to_main(&mut r);

    let hand_before = r.hand_size(0);

    // Link onto the standalone host (no egg sources → inherited draw cannot fire).
    simulate_link(&mut r, standalone_host, "LINK-CARD");
    let _ = r.auto_resolve();

    assert_eq!(
        r.hand_size(0),
        hand_before,
        "no draw when the egg is not a source under the host that got linked"
    );
}
