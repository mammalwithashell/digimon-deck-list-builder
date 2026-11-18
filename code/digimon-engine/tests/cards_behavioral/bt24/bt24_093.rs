//! BT24-093 Temple of Beginnings — Option, Yellow, Cost 2. Traits: Iliad/TS.
//!
//! # Printed text (data/card_bundles/BT24-093.md — official Bandai DB, verbatim)
//!
//! [Main] Add your top security card to the hand and ＜Recovery +1 (Deck)＞
//!   (Place the top card of your deck on top of your security stack). Then,
//!   place this card in the battle area.
//! [All Turns] When your security stack is removed from, ＜Delay＞ (By
//!   trashing this card after the placing turn, activate the effect below.)
//!   ・You may place the top stacked card of any of your Digimon with
//!   [Aegiochusmon] or [Jupitermon] in their names as the top security card.
//!
//! Security Effect:
//! [Security] You may play 1 [Aegiomon] or [Elecmon] from your hand or trash
//!   without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT24/Yellow/BT24_093.cs (base repo)
//!   - EffectTiming.OptionSkill ([Main]): if security non-empty, add top
//!     security to hand + IReduceSecurity; unconditionally IRecovery(1); then
//!     PlaceDelayOptionCards (places self in battle area as a Delay Option).
//!   - EffectTiming.OnLoseSecurity ([All Turns]): CanUseCondition requires
//!     CanTriggerWhenLoseSecurity + CanDeclareOptionDelayEffect (this is the
//!     `<Delay>` gate — placed Option, past its placing turn, triggered by the
//!     controller's OWN security stack being removed from). Body:
//!     DeletePermanentAndProcessAccordingToResult on THIS Option (the Delay's
//!     self-trash cost), then on success: if any own [Aegiochusmon]/
//!     [Jupitermon]-named permanent has >=1 DigivolutionCards, offer an
//!     optional (canNoSelect: true) SelectPermanentEffect over those
//!     candidates; on pick, RemoveDigivolveRootEffect (cosmetic) then
//!     `AddSecurityCard(permanent.TopCard)` — this extracts the permanent's
//!     CURRENT TOP CARD (de-digivolving it in place, leaving the rest of its
//!     stack on the battlefield) and places it face-down on TOP of the
//!     controller's security stack.
//!   - EffectTiming.SecuritySkill (inherited Security Effect): optional
//!     hand-vs-trash zone choice (only prompted when both zones have an
//!     eligible card), then select 1 Aegiomon/Elecmon-named card from that
//!     zone and play it for free (PlayPermanentCards payCost:false).
//!
//! # Patterns (RUST_DSL_TEST_API.md §4.3)
//! - Group C/G-ish: Option self-placement as a `<Delay>` battle-area permanent
//!   (`place_self_as_delay_option`), mirrors BT24-089 / BT24-100 / ST20-15.
//! - B1-adjacent: [Main] unconditional add-top-security-to-hand + Recovery
//!   (BT25-036 Craftmon idiom: `add_top_security_to_hand` + `recover`).
//! - Group E2: inherited [Security] optional hand-OR-trash free play, filtered
//!   by name (BT24-089 `select_union_zone` + `play_union_bound_free` idiom).
//! - All-Turns event-gated `<Delay>` triggered by the controller's OWN
//!   security stack being removed from (`kind: delay`, `trigger:
//!   on_own_security_removed` -> `DelayTrigger::OnEvent(OnOwnSecurityRemoved)`),
//!   optional outer prompt, trash-self cost, declinable host pick, then
//!   `security_place_top_stacked_card` (hunt cluster `delay-trigger-substrate`;
//!   the two gaps an earlier draft of this file recorded -- the event-delay
//!   fan-out skipping `TriggerSource::SecurityRemoved`, and a "pop the top
//!   stacked card to security" primitive -- are closed).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::{OptionPlayResult, TriggerSource};

const CARD_ID: &str = "BT24-093";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// An [Aegiomon]-named Lv.3 filler Digimon — legal Security-effect pick.
fn aegiomon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Aegiomon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

/// An [Elecmon]-named Lv.3 filler Digimon — legal Security-effect pick.
fn elecmon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Elecmon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 2;
    c
}

/// A NON-Aegiomon/Elecmon Digimon — illegal pick for the Security-effect
/// name filter.
fn other_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, "Some Other Digimon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 2;
    c
}

fn filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 3;
    c
}

