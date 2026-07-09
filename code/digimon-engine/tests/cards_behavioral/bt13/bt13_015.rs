//! BT13-015 RizeGreymon — Digimon, Lv.5, Red/Yellow, Cost 7, DP 7000.
//! Traits: Cyborg. Attribute: Vaccine.
//!
//! # Card text (BT13-015.json / official Bandai DB bundle)
//!
//! ```text
//! [When Digivolving] You may play 1 [Marcus Damon] from your hand without
//! paying its cost.
//! [All Turns] [Once Per Turn] When one of your red or yellow Tamers is
//! deleted, place 1 [Marcus Damon] from your trash on top of your security
//! stack face down.
//! ```
//!
//! ```text
//! Inherited Effect:
//! [All Turns] [Once Per Turn] When one of your red or yellow Tamers is
//! deleted, place 1 [Marcus Damon] from your trash on top of your security
//! stack face down.
//! ```
//!
//! Standard digivolve (evo_costs): Red Lv.4/cost 4, Yellow Lv.4/cost 4.
//! Alt-digivolve (xros_req): [Digivolve] [GeoGreymon]: Cost 3.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT13/Red/BT13_015.cs
//!
//! DCGO clause-by-clause (differs from the near-identical BT21-044 sibling
//! in two load-bearing ways — see notes inline):
//!   - `timing == None`: `AddSelfDigivolutionRequirementStaticEffect` for the
//!     [GeoGreymon] cost-3 alt-path (`PermanentCondition` checks
//!     `TopCard.CardNames.Contains("GeoGreymon")`).
//!   - `timing == OnEnterFieldAnyone` (When Digivolving): `SelectHandEffect`
//!     with `canNoSelect: true` (optional — "you may") over hand cards named
//!     "Marcus Damon"/"MarcusDamon" that can be played as a new permanent
//!     without paying cost, then `PlayPermanentCards(payCost: false)`. Unlike
//!     BT13-020/BT21-044's sibling clause, there is NO follow-on
//!     TreatAsDigimon/Rush/CannotDigivolve grant — this clause is JUST the
//!     free play.
//!   - `timing == OnDestroyedAnyone` (own + duplicate `SetIsInheritedEffect`
//!     copy): `CanUseCondition` gates ONLY on
//!     `CanTriggerOnPermanentDeleted(PermanentCondition)` where
//!     `PermanentCondition` requires an OWN Tamer colored red or yellow — it
//!     does NOT check trash contents, so the OPT slot (`maxUsableCount: 1`)
//!     is consumed even when no eligible Marcus Damon exists in trash.
//!     `ActivateCoroutine` is gated by
//!     `card.Owner.CanAddSecurity(activateClass)` and
//!     `HasMatchConditionOwnersCardInTrash`; the `SelectCardEffect` itself
//!     has `canNoSelect: () => false` — **the pick, once installed, is
//!     MANDATORY** (unlike BT21-044's "you may place", this card's printed
//!     text has no "may" on this clause and DCGO does not expose a decline).
//!     `AddSecurityCard` places TOP, and DCGO's `AddSecurityCard(selectedCard)`
//!     default is face-up UNLESS explicitly requested face-down — the printed
//!     text says "face down" explicitly, so this clause places FACE DOWN
//!     (BT21-044 places face UP — this is the other load-bearing difference).
//!
//! # DSL YAML
//! code/digimon-engine/cards/bt13/BT13-015.yaml
//!
//! # Patterns this test covers
//! - G-PLAY-FROM-HAND-FREE-BIND-AS: optional free-play of a named Tamer on
//!   [When Digivolving] (BT13-020 idiom, no follow-on grants here)
//! - F5-adjacent: on_any_deletion observer (own red/yellow Tamer) -> MANDATORY
//!   trash -> security place, FACE DOWN
//! - E2/OPT: [All Turns][Once Per Turn] lockout on the deletion observer,
//!   consumed even when the body no-ops (no Marcus Damon in trash)
//! - Inherited copy of the deletion observer (scope: inherited), independent
//!   OPT counter from the own-scope copy
//! - Alt-path: [Digivolve] [GeoGreymon]: Cost 3
//!
//! # Clause map
//! | Clause | Timing | Effect |
//! |--------|--------|--------|
//! | 1 (own) | [When Digivolving] | may free-play 1 Marcus Damon from hand |
//! | 2 (own) | [All Turns][OPT] on deletion | when own red/yellow Tamer deleted, MANDATORILY place a Marcus Damon from trash on top of security, face down |
//! | 3 (inherited) | [All Turns][OPT] on deletion | identical body, inherited scope, own OPT counter |

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledPlayerRef, CompiledPredicate,
    CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::TriggerSource;

