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
//!   security stack being removed from — **BLOCKED**, see gap notes below.
//!
//! # Engine/DSL gaps (BOTH block the [All Turns] clause; Clauses 0 and the
//! inherited Security effect are fully implemented and tested below)
//!
//! ## Gap 1 (NEW, this card) — `kind: delay` cannot event-gate on
//! `OnOwnSecurityRemoved`
//! `code/digimon-engine/src/dsl_cards/lower_delay.rs::lower_with_raw` maps
//! `CompiledTiming` → `DelayTrigger` via an explicit whitelist for
//! `DelayTrigger::OnEvent(..)`: `OnSuspend | OnUnsuspend | OnAllyPlayed |
//! OnAttack | OnAllyAttack | OnOpponentAttack`. `CompiledTiming::
//! OnOwnSecurityRemoved` (or `OnLoseSecurity`) is NOT in that list, so it
//! silently falls through the `_ => DelayTrigger::EndOfYourNextTurn` catch-all
//! — a wrong, unrelated auto-fire timing (traced by reading the full match
//! arm; confirmed no card in the shipping pool uses `on_own_security_removed`
//! / `on_lose_security` as a `kind: delay` `trigger:` today). Even if the
//! trigger mapped correctly, `Game::enqueue_triggered`'s event-gated-Delay
//! fan-out (`enqueue_event_gated_delayed_options`) is only invoked when the
//! firing `TriggerSource` matches a `matches!` whitelist of `EventObserved |
//! PlayerBattleAreaAttack | AttackTargetChanged | BlockDeclared |
//! EnteredField` (see `effect_queue.rs` ~line 620) — `TriggerSource::
//! SecurityRemoved` (what actually fires `OnOwnSecurityRemoved`, see
//! `complete_effect_security_removal` ~line 3302) is NOT in that whitelist
//! either. Two independent wiring gaps on the same trigger path.
//! Workaround: none faithful — substituting a different `trigger:` (e.g.
//! `end_of_your_next_turn`) would make the Delay auto-fire on a schedule
//! instead of specifically when the controller's OWN security stack shrinks,
//! which is observably different game behavior (no-approximations policy).
//!
//! ## Gap 2 (PRE-EXISTING, already tracked) — no primitive to pop a named
//! permanent's TOP card to security while leaving the rest of its stack
//! `docs/RUST_ENGINE_GAPS.md` § "Digivolution-stack source extraction
//! (`pop_top_source` from named permanent)" already names THIS card
//! (BT24-093) verbatim as the open residual after the BT20-084
//! `security_place_top_stacked_card` shape closed. Traced independently here:
//! - `security_place_(top_)stacked_card` extracts `card_sources[len-2]` (the
//!   MATERIAL just below the visible top) via `CardSourceRef::Material`,
//!   leaving the top card in place. DCGO's `AddSecurityCard(permanent.
//!   TopCard)` extracts `card_sources[len-1]` (the actual top / "digivolve
//!   root") instead — confirmed against `RemoveDigivolveRootEffect`'s name
//!   and behavior and against `docs/digimon-rules/keyword-semantics.md`'s
//!   `<Armor Purge>` row ("top card" = the permanent's visible top). Using
//!   `security_place_top_stacked_card` here would silently move the WRONG
//!   card.
//! - `place_on_security` with `SecuritySource::Permanent{..}` (→
//!   `CompiledStep::PlacePermanentOnSecurity` → `Game::
//!   place_permanent_on_security`) removes the WHOLE permanent from the
//!   battle area, sends only its top card to security, and TRASHES every
//!   other stacked card underneath — also wrong: Temple of Beginnings must
//!   leave the chosen Digimon on the field, de-digivolved by exactly one
//!   card, not remove it entirely.
//! - `ctx.de_digivolve` pops `card_sources.last()` correctly but hardcodes
//!   the destination to `player.trash` (`game_actions/helpers.rs`
//!   `de_digivolve_core`) with no way to redirect to security.
//! No DSL verb / `EffectContext` method composes "pop a named permanent's own
//! top card, keep the remaining stack on the battlefield, route the popped
//! card to security" — exactly the missing `ctx.pop_top_digivolution_source`
//! primitive the gap doc proposes. The doc's own text states: "Workaround:
//! None — BLOCKED." Raw `battle_area[i].card_sources.pop()` from a test/card
//! would skip curated-API bookkeeping (declarative re-tick, replacement
//! windows, `OnDigivolutionCardTrashed`-vs-moved distinction) and is
//! explicitly disallowed by the no-approximations / no-workarounds rules.
//!
//! Both gaps compound on the SAME clause (the `[All Turns] <Delay>` body), so
//! that clause is authored as an explicit YAML comment (no `kind: delay`
//! clause emitted) and its behavioral test is `#[ignore]`d citing both gaps.
//! Clauses 0 (Main) and the inherited Security Effect have zero dependency on
//! either gap and are fully implemented + tested below.

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
        main.process.iter().any(|s| matches!(s, CompiledStep::Recover { .. })),
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
        .find(|t| t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnSecurity))
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
        runner.auto_resolve().expect("Main body has no real player choices");
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
    let mut runner = base().hand(0, &[CARD_ID]).security(0, &[]).memory(10).start();
    runner.place_on_field(0, "YELLOW-BOARD", Some(0));
    let deck_top_before = runner
        .game
        .players[0]
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
    let mut runner = base().hand(0, &[CARD_ID]).security(0, &["FILL"]).memory(10).start();
    runner.place_on_field(0, "YELLOW-BOARD", Some(0));

    let result = runner.game.play_option_from_hand(0, 0);
    if matches!(result, OptionPlayResult::Pending) {
        runner.auto_resolve().expect("Main body has no real player choices");
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

    assert_eq!(runner.hand_size(0), hand_before, "declining leaves hand untouched");
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
    runner.execute_action(0, action).expect("pick the trash card");
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
// BLOCKED — see the file-header gap notes (Gap 1: lower_delay.rs trigger
// whitelist + effect_queue.rs TriggerSource whitelist; Gap 2: no primitive to
// pop a named permanent's own top card to security while leaving the rest of
// its stack on the field — docs/RUST_ENGINE_GAPS.md already names BT24-093
// verbatim under "Digivolution-stack source extraction", Workaround: None).
//
// This test documents the intended faithful behavior and is `#[ignore]`d
// until both gaps close. It is NOT a stub for a shipped clause: the YAML
// deliberately does not emit a `kind: delay` clause for this text (see the
// YAML's own comment block), so there is nothing today for this test to
// drive — the assertion below is a placeholder proving the compiled card has
// exactly the two clauses that ARE implemented (Main + inherited Security)
// and none for the All-Turns Delay body, so a future close of both gaps must
// add a THIRD clause and this test must be rewritten (not merely un-ignored).
#[test]
#[ignore = "pending: engine gap (lower_delay.rs OnEvent trigger whitelist missing OnOwnSecurityRemoved) + docs/RUST_ENGINE_GAPS.md 'Digivolution-stack source extraction (pop_top_source from named permanent)' (BT24-093 named verbatim, Workaround: None)"]
fn bt24_093_all_turns_delay_places_chosen_permanents_top_stacked_card_as_security() {
    // Intended faithful shape once both gaps close:
    //  1. BT24-093 sits on the battle area as a Delay Option past its
    //     placing turn.
    //  2. The controller's security stack is removed from (e.g. a security
    //     check trashes/adds-to-hand a card) — the Delay's event gate fires.
    //  3. The controller may trash BT24-093 (the Delay's cost) to activate:
    //     an optional select over own Aegiochusmon/Jupitermon-named
    //     permanents with >=1 digivolution card; on pick, that permanent's
    //     TOP card (not a lower material) is removed and placed as the new
    //     top security card, face-down, while the rest of its stack stays on
    //     the battlefield.
    //  4. Declining at any point (no eligible permanent, or the player
    //     passes) leaves the board untouched and BT24-093 stays trashed
    //     (Delay's cost is paid regardless — DCGO trashes it before running
    //     `SuccessProcess`).
    unreachable!(
        "intended-behavior placeholder only — do not implement via a workaround; see file header"
    );
}

#[test]
fn bt24_093_compiled_card_has_exactly_main_and_security_clauses_pending_all_turns_gap() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let triggered_count = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered_count, 2,
        "only [Main] and inherited [Security] are implemented today; the \
         [All Turns] <Delay> clause is BLOCKED (see file header) and must \
         NOT be silently emitted as a mis-triggered clause"
    );

    let declarative_delay_count = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(
                    digimon_dsl::compiled::CompiledDeclarativeClause::Delay { .. }
                )
            )
        })
        .count();
    assert_eq!(
        declarative_delay_count, 0,
        "no `kind: delay` clause is authored until both blocking gaps close \
         (a wrongly-triggered Delay would silently misfire on the wrong \
         timing — worse than omitting it)"
    );
}