/// A Yellow board Digimon — satisfies BT24-093's color-match-available gate
/// (`option_color_match_available`: an Option needs a same-colored
/// Digimon/Tamer already on the controller's field to be legally playable).
fn yellow_board(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

fn push_to_hand(runner: &mut DebugRunner, player: PlayerId, card_id: &str) -> usize {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .hand
        .push(CardSource::new(data_index, player, instance_id));
    runner.game.players[player as usize].hand.len() - 1
}

fn push_to_trash(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_index, player, instance_id));
}

fn base() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT24-093 YAML parses and compiles")
        .add_card(aegiomon("AEGIOMON"))
        .add_card(elecmon("ELECMON"))
        .add_card(other_digimon("OTHER"))
        .add_card(filler("FILL"))
        .add_card(yellow_board("YELLOW-BOARD"))
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(0, &["FILL", "FILL"])
        .memory(10)
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt24_093_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT24-093 must be in the embedded DSL pack");

    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.color, vec![CompiledColor::Yellow]);
    assert_eq!(card.cost, Some(2));
    for trait_name in ["Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT24-093 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt24_093_has_main_clause_that_adds_top_security_recovers_and_places_self() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let main = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("[Main] clause present");

    assert!(
        !main.optional,
        "the [Main] clause itself is mandatory (always add-security+recover+place)"
    );
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::AddTopSecurityToHand { .. })),
        "[Main] must add top security card to hand"
    );
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::Recover { .. })),
        "[Main] must <Recovery +1>"
    );
    assert!(
        main.process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlaceSelfAsDelayOption)),
        "[Main] must place this card in the battle area as a Delay Option"
    );
}

#[test]
fn bt24_093_has_inherited_security_clause_with_union_zone_free_play() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let security = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnSecurity)
        })
        .expect("inherited [Security] clause present");

    assert!(
        security
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::SelectUnionZone { .. })),
        "[Security] effect must offer a hand-OR-trash union pick"
    );
    assert!(
        security
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayUnionBoundFree { .. })),
        "[Security] effect must play the picked card for free"
    );
}

// ─── Section 2 — [Main]: add top security to hand ────────────────────────────

#[test]
fn bt24_093_main_adds_top_security_card_to_hand_when_security_nonempty() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["FILL", "FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW-BOARD", Some(0));
    let top_before = runner.game.players[0]
        .security
        .last()
        .expect("security has a top card")
        .handle();
    let hand_before = runner.hand_size(0);
    let security_before = runner.security_count(0);

    // NOTE: `play_option_core`'s post-dispose return path hardcodes
    // `OptionPlayResult::Trashed` regardless of the Option's actual disposal
    // subtype (see `game_actions/options.rs` step 8 — `dispose_option()`
    // returns `()`, so the literal `Trashed` does not reflect a Delay-Option
    // placement). Assert on board/zone STATE, not the return-value literal
    // (see `bt24_093_main_places_self_in_battle_area_as_delay_option` for the
    // dedicated placement assertion).
    let result = runner.game.play_option_from_hand(0, 0);
    if matches!(result, OptionPlayResult::Pending) {
        runner
            .auto_resolve()
            .expect("Main body has no real player choices");
    }

    assert_eq!(
        runner.hand_size(0),
        hand_before, // hand loses BT24-093 (played) but gains the top security card
        "hand size unchanged: -1 (BT24-093 played) +1 (top security added)"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.handle() == top_before),
        "the pre-play top security card must now be in hand"
    );
    // Security lost 1 (moved to hand) but gained 1 back from <Recovery +1> (deck
    // top placed on top of security) — net security count is unchanged.
    assert_eq!(
        runner.security_count(0),
        security_before,
        "security count is net-unchanged: -1 (to hand) +1 (Recovery)"
    );
}

#[test]
fn bt24_093_main_recovers_even_when_security_is_empty() {
    // DCGO gates the add-to-hand step on `SecurityCards.Count > 0` but runs
    // <Recovery +1> unconditionally afterward (outside that `if`).
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &[])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW-BOARD", Some(0));
    let deck_top_before = runner.game.players[0]
        .deck
        .last()
        .expect("deck has a top card")
        .handle();

    // See the state-vs-return-value note in the sibling test above.
    let result = runner.game.play_option_from_hand(0, 0);
    if matches!(result, OptionPlayResult::Pending) {
        runner.auto_resolve().expect("no real choices in Main body");
    }

    assert_eq!(
        runner.security_count(0),
        1,
        "Recovery +1 places the deck top onto security even though there was nothing to move to hand"
    );
    assert!(
        runner.game.players[0]
            .security
            .last()
            .is_some_and(|c| c.handle() == deck_top_before),
        "the recovered security card must be the pre-play deck top"
    );
}