use super::super::dsl_card_data::compiled;

// ─── Card-data helpers ────────────────────────────────────────────────────────

/// A [Marcus Damon] Tamer of a chosen color. The card name MUST be exactly
/// "Marcus Damon" so `name_contains: "Marcus Damon"` filters match it.
fn make_marcus(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, "Marcus Damon");
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = vec![];
    c
}

/// A generic Tamer of a chosen color whose name is NOT "Marcus Damon".
fn make_tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = vec![];
    c
}

/// A simple Lv.5 filler Digimon (digivolution base for the alt-path test).
fn make_geogreymon(id: &str) -> CardData {
    let mut c = make_test_card(id, "GeoGreymon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c.traits = vec![];
    c
}

/// Standard runner: RizeGreymon in P0 hand, Marcus Damon copies + fillers
/// registered, generous memory.
fn rizegreymon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT13-015")
        .expect("BT13-015 in embedded DSL pack")
        .add_card(make_marcus("MARCUS-Y", CardColor::Yellow))
        .add_card(make_marcus("MARCUS-R", CardColor::Red))
        .add_card(make_tamer("TAMER-G", CardColor::Green))
        .add_card(make_tamer("TAMER-R", CardColor::Red))
        .add_card(make_geogreymon("GEO"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["BT13-015"])
        .deck(0, &["FILLER", "FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER"])
        .memory(10)
        .start()
}

/// Push a [Marcus Damon] card directly into a player's trash.
fn seed_trash_with_marcus(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("marcus card registered");
    let next_idx = runner.game.next_card_index();
    let source = digimon_engine::card_source::CardSource::new(data_idx, player, next_idx);
    runner.game.players[player as usize].trash.push(source);
}

/// Push a [Marcus Damon] card directly into a player's hand.
fn add_marcus_to_hand(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("marcus card registered");
    let next_idx = runner.game.next_card_index();
    let source = digimon_engine::card_source::CardSource::new(data_idx, player, next_idx);
    runner.game.players[player as usize].hand.push(source);
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn bt13_015_compiles_from_dsl_pack() {
    let _ = compiled("BT13-015");
}

#[test]
fn bt13_015_is_red_yellow_lv5_cyborg_digimon() {
    let card = compiled("BT13-015");
    assert_eq!(card.kind, digimon_dsl::compiled::CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Red));
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Yellow));
    assert!(card.traits.iter().any(|t| t == "Cyborg"));
}

/// Alt-path: [Digivolve] [GeoGreymon]: Cost 3.
#[test]
fn bt13_015_has_geogreymon_alt_path_cost3() {
    let card = compiled("BT13-015");
    let geo = card
        .alt_paths
        .iter()
        .find(|p| {
            p.kind == CompiledAltPathKind::Digivolve && p.cost == Some(CompiledCost::Literal(3))
        })
        .expect("must have a GeoGreymon cost-3 digivolve alt-path");
    assert_eq!(geo.cost, Some(CompiledCost::Literal(3)));
}

/// Clause 1 fires on [When Digivolving] only (unlike BT21-044/BT13-020 this
/// card does NOT also fire on [On Play]).
#[test]
fn bt13_015_clause1_fires_on_when_digivolving_only() {
    let card = compiled("BT13-015");
    let clause1 = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.scope == CompiledScope::FaceUp =>
            {
                Some(t)
            }
            _ => None,
        })
        .next()
        .expect("clause 1 (own When Digivolving) must exist");
    assert!(clause1.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(
        !clause1.when.contains(&CompiledTiming::OnPlay),
        "printed text is [When Digivolving] only, no [On Play]"
    );
    // Note: "you may play" is surfaced by the inner `select_hand`'s own
    // `optional: true` (asserted behaviorally below via
    // `pending.is_optional` on the installed selection) — the top-level
    // `CompiledTriggeredClause::optional` flag is a separate mechanism (the
    // BT21-044-style "the whole activation is a mandatory-once-triggered,
    // optional-body" shape) that this clause does not use, matching the
    // BT13-020 sibling's identical select_hand(optional)+play_from_hand_free
    // idiom.
}

/// The card ships exactly two OnAnyDeletion observers: one own (FaceUp) and one
/// inherited — matching the two DCGO ActivateClass shells (main + inherited).
#[test]
fn bt13_015_has_two_on_any_deletion_observers_one_inherited() {
    let card = compiled("BT13-015");
    let deletion: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        deletion.len(),
        2,
        "expected 2 OnAnyDeletion observers (own + inherited), got {}",
        deletion.len()
    );
    assert_eq!(
        deletion
            .iter()
            .filter(|t| t.scope == CompiledScope::Inherited)
            .count(),
        1,
        "exactly one of the two deletion observers is inherited"
    );
    assert_eq!(
        deletion
            .iter()
            .filter(|t| t.scope == CompiledScope::FaceUp)
            .count(),
        1,
        "exactly one of the two deletion observers is own (FaceUp)"
    );
}

/// Both deletion observers are [Once Per Turn] and MANDATORY (no "may" in the
/// printed text on this clause — unlike BT21-044's optional sibling).
#[test]
fn bt13_015_deletion_observers_are_opt_and_mandatory() {
    let card = compiled("BT13-015");
    let deletion: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(deletion.len(), 2);
    for t in &deletion {
        assert!(t.once_per_turn, "deletion observer must be [Once Per Turn]");
        assert!(
            !t.optional,
            "printed text has no 'may' on this clause -> mandatory once it installs"
        );
    }
}

/// The deletion observers gate on the deleted object: own red/yellow Tamer.
fn predicate_mentions_event_target_kind(p: &CompiledPredicate) -> bool {
    p.event_target_kind.is_some()
        || p.all_of.iter().any(predicate_mentions_event_target_kind)
        || p.any_of.iter().any(predicate_mentions_event_target_kind)
}

fn predicate_mentions_event_target_color(p: &CompiledPredicate) -> bool {
    p.event_target_color_any_of.is_some()
        || p.all_of.iter().any(predicate_mentions_event_target_color)
        || p.any_of.iter().any(predicate_mentions_event_target_color)
}

fn predicate_mentions_event_target_owner_you(p: &CompiledPredicate) -> bool {
    p.event_target_owner == Some(CompiledPlayerRef::You)
        || p.all_of
            .iter()
            .any(predicate_mentions_event_target_owner_you)
        || p.any_of
            .iter()
            .any(predicate_mentions_event_target_owner_you)
}

#[test]
fn bt13_015_deletion_observers_gate_on_own_red_yellow_tamer() {
    let card = compiled("BT13-015");
    let deletion: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(deletion.len(), 2);
    for t in &deletion {
        let cond = t
            .condition
            .as_ref()
            .expect("deletion observer must gate on the deleted object");
        assert!(
            predicate_mentions_event_target_owner_you(cond),
            "must require the deleted Tamer to be yours"
        );
        assert!(
            predicate_mentions_event_target_kind(cond),
            "must require the deleted object be a Tamer"
        );
        assert!(
            predicate_mentions_event_target_color(cond),
            "must require the deleted Tamer be red or yellow"
        );
    }
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1 [When Digivolving] behavioral
// ════════════════════════════════════════════════════════════════════════════

/// Helper: place RizeGreymon atop a GeoGreymon stack and fire WhenDigivolving.
fn digivolve_rize(runner: &mut DebugRunner) -> PermanentHandle {
    let rize = runner.place_stack(0, &["GEO", "BT13-015"]);
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(rize),
    );
    runner.game.drain_effect_queue();
    rize
}

/// POSITIVE: with a Marcus Damon in hand, digivolving into RizeGreymon installs
/// an optional selection to free-play it.
#[test]
fn bt13_015_when_digivolving_installs_optional_marcus_select_when_in_hand() {
    let mut runner = rizegreymon_runner();
    add_marcus_to_hand(&mut runner, 0, "MARCUS-Y");

    digivolve_rize(&mut runner);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("a selection must install: you may play the Marcus Damon from hand");
    assert_eq!(pending.selecting_player, 0);
    assert!(pending.is_optional, "'you may play' -> PASS must be legal");
}