// ─── Section 3 — [Main]: places self as a Delay Option permanent ─────────────

#[test]
fn bt24_093_main_places_self_in_battle_area_as_delay_option() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["FILL"])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW-BOARD", Some(0));

    let result = runner.game.play_option_from_hand(0, 0);
    if matches!(result, OptionPlayResult::Pending) {
        runner
            .auto_resolve()
            .expect("Main body has no real player choices");
    }

    let placed = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("BT24-093 must be placed on the battle area");
    assert!(
        matches!(placed.option_state, OptionState::Delayed { .. }),
        "BT24-093 must be placed with Delayed option_state, got {:?}",
        placed.option_state
    );
    assert!(
        !runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == CARD_ID),
        "BT24-093 must leave the hand once played"
    );
}

// ─── Section 4 — inherited [Security]: optional free play of Aegiomon/Elecmon ─
//
// A card's printed [Security] box is a genuinely different lifecycle moment
// from being played onto the battle area — it fires ONLY while the card is
// revealed FROM a player's own security stack during a security check
// (`EffectTiming::SecuritySkill` dispatched via `TriggerSource::
// SecurityRevealed`, gated on `Game::pending_security` — see
// `effect_queue.rs::enqueue_from_security_card`). This is unrelated to the
// `effect.inherited`-scoped digivolution-stack inheritance gate (RULES
// 15-3-1) that `enqueue_from_permanent`'s top-card scan skips — that gate
// governs a DIFFERENT `scope: inherited` use-case (a card's inherited body
// while it sits BENEATH another card in a stack), not this card's printed
// Security Effect. The faithful way to drive it in a test is a REAL attack
// on the card's owner via `attack_player`, so the production
// `security_resolution` state machine (`combat/mod.rs`) populates
// `pending_security` and reveals BT24-093 exactly as it would in a real game.

/// A Yellow attacker for player 1 to declare a security attack against
/// player 0 (whose security stack holds BT24-093).
fn attacker_for_p1(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Blue];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

#[test]
fn bt24_093_security_effect_offers_free_play_from_hand() {
    let mut runner = base()
        .add_card(attacker_for_p1("ATTACKER"))
        .security(0, &[CARD_ID])
        .start();
    let _idx = push_to_hand(&mut runner, 0, "AEGIOMON");
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));

    runner.attack_player(attacker, 0, false);

    assert!(
        runner.pending_selection().is_some(),
        "[Security] effect must offer the free-play selection when a legal hand target exists"
    );
    assert!(
        runner.pending_is_optional(),
        "[Security] effect is 'you may' — PASS must be legal"
    );
    runner.auto_resolve().ok();
}

#[test]
fn bt24_093_security_effect_no_prompt_without_legal_hand_or_trash_target() {
    let mut runner = base()
        .add_card(attacker_for_p1("ATTACKER"))
        .security(0, &[CARD_ID])
        .start();
    let _idx = push_to_hand(&mut runner, 0, "OTHER");
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));

    runner.attack_player(attacker, 0, false);
    let hand_before_pending = runner.pending_selection().is_some();
    runner.auto_resolve().ok();

    assert!(
        !hand_before_pending,
        "with no [Aegiomon]/[Elecmon] card in hand or trash, no free-play selection should install"
    );
}

#[test]
fn bt24_093_security_effect_declining_leaves_hand_and_trash_untouched() {
    let mut runner = base()
        .add_card(attacker_for_p1("ATTACKER"))
        .security(0, &[CARD_ID])
        .start();
    let _idx = push_to_hand(&mut runner, 0, "ELECMON");
    let hand_before = runner.hand_size(0);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    let field_before = runner.battle_area_size(0);

    runner.attack_player(attacker, 0, false);

    assert!(runner.pending_selection().is_some(), "prompt must install");
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the free play");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining leaves hand untouched"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "declining plays nothing new onto the battle area"
    );
}