/// POSITIVE: accepting the selection plays the Marcus Damon from hand without
/// paying its cost (memory is unaffected).
#[test]
fn bt13_015_when_digivolving_accept_plays_marcus_free() {
    let mut runner = rizegreymon_runner();
    add_marcus_to_hand(&mut runner, 0, "MARCUS-Y");

    let hand_before = runner.game.players[0].hand.len();

    digivolve_rize(&mut runner);

    // Capture AFTER RizeGreymon's own stack placement (digivolve_rize places
    // the RizeGreymon/GeoGreymon stack as its own battle-area entry) but
    // BEFORE resolving the Marcus Damon pick.
    let memory_before = runner.game.memory;
    let battle_before = runner.game.players[0].battle_area.len();

    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, action)
        .expect("Marcus selection resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "Marcus Damon leaves hand"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "Marcus Damon enters the battle area"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "played without paying its cost -> memory unaffected"
    );
}

/// NEGATIVE (decline): PASS leaves hand and battle area unchanged.
#[test]
fn bt13_015_when_digivolving_decline_plays_nothing() {
    let mut runner = rizegreymon_runner();
    add_marcus_to_hand(&mut runner, 0, "MARCUS-Y");

    let hand_before = runner.game.players[0].hand.len();

    digivolve_rize(&mut runner);

    // Capture AFTER RizeGreymon's own stack placement, BEFORE resolving the
    // decline, so the assertion below isolates the decline's effect only.
    let battle_before = runner.game.players[0].battle_area.len();

    runner
        .game
        .resolve_selection(0, PASS)
        .expect("decline resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "declining must not play anything from hand"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before,
        "declining must not add a permanent"
    );
}

/// NEGATIVE: with NO Marcus Damon in hand, no selection installs at all.
#[test]
fn bt13_015_when_digivolving_no_marcus_in_hand_installs_nothing() {
    let mut runner = rizegreymon_runner();
    digivolve_rize(&mut runner);
    assert!(
        runner.game.pending_selection.is_none(),
        "no Marcus Damon in hand -> no selection installs"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 2/3 [All Turns][OPT] deletion -> trash -> security
//              (MANDATORY placement, FACE DOWN)
// ════════════════════════════════════════════════════════════════════════════

/// Setup: RizeGreymon on the field (own clause active), a yellow Marcus Damon
/// in trash, and a red Marcus Damon Tamer on the field to be deleted.
fn deletion_runner_with_rize_on_field() -> (DebugRunner, PermanentHandle) {
    let mut runner = rizegreymon_runner();
    // RizeGreymon on the battle area so its own [All Turns] observer is active.
    let _rize = runner.place_on_field(0, "BT13-015", Some(0));
    // A Marcus Damon sitting in the trash to be placed on security.
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-Y");
    // The deletable red Marcus Damon Tamer on the field.
    let tamer = runner.place_on_field(0, "MARCUS-R", Some(0));
    (runner, tamer)
}

/// POSITIVE: deleting an own red/yellow Tamer installs the MANDATORY
/// trash-place selection; resolving it places the Marcus Damon from trash
/// FACE DOWN onto the top of security. Net: security +1.
#[test]
fn bt13_015_deletion_places_marcus_from_trash_to_top_security_face_down() {
    let (mut runner, tamer) = deletion_runner_with_rize_on_field();

    let sec_before = runner.security_count(0);

    runner.game.delete_permanent_with_effects(tamer);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("deletion of a red/yellow Tamer installs the trash-place selection");
    assert_eq!(pending.selecting_player, 0);
    assert!(
        !pending.is_optional,
        "the printed text has no 'may' on this clause -> mandatory pick, PASS illegal"
    );
    let action = pending.valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, action)
        .expect("trash selection resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.security_count(0),
        sec_before + 1,
        "a Marcus Damon must be placed as the top security card"
    );

    // `security` is a Vec where the TOP card is the LAST element (mirrors
    // `StackPosition::Top => security.push(..)` / `security.pop()` on
    // security-check — see `game_actions/mod.rs::place_on_security_observed`
    // and `combat/mod.rs`'s security-pop).
    let top = runner.game.players[0]
        .security
        .last()
        .expect("security must be non-empty after the place");
    let name = runner.game.card_data[top.data_index].card_name.clone();
    assert_eq!(
        name, "Marcus Damon",
        "top security card must be the Marcus Damon"
    );
    // Face-up-ness is tracked via `Player.face_up_security: HashSet<u16>`
    // keyed by `CardSource.card_index`, not a field on `CardSource` itself.
    assert!(
        !runner.game.players[0]
            .face_up_security
            .contains(&top.card_index),
        "the placed security card must be FACE DOWN, not face up"
    );
}

/// The placed security card is explicitly face down (direct field check on
/// the `face_up_security` membership set — belt and suspenders on the
/// load-bearing faithfulness distinction vs BT21-044's face-up sibling).
#[test]
fn bt13_015_placed_security_card_is_face_down() {
    let (mut runner, tamer) = deletion_runner_with_rize_on_field();
    runner.game.delete_permanent_with_effects(tamer);
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner.game.resolve_selection(0, action).expect("resolves");
    let _ = runner.auto_resolve();

    let top = runner.game.players[0]
        .security
        .last()
        .expect("security must be non-empty after the place");
    assert!(
        !runner.game.players[0]
            .face_up_security
            .contains(&top.card_index),
        "RizeGreymon's on-deletion place is explicitly 'face down' in the printed text"
    );
}

/// NEGATIVE: deleting a Tamer of a non-matching color (green) must NOT fire
/// the observer.
#[test]
fn bt13_015_deletion_of_green_tamer_does_not_fire() {
    let mut runner = rizegreymon_runner();
    let _rize = runner.place_on_field(0, "BT13-015", Some(0));
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-Y");
    let green_tamer = runner.place_on_field(0, "TAMER-G", Some(0));

    let sec_before = runner.security_count(0);
    runner.game.delete_permanent_with_effects(green_tamer);

    assert!(
        runner.game.pending_selection.is_none(),
        "a green Tamer is neither red nor yellow -> observer must not fire"
    );
    assert_eq!(runner.security_count(0), sec_before);
}

/// NEGATIVE: deleting an OPPONENT'S red/yellow Tamer must NOT fire your
/// observer.
#[test]
fn bt13_015_deletion_of_opponent_tamer_does_not_fire() {
    let mut runner = rizegreymon_runner();
    let _rize = runner.place_on_field(0, "BT13-015", Some(0));
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-Y");
    let opp_tamer = runner.place_on_field(1, "MARCUS-Y", Some(0));

    let sec_before = runner.security_count(0);
    runner.game.delete_permanent_with_effects(opp_tamer);

    assert!(
        runner.game.pending_selection.is_none(),
        "opponent's Tamer deletion must not fire your observer"
    );
    assert_eq!(runner.security_count(0), sec_before);
}

/// NEGATIVE: with NO Marcus Damon in trash, the observer's OPT slot is still
/// consumed (DCGO `CanUseCondition` does not check trash — only
/// `ActivateCoroutine`'s body does), but no selection installs and security
/// is unchanged. This is the DCGO-verified divergence from a naive
/// "gate the whole clause on trash contents" reading.
///
/// Uses a non-Marcus-Damon red Tamer (`TAMER-R`) for the eligible deletion —
/// deleting an actual Marcus Damon Tamer would immediately put a matching
/// card into trash (itself), which is a legitimately different scenario
/// (covered by `bt13_015_deletion_places_marcus_from_trash_to_top_security_face_down`).
#[test]
fn bt13_015_deletion_with_empty_trash_consumes_opt_but_installs_nothing() {
    let mut runner = rizegreymon_runner();
    let _rize = runner.place_on_field(0, "BT13-015", Some(0));
    // No Marcus Damon seeded into trash yet — trash is genuinely empty of
    // matches for the first deletion below.
    let tamer1 = runner.place_on_field(0, "TAMER-R", Some(0));
    let tamer2 = runner.place_on_field(0, "MARCUS-Y", Some(0));

    let sec_before = runner.security_count(0);
    runner.game.delete_permanent_with_effects(tamer1);

    assert!(
        runner.game.pending_selection.is_none(),
        "no Marcus Damon in trash -> the body no-ops, no selection installs"
    );
    assert_eq!(runner.security_count(0), sec_before);

    // OPT was consumed by the first (no-op) deletion: seed a Marcus Damon
    // into trash now (available for a second eligible deletion this turn)
    // and confirm the OPT lockout still blocks it despite the earlier
    // activation having done nothing observable.
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-Y");
    runner.game.delete_permanent_with_effects(tamer2);
    assert!(
        runner.game.pending_selection.is_none(),
        "OPT must already be consumed by the first (no-op) deletion this turn"
    );
    assert_eq!(runner.security_count(0), sec_before);
}

/// OPT: a second eligible Tamer deletion in the same turn must NOT re-fire.
#[test]
fn bt13_015_deletion_observer_is_once_per_turn() {
    let mut runner = rizegreymon_runner();
    let _rize = runner.place_on_field(0, "BT13-015", Some(0));
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-Y");
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-R");
    let tamer1 = runner.place_on_field(0, "MARCUS-Y", Some(0));
    let tamer2 = runner.place_on_field(0, "MARCUS-R", Some(0));

    // First deletion fires; resolve the mandatory placement.
    runner.game.delete_permanent_with_effects(tamer1);
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, action)
        .expect("first resolves");
    let _ = runner.auto_resolve();
    let sec_after_first = runner.security_count(0);

    // Second deletion same turn must NOT install a selection (OPT lockout).
    runner.game.delete_permanent_with_effects(tamer2);
    assert!(
        runner.game.pending_selection.is_none(),
        "OPT must block the second eligible Tamer deletion in the same turn"
    );
    assert_eq!(
        runner.security_count(0),
        sec_after_first,
        "no second placement under the OPT lockout"
    );
}