#[test]
fn bt24_093_security_effect_picking_hand_plays_aegiomon_free() {
    let mut runner = base()
        .add_card(attacker_for_p1("ATTACKER"))
        .security(0, &[CARD_ID])
        .start();
    let _idx = push_to_hand(&mut runner, 0, "AEGIOMON");
    let memory_before = runner.memory();
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));

    runner.attack_player(attacker, 0, false);

    let view = runner
        .pending_selection_view()
        .expect("free-play selection must be pending");
    // Drive the first legal (non-PASS) action explicitly — asserting a
    // specific branch, not `auto_resolve` blind.
    let action = view
        .valid_action_ids
        .first()
        .copied()
        .expect("at least one legal pick exists");
    runner.execute_action(0, action).expect("pick the card");
    runner.auto_resolve().expect("finish the free play");

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "AEGIOMON"),
        "Aegiomon must be played onto the battle area"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "playing 'without paying the cost' must not change memory"
    );
}

#[test]
fn bt24_093_security_effect_picking_trash_plays_elecmon_free() {
    let mut runner = base()
        .add_card(attacker_for_p1("ATTACKER"))
        .security(0, &[CARD_ID])
        .start();
    push_to_trash(&mut runner, 0, "ELECMON");
    let trash_before = runner.trash_size(0);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));

    runner.attack_player(attacker, 0, false);

    let view = runner
        .pending_selection_view()
        .expect("free-play selection must be pending (trash-only candidate)");
    let action = view
        .valid_action_ids
        .first()
        .copied()
        .expect("at least one legal pick exists");
    runner
        .execute_action(0, action)
        .expect("pick the trash card");
    runner.auto_resolve().expect("finish the free play");

    assert!(
        runner
            .game
            .player(0)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ELECMON"),
        "Elecmon must be played onto the battle area from trash"
    );
    // NOTE: aggregate trash SIZE is unchanged (Elecmon leaves, but BT24-093
    // itself lands in trash as the normal post-reveal security-check
    // disposition of a non-Delay-placed card) — assert on ELECMON
    // specifically, not the trash count.
    let _ = trash_before;
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "ELECMON"),
        "Elecmon must leave the trash once played"
    );
}

// ─── Section 5 — [All Turns] <Delay> event-gated on own-security-removed ─────
//
// DCGO `BT24_093.cs` Block 2 (`EffectTiming.OnLoseSecurity`):
// `CanTriggerWhenLoseSecurity(player == card.Owner)` +
// `CanDeclareOptionDelayEffect` (past the placing turn), optional
// (`SetUpActivateClass(..., isOptional: true, ...)`). CR 4.3 16-17-1/-2/-3.

const DELAY_HOST_TOP: &str = "AEGIOCHUSMON-TOP";
const DELAY_HOST_BASE: &str = "AEGIOMON-BASE";

fn aegiochusmon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 6;
    c
}

/// Seat BT24-093 as a placed `<Delay>` Option that was placed on
/// `placed_on_turn` (the printed "after the placing turn" gate compares it to
/// the current turn).
fn seat_delay(runner: &mut DebugRunner, player: PlayerId, placed_on_turn: u16) {
    let handle = runner.place_on_field(player, CARD_ID, Some(0));
    runner.game.players[player as usize].battle_area[handle.index as usize].option_state =
        OptionState::Delayed {
            owner: player,
            trash_on_turn: u16::MAX,
            trigger: digimon_engine::enums::DelayTrigger::OnEvent(
                EffectTiming::OnOwnSecurityRemoved,
            ),
            placed_on_turn,
        };
}

fn delay_runner() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = base()
        .add_card(attacker_for_p1("ATTACKER"))
        .add_card(aegiochusmon(DELAY_HOST_TOP, "Aegiochusmon: Holy"))
        .add_card(aegiochusmon(DELAY_HOST_BASE, "Aegiomon"))
        .security(0, &["FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .start();
    seat_delay(&mut runner, 0, 0);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    (runner, attacker)
}

fn option_on_field(runner: &DebugRunner, player: usize) -> bool {
    runner.game.players[player].battle_area.iter().any(|p| {
        p.top_card().card_id(&runner.game.card_data) == CARD_ID
            && matches!(p.option_state, OptionState::Delayed { .. })
    })
}

fn accept_branch(runner: &DebugRunner) -> u16 {
    runner
        .pending_selection_view()
        .expect("a pending selection")
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != digimon_engine::action::space::PASS)
        .expect("a non-pass branch")
}