/// The OPT lockout clears next turn: after end_turn cycles back to P0, the
/// observer fires again on a fresh eligible deletion.
#[test]
fn bt13_015_deletion_opt_clears_next_turn() {
    let mut runner = rizegreymon_runner();
    let _rize = runner.place_on_field(0, "BT13-015", Some(0));
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-Y");
    seed_trash_with_marcus(&mut runner, 0, "MARCUS-R");
    let tamer1 = runner.place_on_field(0, "MARCUS-Y", Some(0));

    runner.game.delete_permanent_with_effects(tamer1);
    let action = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .valid_action_ids[0];
    runner
        .game
        .resolve_selection(0, action)
        .expect("first resolves");
    let _ = runner.auto_resolve();

    // Cycle back to P0's turn.
    runner.game.end_turn();
    runner.game.end_turn();

    let tamer2 = runner.place_on_field(0, "MARCUS-R", Some(0));
    runner.game.delete_permanent_with_effects(tamer2);
    assert!(
        runner.game.pending_selection.is_some(),
        "OPT lockout must clear next turn — observer fires again"
    );
}

/// The own-scope and inherited-scope copies of the deletion observer track
/// INDEPENDENT OPT counters (each has its own hash string in DCGO:
/// "AddSecurity_BT13_015" vs "AddSecurity_BT13_015_inherited"). This test
/// mirrors the pre-flight brief's explicit callout: "main and inherited
/// copies each have their OWN OPT counter." We cannot directly force only
/// the inherited copy to fire without a second card granting it, so this
/// test instead pins the observable consequence on the OWN-scope path (OPT
/// count of 1 within the clause) via the two tests above, and structurally
/// pins the two clauses as materially distinct compiled entries so a future
/// regression collapsing them into a single shared-hash clause is caught by
/// `bt13_015_has_two_on_any_deletion_observers_one_inherited`.
#[test]
fn bt13_015_own_and_inherited_deletion_clauses_are_structurally_distinct() {
    let card = compiled("BT13-015");
    let deletion: Vec<&CompiledTriggeredClause> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyDeletion) => {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(deletion.len(), 2);
    let scopes: Vec<CompiledScope> = deletion.iter().map(|t| t.scope).collect();
    assert!(scopes.contains(&CompiledScope::FaceUp));
    assert!(scopes.contains(&CompiledScope::Inherited));
    assert_ne!(
        scopes[0], scopes[1],
        "own and inherited deletion clauses must be distinct compiled entries (separate OPT identity)"
    );
}