/// A security check on the Option's controller opens the optional `<Delay>`
/// window; accepting trashes the Option, then the declinable pick moves the
/// chosen Digimon's TOP card (not a lower material) face-down to the TOP of
/// security and leaves the rest of that Digimon's stack in play.
#[test]
fn bt24_093_delay_on_own_security_removed_places_top_stacked_card_as_top_security() {
    let (mut runner, attacker) = delay_runner();
    runner.place_stack(0, &[DELAY_HOST_BASE, DELAY_HOST_TOP]);
    assert!(option_on_field(&runner, 0));

    runner.attack_player(attacker, 0, false);

    let outer = runner
        .pending_selection_view()
        .expect("the security removal opens the optional <Delay> window");
    assert_eq!(outer.selecting_player, 0, "the Option's controller decides");
    assert!(runner.pending_is_optional(), "16-17-2: <Delay> is optional");
    let accept = accept_branch(&runner);
    runner
        .execute_action(outer.selecting_player, accept)
        .expect("accept the <Delay>");
    assert!(
        !option_on_field(&runner, 0),
        "accepting trashes the Option as the activation cost (16-17-1)"
    );

    let pick = runner
        .pending_selection_view()
        .expect("the declinable [Aegiochusmon]/[Jupitermon] pick");
    assert!(runner.pending_is_optional(), "the placement is 'you may'");
    let action = accept_branch(&runner);
    runner
        .execute_action(pick.selecting_player, action)
        .expect("pick the host");
    runner.auto_resolve().ok();

    let perm = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| {
            p.card_sources
                .iter()
                .any(|c| c.card_id(&runner.game.card_data) == DELAY_HOST_BASE)
        })
        .expect("the host stays in play");
    assert_eq!(perm.card_sources.len(), 1, "only its top card left the stack");
    assert_eq!(
        perm.top_card().card_id(&runner.game.card_data),
        DELAY_HOST_BASE,
        "the card that was under the top is the new top"
    );
    let top_security = runner.game.players[0]
        .security
        .last()
        .expect("security has cards")
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        top_security, DELAY_HOST_TOP,
        "the popped top card is the top security card"
    );
}

/// Declining the outer window keeps the Option in play; it is offered again at
/// the NEXT security removal.
#[test]
fn bt24_093_delay_decline_keeps_option_and_reoffers_on_next_removal() {
    let (mut runner, attacker) = delay_runner();
    runner.place_stack(0, &[DELAY_HOST_BASE, DELAY_HOST_TOP]);

    runner.attack_player(attacker, 0, false);
    let outer = runner.pending_selection_view().expect("first <Delay> window");
    runner
        .execute_action(outer.selecting_player, digimon_engine::action::space::PASS)
        .expect("decline");
    runner.auto_resolve().ok();
    assert!(option_on_field(&runner, 0), "a declined <Delay> stays (16-17-1)");

    // A second, separate removal opens a second window.
    let attacker2 = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.attack_player(attacker2, 0, false);
    assert!(
        runner.pending_selection_view().is_some(),
        "the next security removal re-offers the <Delay>"
    );
}

/// 16-17-3: not on the placing turn.
#[test]
fn bt24_093_delay_not_offered_on_the_placing_turn() {
    let mut runner = base()
        .add_card(attacker_for_p1("ATTACKER"))
        .security(0, &["FILL", "FILL"])
        .start();
    let turn = runner.game.turn_count;
    seat_delay(&mut runner, 0, turn);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));

    runner.attack_player(attacker, 0, false);

    assert!(
        runner.pending_selection_view().is_none(),
        "the <Delay> can't be activated the turn its card was placed (16-17-3)"
    );
    assert!(option_on_field(&runner, 0));
}

/// "When YOUR security stack is removed from": the opponent's own removals do
/// not open the Option's window.
#[test]
fn bt24_093_delay_ignores_the_opponents_security_removal() {
    let (mut runner, _) = delay_runner();
    // P0 attacks P1's security: that is P1's stack being removed from, not P0's.
    let p0_attacker = runner.place_on_field(0, "YELLOW-BOARD", Some(0));
    runner.attack_player(p0_attacker, 1, false);
    assert!(
        runner.pending_selection_view().is_none(),
        "the opponent's security loss is not this Option's trigger"
    );
    assert!(option_on_field(&runner, 0));
}

/// Only [Aegiochusmon]/[Jupitermon]-named Digimon with a card under their top
/// card are candidates: with none, the body finds nothing to pick and the
/// Option is still spent (DCGO trashes the Option BEFORE `SuccessProcess`).
#[test]
fn bt24_093_delay_candidates_need_the_name_and_a_card_under_the_top() {
    let (mut runner, attacker) = delay_runner();
    // A lone [Aegiochusmon] (no cards under) and a stacked non-matching name.
    runner.place_on_field(0, DELAY_HOST_TOP, Some(0));
    runner.place_stack(0, &["FILL", "OTHER"]);

    runner.attack_player(attacker, 0, false);
    let outer = runner.pending_selection_view().expect("the <Delay> window");
    let accept = accept_branch(&runner);
    runner
        .execute_action(outer.selecting_player, accept)
        .expect("accept");
    runner.auto_resolve().ok();
    assert!(!option_on_field(&runner, 0), "the Option's cost was paid");
    // No host qualified, so nothing was placed on security by the Delay.
    assert!(
        runner.game.players[0]
            .security
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != DELAY_HOST_TOP),
        "a lone [Aegiochusmon] with no card under it is not a candidate"
    );
}

#[test]
fn bt24_093_compiled_card_has_main_security_and_the_all_turns_delay_clauses() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let triggered_count = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(triggered_count, 2, "[Main] and inherited [Security]");

    let delay_triggers: Vec<CompiledTiming> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(
                digimon_dsl::compiled::CompiledDeclarativeClause::Delay { trigger, .. },
            ) => Some(*trigger),
            _ => None,
        })
        .collect();
    assert_eq!(
        delay_triggers,
        vec![CompiledTiming::OnOwnSecurityRemoved],
        "exactly one `kind: delay` clause, event-gated on the controller's own \
         security stack being removed from"
    );
}

// candidate legality marker (select-hand audit batch 12)

// ─── Section 6 — Security pick offers only playable cards ────────────────────
//
// DCGO BT24_093.cs:177-182 filters the hand AND trash candidates with
// `(EqualsCardName("Aegiomon") || EqualsCardName("Elecmon")) &&
// CanPlayAsNewPermanent(cardSource, false, activateClass)`, and
// `HasMatchConditionOwnersHand` / `HasMatchConditionOwnersCardInTrash`
// (186-187) decide whether a zone is offered at all. CR 4.3 1-3-11-1: a card
// can be declared for play only if it can be played; CR 4.3 1-3-2: an
// impossible play is not carried out, so offering it is a dead choice. Every
// real [Aegiomon] / [Elecmon] is a Digimon, so the checks that matter are the
// free slot, the effect-play prohibition and the trash-play lock.

/// Player 0 (security holder) with `card_id` in its hand, under player 1's
/// security attack. Returns the runner before the attack, plus the attacker.
fn security_pick_runner() -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let mut runner = base()
        .add_card(attacker_for_p1("ATTACKER"))
        .security(0, &[CARD_ID])
        .start();
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    (runner, attacker)
}

/// A full battle area (DCGO `CanPlayAsNewPermanent` needs a free frame; the
/// engine's play refuses with no slot): [Aegiomon] in hand is not offered and
/// no pick installs.
#[test]
fn bt24_093_security_pick_not_offered_when_battle_area_full() {
    let (mut runner, attacker) = security_pick_runner();
    push_to_hand(&mut runner, 0, "AEGIOMON");
    runner.place_on_field(0, "FILL", Some(0));
    runner.game.rules.field_slots = 1;
    let field_before = runner.battle_area_size(0);

    runner.attack_player(attacker, 0, false);

    assert!(
        runner.pending_selection().is_none(),
        "no free battle-area slot: the [Aegiomon] can't be played, so nothing is offered"
    );
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(0), field_before);
}

/// "You can't play Digimon by effects" on the effect's controller: no pick.
#[test]
fn bt24_093_security_pick_not_offered_when_effect_play_is_prohibited() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut runner, attacker) = security_pick_runner();
    push_to_hand(&mut runner, 0, "ELECMON");
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect, 0, Expiry::Permanent, None, 1),
    );

    runner.attack_player(attacker, 0, false);

    assert!(
        runner.pending_selection().is_none(),
        "an effect-play prohibition makes the [Elecmon] unplayable, so nothing is offered"
    );
}

/// "You can't play cards from the trash" (CannotPlayFromTrash, e.g. BT23-014):
/// an [Elecmon] in the trash is not offered.
#[test]
fn bt24_093_security_pick_not_offered_from_trash_when_trash_play_is_locked() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::PlayerModifierEntry;
    let (mut runner, attacker) = security_pick_runner();
    push_to_trash(&mut runner, 0, "ELECMON");
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotPlayFromTrash, 0, Expiry::Permanent, None, 1),
    );

    runner.attack_player(attacker, 0, false);

    assert!(
        runner.pending_selection().is_none(),
        "the trash [Elecmon] can't be played from the trash, so nothing is offered"
    );
    let _ = runner.auto_resolve();
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "ELECMON"),
        "Elecmon stays in the trash"
    );
}
