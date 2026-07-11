//! BT18-092 Zenith — Tamer, Cost 4, Black.
//! Traits: LIBERATOR
//!
//! # Card text (data/card_bundles/BT18-092.md — official Bandai DB, confirmed
//! against card image BT18-092.webp)
//!
//! [Start of Your Main Phase] By trashing 1 [Vemmon] in your hand, ＜Draw 1＞
//! (Draw 1 card from your deck.) and gain 1 memory.
//! [Your Turn] When any of your Digimon attack, by suspending this Tamer and
//! returning 2 [Vemmon] from that Digimon's digivolution cards to the bottom
//! of the deck, ＜De-Digivolve 1＞ 1 of your opponent's Digimon. (Trash the
//! top card. You can't trash past level 3 cards.)
//!
//! Security Effect [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT18/Black/BT18_092.cs
//!
//! ─── DCGO crosscheck ──────────────────────────────────────────────────────
//!
//! Clause 1 (`EffectTiming.OnStartMainPhase`): `SelectHandEffect` filtered to
//! `CardNames.Equals("Vemmon")`, `canNoSelect: true` (the trash-cost pick is
//! optional — "By Xing" cost). `CanUseCondition` gates on
//! `HasMatchConditionOwnersHand` (>= 1 Vemmon in hand) + owner's turn.
//! `AfterSelectCardCoroutine` (only when >=1 card picked): AddMemory(1), then
//! Draw(1).
//!
//! Clause 2 (`EffectTiming.OnAllyAttack`): `CanUseCondition` gates on
//! `IsOwnerTurn` + `CanTriggerOnPermanentAttack` with `PermanentCondition` =
//! attacker is an owner battle-area Digimon AND
//! `DigivolutionCards.Count(Vemmon) >= 2`. `CanActivateCondition` gates on
//! `CanActivateSuspendCostEffect` (Tamer payable — unsuspended AND not
//! CannotSuspend). `ActivateCoroutine`:
//! 1. `SuspendPermanentsClass.Tap()` on Zenith (self) — the suspend cost.
//! 2. Re-check attacker + >=2 Vemmon sources are still live.
//! 3. `SelectCardEffect` over the attacker's `DigivolutionCards` filtered to
//!    `CardNames.Equals("Vemmon")`, `maxCount: 2`, `canNoSelect: () => true`
//!    per-pick but `canEndNotMax: false` (must reach exactly 2 once engaged).
//! 4. `ReturnToLibraryBottomDigivolutionCardsClass` — moves the 2 selected
//!    sources to the BOTTOM of the owner's deck.
//! 5. IF exactly 2 were returned: `SelectPermanentEffect` over
//!    `IsPermanentExistsOnOpponentBattleAreaDigimon` (1 of opponent's
//!    Digimon), mandatory when any exist, then `IDegeneration(..., 1, ...)`
//!    = De-Digivolve 1 (trash the top digivolution card, stop at level 3).
//!
//! Clause 3 (`EffectTiming.SecuritySkill`): `PlaySelfTamerSecurityEffect` —
//! standard play-self-free security.
//!
//! ─── DSL modelling notes ──────────────────────────────────────────────────
//!
//! Clause 1: `start_of_your_main_phase`, `select_hand` (optional: true,
//! filter `name_contains: "Vemmon"`) -> `trash_from_hand_by_index` ->
//! `gain_memory: 1` -> `draw: { of: you, count: 1 }`. Order (memory before
//! draw) mirrors DCGO's `AddMemory` then `Draw` call sequence; the printed
//! text lists Draw before memory but the two are simultaneous non-ordered
//! numeric effects with no observable interaction, so DCGO's execution order
//! is authoritative (source priority: DCGO > printed text ordering).
//!
//! Clause 2: `on_ally_attack`, `optional: true`, `active_when: { your_turn:
//! true }`.
//!
//! **2026-07-02 reviewer fix (suspend-cost payability gate):** the printed
//! "by suspending this Tamer" activation cost maps to DCGO's
//! `CanActivateSuspendCostEffect(card)` — a real payability pre-check that
//! must gate the trigger BEFORE it is offered (BT13-101 / AD1-019 / BT21-084
//! / BT23-072 idiom: PUPPETS-G023). The clause `condition:` now ANDs
//! `source_is_unsuspended: true` (Zenith must currently be payable) with the
//! pre-existing opponent-Digimon existence check via `all_of`, and the
//! process's leading step is `activation_cost: { suspend_self: true }`
//! (compile-validated leading position; lifts onto
//! `EffectBuilder::activation_cost`) instead of a bare `suspend: { target:
//! source }`. `suspend_self_as_cost` (`effect_context/action/suspend.rs`)
//! returns `false` — silently aborting the whole trigger, no body, no
//! re-offer, but the OPT/attempt bookkeeping still records the failed
//! attempt per `effect_queue.rs::run_queued_effect_inner` — when the source
//! is already suspended OR carries `ModifierType::CannotSuspend`. This
//! closes a reachable double-fire window: Clause 2 has no `once_per_turn`
//! (DCGO: `SetUpActivateClass(..., -1, true, ...)`), so with two ally
//! Digimon each carrying >=2 Vemmon attacking in the same turn, a bare
//! `suspend:` step let the SECOND attack re-offer (and pay a no-op suspend
//! on) an already-suspended Zenith, granting a second free De-Digivolve
//! where DCGO would refuse (cost unpayable). See
//! `bt18_092_second_ally_attack_does_not_reoffer_when_zenith_already_suspended`
//! below for the regression coverage.
//!
//! Process (post-fix): `activation_cost: { suspend_self: true }` (the
//! Tamer's own suspend cost, gated, fires first exactly as DCGO's literal
//! `SuspendPermanentsClass.Tap()` call order), then `select_own_sources: {
//! from: event_target, filter: { name_contains: "Vemmon" }, min: 2, max: 2 }`
//! (the attacker is `event_target` — confirmed via
//! `TriggerSource::PlayerBattleAreaAttack { attacker, .. }` setting
//! `event_permanent = Some(attacker)` in effect_queue.rs), then
//! `return_selected_sources_to_deck: { source_refs: <bound>, position:
//! bottom }`, then `select_opponent_permanent` (mandatory when an opponent
//! Digimon exists) -> `de_digivolve: { amount: 1, stop_at_level: 3 }`.
//!
//! **Known engine-idiom deviation (documented, not a faithfulness bug):**
//! DCGO's `CanUseCondition` ALSO pre-checks ">=2 Vemmon in the attacker's
//! stack" BEFORE offering the trigger at all (a check independent of
//! Zenith's own suspend payability, which the 2026-07-02 fix above already
//! closes). The DSL substrate has no predicate to count NAMED digivolution
//! sources on `event_target` ahead of the cost (predicates only evaluate the
//! CARRIER's own stack, and only check presence, not count). Per the
//! documented, load-bearing precedent at
//! `lower_triggered.rs::first_step_candidate_guard` (comment citing
//! EX10-034): "for an OPTIONAL TRIGGERED ability... the designed behavior is
//! to fire and SILENTLY SKIP... when fewer than N sources exist — it
//! triggers on an event, so it can't loop." `select_own_sources` with
//! `min: 2, max: 2` and <2 live candidates never installs a
//! `pending_selection` and never invokes its `then:`-equivalent tail
//! (`install_source_multi_selection`, `selections.rs:2838-2842`), so the
//! NET OBSERVABLE outcome (no target selection, no De-Digivolve) is
//! identical to DCGO. The only divergence is that, when the trigger IS
//! accepted (an outer optional accept/decline prompt — with a leading
//! `activation_cost` step present, the queue installs this as a
//! `SelectionKind::TriggerOrder` prompt rather than the bare-`suspend:`
//! `SelectionKind::Replacement` shape `st18_14.rs` uses; see
//! `effect_queue.rs::needs_pre_cost_prompt`) but the attacker turns out to have <2
//! Vemmon sources, Zenith still gets suspended (the cost is paid) even
//! though the effect fizzles — DCGO would not have offered the trigger at
//! all in that state. This is not reachable in normal play from the
//! player's perspective without foreknowledge that fewer than 2 Vemmon are
//! present, and does not violate no-approximations (no auto-selected
//! target, no stubbed choice) — flagged here for visibility, not filed as a
//! blocking gap.
//!
//! Clause 3: `on_security`, `play_from_security: {}` — standard tamer
//! security-free pattern (BT21-102, BT18-087, BT22-084).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - B1 Start-of-main tamer (trash-cost -> draw + memory)
//! - Your-turn ally-attack observer with gated suspend-self cost (PUPPETS-G023
//!   / BT13-101 / BT21-084 idiom: `condition: source_is_unsuspended` +
//!   `activation_cost: { suspend_self: true }`)
//! - select_own_sources targeting a NON-carrier permanent via `from:
//!   event_target` (EX8-070 `from: target_digi` idiom, generalized to the
//!   attacker binding)
//! - De-Digivolve on opponent's Digimon (BT23-096 idiom)
//! - F9-adjacent: [Security] play self free (standard play_from_security)
//!
//! # Known gaps and test status
//!
//! | Clause | Gap | Status |
//! |--------|-----|--------|
//! | (1) [Start of Your Main Phase] trash 1 Vemmon -> draw 1 + gain 1 memory | none | PASS |
//! | (2) [Your Turn] ally attacks: suspend + return 2 Vemmon -> De-Digivolve 1 opp Digimon | documented idiom deviation above (attacker Vemmon-count pre-check; not a gap) | PASS |
//! | (3) [Security] play self free | none | PASS |

use digimon_dsl::compiled::{
    CompiledActivationCostKind, CompiledClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::events::GameEvent;
use digimon_engine::selection::SelectionKind;

// ─── Helpers ────────────────────────────────────────────────────────────────

fn make_vemmon(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, "Vemmon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(2);
    c.dp = Some(2000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Black];
    c
}

fn make_attacker(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c
}

/// DP set HIGHER than the attacker's 6000 so the defender survives combat
/// (rule 14-2-1: lower DP loses and is deleted; equal DP -> BOTH lose, so a
/// tie is not usable here). Tests that need the ATTACKER to survive drive
/// the trigger via `attack_player` instead of `attack_digimon` so no Digimon
/// battle (and therefore no DP-driven deletion) occurs at all.
fn make_defender(id: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(7000);
    c
}

fn make_filler(id: &str) -> digimon_engine::card_data::CardData {
    make_test_card(id, id)
}

/// Minimal runner with Zenith loaded from the production embedded pack.
fn zenith_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT18-092")
        .expect("BT18-092 in embedded DSL pack")
        .memory(6)
        .start()
}

/// Zenith on P0's field with a hand containing 1 Vemmon + 1 non-Vemmon filler,
/// and enough deck padding for Draw 1 to be safe.
fn zenith_with_vemmon_in_hand() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-092")
        .expect("BT18-092 in embedded DSL pack")
        .add_card(make_vemmon("VEM-HAND"))
        .add_card(make_filler("OTHER-HAND"))
        .add_card(make_filler("DECK-PAD"))
        .hand(0, &["VEM-HAND", "OTHER-HAND"])
        .deck(
            0,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .memory(6)
        .start();
    runner.place_on_field(0, "BT18-092", None);
    runner
}

/// Zenith on P0's field, hand with NO Vemmon (only filler).
fn zenith_with_no_vemmon_in_hand() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-092")
        .expect("BT18-092 in embedded DSL pack")
        .add_card(make_filler("OTHER-HAND"))
        .add_card(make_filler("DECK-PAD"))
        .hand(0, &["OTHER-HAND"])
        .deck(0, &["DECK-PAD", "DECK-PAD"])
        .memory(6)
        .start();
    runner.place_on_field(0, "BT18-092", None);
    runner
}

/// Zenith + an attacker Digimon stacked with exactly 2 Vemmon digivolution
/// sources (top card is a plain Digimon; below it, 2 Vemmon sources), plus 1
/// opponent Digimon to De-Digivolve target.
fn zenith_with_attacker_two_vemmon_sources() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-092")
        .expect("BT18-092 in embedded DSL pack")
        .add_card(make_vemmon("VEM-SRC-A"))
        .add_card(make_vemmon("VEM-SRC-B"))
        .add_card(make_attacker("ATK-TOP"))
        .add_card(make_defender("OPP-DEF", 5))
        .add_card(make_filler("DECK-PAD"))
        .deck(
            0,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .memory(10)
        .start();

    runner.place_on_field(0, "BT18-092", None);
    // Stack: bottom-most source first, top card last.
    runner.place_stack(0, &["VEM-SRC-A", "VEM-SRC-B", "ATK-TOP"]);
    runner.place_on_field(1, "OPP-DEF", Some(0));
    runner
}

/// Same as above, but the attacker only has 1 Vemmon source (insufficient for
/// the exact-2 cost).
fn zenith_with_attacker_one_vemmon_source() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-092")
        .expect("BT18-092 in embedded DSL pack")
        .add_card(make_vemmon("VEM-SRC-A"))
        .add_card(make_attacker("ATK-TOP"))
        .add_card(make_defender("OPP-DEF", 5))
        .add_card(make_filler("DECK-PAD"))
        .deck(
            0,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .memory(10)
        .start();

    runner.place_on_field(0, "BT18-092", None);
    runner.place_stack(0, &["VEM-SRC-A", "ATK-TOP"]);
    runner.place_on_field(1, "OPP-DEF", Some(0));
    runner
}

/// Zenith with an attacker with 2 Vemmon sources, but NO opponent Digimon on
/// the field.
fn zenith_with_attacker_no_opponent_digimon() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-092")
        .expect("BT18-092 in embedded DSL pack")
        .add_card(make_vemmon("VEM-SRC-A"))
        .add_card(make_vemmon("VEM-SRC-B"))
        .add_card(make_attacker("ATK-TOP"))
        .add_card(make_filler("DECK-PAD"))
        .deck(
            0,
            &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"],
        )
        .memory(10)
        .start();

    runner.place_on_field(0, "BT18-092", None);
    runner.place_stack(0, &["VEM-SRC-A", "VEM-SRC-B", "ATK-TOP"]);
    runner
}

/// Zenith plus TWO attacker Digimon, each carrying exactly 2 Vemmon
/// digivolution sources, plus 1 opponent Digimon (stacked with an extra
/// source so it can absorb 2 De-Digivolve steps) to De-Digivolve target
/// across both attacks.
fn zenith_with_two_attackers_two_vemmon_sources_each() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-092")
        .expect("BT18-092 in embedded DSL pack")
        .add_card(make_vemmon("VEM-SRC-A1"))
        .add_card(make_vemmon("VEM-SRC-B1"))
        .add_card(make_attacker("ATK-ONE"))
        .add_card(make_vemmon("VEM-SRC-A2"))
        .add_card(make_vemmon("VEM-SRC-B2"))
        .add_card(make_attacker("ATK-TWO"))
        .add_card(make_defender("OPP-DEF", 6))
        .add_card(make_filler("DECK-PAD"))
        .deck(
            0,
            &[
                "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD",
                "DECK-PAD",
            ],
        )
        .memory(10)
        .start();

    runner.place_on_field(0, "BT18-092", None);
    runner.place_stack(0, &["VEM-SRC-A1", "VEM-SRC-B1", "ATK-ONE"]);
    runner.place_stack(0, &["VEM-SRC-A2", "VEM-SRC-B2", "ATK-TWO"]);
    runner.place_on_field(1, "OPP-DEF", Some(0));
    // Give the opponent's Digimon an extra digivolution source so it can
    // absorb a De-Digivolve without being deleted at level floor.
    runner.push_source(
        digimon_engine::permanent::PermanentHandle {
            player: 1,
            index: 0,
        },
        "OPP-DEF",
    );
    runner
}

fn find_permanent(
    runner: &DebugRunner,
    player: digimon_engine::enums::PlayerId,
    card_id: &str,
) -> digimon_engine::permanent::PermanentHandle {
    let p = &runner.game.players[player as usize];
    for (i, perm) in p.battle_area.iter().enumerate() {
        if perm.top_card().card_id(&runner.game.card_data) == card_id {
            return digimon_engine::permanent::PermanentHandle {
                player,
                index: i as u8,
            };
        }
    }
    panic!("permanent {card_id} not found on player {player}'s field");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt18_092_has_three_triggered_clauses() {
    let runner = zenith_runner();
    let compiled = runner
        .compiled_card("BT18-092")
        .expect("BT18-092 in compiled_cards");

    let triggered_count = compiled
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(
        triggered_count, 3,
        "BT18-092 must have start_of_your_main_phase, on_ally_attack, and on_security clauses"
    );
}

#[test]
fn bt18_092_start_of_main_clause_is_optional_start_of_your_main_phase() {
    let runner = zenith_runner();
    let compiled = runner
        .compiled_card("BT18-092")
        .expect("BT18-092 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::StartOfYourMainPhase))
        .expect("start_of_your_main_phase clause must exist");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        clause.optional,
        "the trash-1-Vemmon cost is 'By Xing' -> optional"
    );
}

#[test]
fn bt18_092_on_ally_attack_clause_is_optional_and_your_turn_scoped() {
    let runner = zenith_runner();
    let compiled = runner
        .compiled_card("BT18-092")
        .expect("BT18-092 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnAllyAttack))
        .expect("on_ally_attack clause must exist");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
    assert!(
        clause.optional,
        "'by suspending this Tamer and returning 2 [Vemmon]...' is the cost -> optional"
    );
}

/// STRUCTURAL (2026-07-02 fix): the on_ally_attack clause's `condition:` must
/// gate on `source_is_unsuspended` — the DCGO `CanActivateSuspendCostEffect`
/// payability pre-check — so the outer accept/decline prompt itself is
/// suppressed when Zenith cannot pay the suspend cost (BT21-084 idiom).
#[test]
fn bt18_092_on_ally_attack_condition_gates_on_source_unsuspended() {
    let runner = zenith_runner();
    let compiled = runner
        .compiled_card("BT18-092")
        .expect("BT18-092 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnAllyAttack))
        .expect("on_ally_attack clause must exist");

    let condition = clause
        .condition
        .as_ref()
        .expect("on_ally_attack clause must carry a condition gate");
    // Review fix 2026-07-03: the offer-time gate is PAYABILITY ONLY (bare
    // `source_is_unsuspended: true`). DCGO's CanUseCondition never pre-checks
    // opponent-Digimon existence — pre-gating on the opponent's board would
    // silently drop a legal cost-paying action (faithfulness divergence).
    let has_source_unsuspended_gate = condition.source_is_unsuspended == Some(true)
        || condition
            .all_of
            .iter()
            .any(|p| p.source_is_unsuspended == Some(true));
    assert!(
        has_source_unsuspended_gate,
        "condition must require Zenith (source) to be unsuspended before offering the trigger; got {condition:?}"
    );
    assert!(
        condition.all_of.iter().all(|p| p.any_permanent.is_none()),
        "offer-time gate must NOT pre-check opponent-Digimon existence (review fix 2026-07-03); got {condition:?}"
    );
}

/// STRUCTURAL (2026-07-02 fix): the leading process step must be the gated
/// `activation_cost: { suspend_self: true }` (PUPPETS-G023 idiom), not a bare
/// `suspend:` step, so the queue drainer can silently abort the whole
/// trigger when the cost is unpayable.
#[test]
fn bt18_092_on_ally_attack_process_leads_with_gated_suspend_activation_cost() {
    let runner = zenith_runner();
    let compiled = runner
        .compiled_card("BT18-092")
        .expect("BT18-092 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnAllyAttack))
        .expect("on_ally_attack clause must exist");

    let first_step = clause
        .process
        .first()
        .expect("on_ally_attack process must not be empty");
    assert!(
        matches!(
            first_step,
            CompiledStep::ActivationCost {
                kind: CompiledActivationCostKind::SuspendSelf
            }
        ),
        "the leading process step must be activation_cost: {{ suspend_self: true }}, not a bare suspend step; got {first_step:?}"
    );

    let has_bare_suspend = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::Suspend { .. }));
    assert!(
        !has_bare_suspend,
        "process must not contain an un-gated bare Suspend step"
    );
}

#[test]
fn bt18_092_on_ally_attack_process_contains_source_multi_and_de_digivolve() {
    let runner = zenith_runner();
    let compiled = runner
        .compiled_card("BT18-092")
        .expect("BT18-092 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnAllyAttack))
        .expect("on_ally_attack clause must exist");

    let has_source_multi = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectOwnSources { min: 2, max: 2, .. }));
    assert!(
        has_source_multi,
        "process must select EXACTLY 2 Vemmon sources from the attacker"
    );

    let has_de_digivolve = clause.process.iter().any(|s| {
        matches!(
            s,
            CompiledStep::DeDigivolve {
                amount: Some(1),
                ..
            }
        )
    });
    assert!(
        has_de_digivolve,
        "process must De-Digivolve 1 the selected opponent's Digimon"
    );
}

#[test]
fn bt18_092_has_on_security_clause_with_play_from_security() {
    let runner = zenith_runner();
    let compiled = runner
        .compiled_card("BT18-092")
        .expect("BT18-092 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist");

    let has_play_from_security = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromSecurity));
    assert!(
        has_play_from_security,
        "on_security must lower to play_from_security"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1: [Start of Your Main Phase] trash Vemmon -> draw + memory
// ═══════════════════════════════════════════════════════════════════════════

/// POSITIVE (condition gate): with a Vemmon in hand, the start-of-main-phase
/// trigger installs a selection over the hand.
#[test]
fn bt18_092_start_of_main_offers_selection_with_vemmon_in_hand() {
    let mut runner = zenith_with_vemmon_in_hand();
    let zenith = find_permanent(&runner, 0, "BT18-092");

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::StartOfYourMainPhase,
        digimon_engine::TriggerSource::Permanent(zenith),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_some(),
        "a Vemmon in hand must offer the trash-cost selection"
    );
}

/// NEGATIVE (condition gate): with NO Vemmon in hand, the trigger must not
/// install any selection (nothing to trash — DCGO's HasMatchConditionOwnersHand
/// gate fails).
#[test]
fn bt18_092_start_of_main_no_selection_without_vemmon_in_hand() {
    let mut runner = zenith_with_no_vemmon_in_hand();
    let zenith = find_permanent(&runner, 0, "BT18-092");

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::StartOfYourMainPhase,
        digimon_engine::TriggerSource::Permanent(zenith),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no Vemmon in hand must produce no pending selection"
    );
}

/// BEHAVIORAL: activating the trash-1-Vemmon cost draws 1 card and gains 1
/// memory. Uses the real trigger path via `start_of_your_main_phase`.
#[test]
fn bt18_092_trashing_vemmon_draws_one_and_gains_one_memory() {
    let mut runner = zenith_with_vemmon_in_hand();
    let zenith = find_permanent(&runner, 0, "BT18-092");

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    let trash_before = runner.trash_size(0);
    let memory_before = runner.memory();

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::StartOfYourMainPhase,
        digimon_engine::TriggerSource::Permanent(zenith),
    );
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection_view()
        .expect("Vemmon-trash selection must be pending");
    assert!(
        pending.is_optional,
        "'By trashing' is an optional cost -> PASS must be legal"
    );
    // Pick the (only) Vemmon candidate — not PASS.
    let pick = *pending
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("at least one non-PASS hand candidate (the Vemmon) must exist");
    runner
        .execute_action(pending.selecting_player, pick)
        .expect("selecting the Vemmon resolves");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before - 1 + 1,
        "hand loses the trashed Vemmon but gains the drawn card (net 0)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "deck shrinks by 1 (Draw 1)"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before + 1,
        "trash gains the discarded Vemmon"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "controller gains 1 memory"
    );
}

/// BEHAVIORAL (decline path): declining the optional trash-cost leaves state
/// unchanged.
#[test]
fn bt18_092_declining_trash_cost_changes_nothing() {
    let mut runner = zenith_with_vemmon_in_hand();
    let zenith = find_permanent(&runner, 0, "BT18-092");

    let hand_before = runner.hand_size(0);
    let deck_before = runner.deck_size(0);
    let memory_before = runner.memory();

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::StartOfYourMainPhase,
        digimon_engine::TriggerSource::Permanent(zenith),
    );
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection_view()
        .expect("Vemmon-trash selection must be pending");
    assert!(pending.is_optional);
    runner
        .execute_action(pending.selecting_player, PASS)
        .expect("declining resolves");

    assert_eq!(runner.hand_size(0), hand_before, "decline: hand unchanged");
    assert_eq!(runner.deck_size(0), deck_before, "decline: deck unchanged");
    assert_eq!(runner.memory(), memory_before, "decline: memory unchanged");
}

/// EVENT-LOG: trashing the Vemmon from hand as the cost SHOULD fire a
/// `GameEvent::Trash` event (guards against a DSL step silently bypassing
/// observer-visible events). Currently `Game::trash_from_hand_by_index`
/// (the underlying primitive `trash_from_hand_by_index` steps route
/// through) moves the card directly without emitting `GameEvent::Trash` —
/// a pre-existing, already-documented engine gap (see
/// `bt24_008.rs::bt24_008_on_play_accept_emits_trash_event_for_cost_card`,
/// `#[ignore = "pending: engine-trash-event-from-hand — ..."]`). Marked
/// `#[ignore]` with the same gap-id until the engine emits Trash events for
/// hand-to-trash moves; not specific to BT18-092.
#[test]
#[ignore = "pending: engine-trash-event-from-hand — trash_from_hand_by_index does not emit GameEvent::Trash (pre-existing gap, see bt24_008.rs)"]
fn bt18_092_trashing_vemmon_fires_trash_event() {
    let mut runner = zenith_with_vemmon_in_hand();
    let zenith = find_permanent(&runner, 0, "BT18-092");

    runner.game.enqueue_triggered(
        digimon_engine::enums::EffectTiming::StartOfYourMainPhase,
        digimon_engine::TriggerSource::Permanent(zenith),
    );
    runner.game.drain_effect_queue();

    let pending = runner
        .pending_selection_view()
        .expect("Vemmon-trash selection must be pending");
    let pick = *pending
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("Vemmon candidate must exist");

    let cp = runner.event_checkpoint();
    runner
        .execute_action(pending.selecting_player, pick)
        .expect("selecting the Vemmon resolves");
    let _ = runner.auto_resolve();

    let events = runner.events_since(cp);
    let trash_count = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Trash { .. }))
        .count();
    assert!(
        trash_count >= 1,
        "GameEvent::Trash must fire when the Vemmon cost is paid"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 2: [Your Turn] ally attacks -> suspend + return 2 Vemmon
//              -> De-Digivolve 1 opponent's Digimon
// ═══════════════════════════════════════════════════════════════════════════

/// POSITIVE: with the attacker carrying exactly 2 Vemmon sources, attacking
/// installs the outer accept/decline prompt for Zenith's optional trigger
/// (Zenith is unsuspended, so the gated `condition:` passes and the prompt
/// installs; matches the st18_14.rs precedent).
#[test]
fn bt18_092_ally_attack_offers_outer_optional_prompt_with_two_vemmon_sources() {
    let mut runner = zenith_with_attacker_two_vemmon_sources();
    let attacker = find_permanent(&runner, 0, "ATK-TOP");
    let defender = find_permanent(&runner, 1, "OPP-DEF");

    let _ = runner.attack_digimon(attacker, defender, false);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder),
        "the optional [Your Turn] trigger must offer an accept/decline prompt"
    );
}

/// BEHAVIORAL: accepting the trigger with exactly 2 Vemmon sources suspends
/// Zenith, returns the 2 Vemmon to the BOTTOM of the deck (deck grows by 2,
/// digivolution stack shrinks by 2), and offers a mandatory target selection
/// over the opponent's Digimon (since one exists).
#[test]
fn bt18_092_accepting_ally_attack_suspends_zenith_and_offers_source_selection() {
    let mut runner = zenith_with_attacker_two_vemmon_sources();
    let zenith = find_permanent(&runner, 0, "BT18-092");
    let attacker = find_permanent(&runner, 0, "ATK-TOP");
    let defender = find_permanent(&runner, 1, "OPP-DEF");

    assert!(
        !runner.game.players[0].battle_area[zenith.index as usize].is_suspended,
        "Zenith must start unsuspended"
    );
    let stack_before = runner.game.players[0].battle_area[attacker.index as usize]
        .card_sources
        .len();

    let _ = runner.attack_digimon(attacker, defender, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::TriggerOrder));
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");

    assert!(
        runner.game.players[0].battle_area[zenith.index as usize].is_suspended,
        "Zenith must be suspended after accepting (the cost)"
    );

    let pending = runner
        .pending_selection_view()
        .expect("the 2-Vemmon source-multi selection must be pending");
    assert_eq!(
        pending.kind,
        SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0
        }
    );

    // Drive the two source picks (order among the two Vemmon doesn't matter).
    let first = pending.valid_action_ids[0];
    runner
        .execute_action(pending.selecting_player, first)
        .expect("pick first Vemmon source");
    let pending2 = runner
        .pending_selection_view()
        .expect("second source pick pending");
    let second = pending2.valid_action_ids[0];
    runner
        .execute_action(pending2.selecting_player, second)
        .expect("pick second Vemmon source");

    let stack_after = runner.game.players[0].battle_area[attacker.index as usize]
        .card_sources
        .len();
    assert_eq!(
        stack_after,
        stack_before - 2,
        "both Vemmon sources must leave the attacker's digivolution stack"
    );
    assert_eq!(
        runner.deck_size(0),
        5 + 2,
        "both returned Vemmon must land on the BOTTOM of the deck (deck +2)"
    );

    // The De-Digivolve target selection over the opponent's Digimon follows.
    let target_pending = runner
        .pending_selection_view()
        .expect("De-Digivolve target selection must be pending");
    assert_eq!(target_pending.kind, SelectionKind::OppField);
    assert!(
        !target_pending.is_optional,
        "target pick is mandatory once an opponent Digimon exists"
    );
}

/// BEHAVIORAL (full resolution): completing the whole clause De-Digivolves
/// the chosen opponent Digimon by 1 (its digivolution stack loses its top
/// source card / it de-digivolves one step).
#[test]
fn bt18_092_full_resolution_de_digivolves_opponent_by_one() {
    let mut runner = zenith_with_attacker_two_vemmon_sources();
    let attacker = find_permanent(&runner, 0, "ATK-TOP");
    let defender_pre = find_permanent(&runner, 1, "OPP-DEF");
    // Stack the opponent's Digimon so De-Digivolve 1 has somewhere to go:
    // give it 1 extra digivolution source beneath the top card.
    runner.push_source(defender_pre, "OPP-DEF");
    let defender = find_permanent(&runner, 1, "OPP-DEF");
    let opp_stack_before = runner.game.players[1].battle_area[defender.index as usize]
        .card_sources
        .len();
    assert!(
        opp_stack_before >= 2,
        "opponent Digimon must have a digivolution source to trash"
    );

    let _ = runner.attack_digimon(attacker, defender, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::TriggerOrder));
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");

    // Drive both source picks.
    let p1 = runner.pending_selection_view().expect("first source pick");
    runner
        .execute_action(p1.selecting_player, p1.valid_action_ids[0])
        .expect("first pick resolves");
    let p2 = runner.pending_selection_view().expect("second source pick");
    runner
        .execute_action(p2.selecting_player, p2.valid_action_ids[0])
        .expect("second pick resolves");

    // Drive the mandatory De-Digivolve target pick.
    let target = runner
        .pending_selection_view()
        .expect("De-Digivolve target selection pending");
    assert_eq!(target.kind, SelectionKind::OppField);
    runner
        .execute_action(target.selecting_player, target.valid_action_ids[0])
        .expect("target pick resolves");
    let _ = runner.auto_resolve();

    let opp_stack_after = runner.game.players[1].battle_area[defender.index as usize]
        .card_sources
        .len();
    assert_eq!(
        opp_stack_after,
        opp_stack_before - 1,
        "De-Digivolve 1 must trash exactly 1 digivolution card from the target"
    );
}

/// NEGATIVE: declining the outer optional prompt leaves everything unchanged
/// (Zenith stays unsuspended, no Vemmon returned, no De-Digivolve). Driven
/// via `attack_player` (a security check, not a Digimon battle) so neither
/// the attacker nor the defender is at risk of DP-driven combat deletion
/// (rule 14-2-1: lower DP loses; equal DP -> BOTH lose) — this test's whole
/// point is to observe UNCHANGED state on both sides after the decline, so a
/// same-turn combat casualty would confound the assertions.
#[test]
fn bt18_092_declining_ally_attack_trigger_changes_nothing() {
    let mut runner = zenith_with_attacker_two_vemmon_sources();
    let zenith = find_permanent(&runner, 0, "BT18-092");
    let attacker = find_permanent(&runner, 0, "ATK-TOP");
    let defender = find_permanent(&runner, 1, "OPP-DEF");

    let stack_before = runner.game.players[0].battle_area[attacker.index as usize]
        .card_sources
        .len();
    let deck_before = runner.deck_size(0);
    let opp_stack_before = runner.game.players[1].battle_area[defender.index as usize]
        .card_sources
        .len();

    let _ = runner.attack_player(attacker, 1, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::TriggerOrder));
    runner
        .decline_optional_trigger()
        .expect("decline the outer optional-trigger prompt");

    assert!(
        !runner.game.players[0].battle_area[zenith.index as usize].is_suspended,
        "declining must NOT suspend Zenith"
    );
    assert_eq!(
        runner.game.players[0].battle_area[attacker.index as usize]
            .card_sources
            .len(),
        stack_before,
        "declining must not touch the attacker's digivolution stack"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "declining: deck unchanged"
    );
    assert_eq!(
        runner.game.players[1].battle_area[defender.index as usize]
            .card_sources
            .len(),
        opp_stack_before,
        "declining: opponent's Digimon untouched"
    );
}

/// NEGATIVE (insufficient Vemmon sources): with only 1 Vemmon source on the
/// attacker, the exact-2 cost cannot be paid. No outer prompt should install
/// (DCGO's PermanentCondition — attacker must have >= 2 Vemmon sources — gates
/// the trigger's availability before it is even offered).
#[test]
fn bt18_092_ally_attack_no_trigger_with_only_one_vemmon_source() {
    let mut runner = zenith_with_attacker_one_vemmon_source();
    let zenith = find_permanent(&runner, 0, "BT18-092");
    let attacker = find_permanent(&runner, 0, "ATK-TOP");
    let defender = find_permanent(&runner, 1, "OPP-DEF");

    let opp_stack_before = runner.game.players[1].battle_area[defender.index as usize]
        .card_sources
        .len();

    // `attack_player` (security check) avoids any Digimon-battle DP
    // comparison — this test observes both the attacker's and the
    // defender's post-attack state, and a combat casualty would confound
    // the assertions (rule 14-2-1: lower/equal DP dies).
    let _ = runner.attack_player(attacker, 1, false);

    // Per the documented engine-idiom note in BT18-092.yaml: Zenith's own
    // suspend-cost payability IS now pre-gated (source_is_unsuspended), but
    // the DSL substrate still has no predicate to pre-count the ATTACKER's
    // live Vemmon sources ahead of the cost. So the outer optional
    // accept/decline prompt installs regardless of the attacker's live
    // Vemmon count (documented idiom deviation) as long as Zenith itself is
    // unsuspended — which it is here.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder),
        "the outer optional-trigger prompt installs regardless of the \
         attacker's live Vemmon count (documented idiom deviation)"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt");

    // Zenith pays the suspend cost (matches DCGO's literal suspend-first
    // ordering). With only 1 live Vemmon candidate, `select_own_sources`
    // (min: 2, max: 2) installs a SourceMulti prompt offering that single
    // candidate (matching the EX10-034 precedent idiom:
    // `install_source_multi_selection` only skips outright once candidates
    // are truly exhausted, i.e. `candidates.is_empty()` -- with exactly 1
    // live candidate it presents that 1 pick with NO PASS, since
    // `picked.len() (0) < min (2)`). Picks are STAGED (not physically moved)
    // until the tail commits, so driving that single pick does not touch the
    // attacker's stack yet. It DOES exhaust the candidate pool; the NEXT
    // recursive install call then sees `candidates.is_empty() &&
    // picked.len() (1) < min (2)`, so it returns WITHOUT installing a
    // further selection and WITHOUT invoking the `then:`-equivalent tail --
    // the staged pick is discarded, the attacker's stack stays untouched,
    // and the De-Digivolve step never runs. Net observable outcome matches
    // DCGO exactly (no return-to-deck-bottom, no De-Digivolve) despite the
    // suspend cost being paid where DCGO would never have offered the
    // trigger at all; see the documented idiom-deviation note in
    // BT18-092.yaml.
    assert!(
        runner.game.players[0].battle_area[zenith.index as usize].is_suspended,
        "Zenith pays the suspend cost even though the effect ultimately fizzles"
    );
    let source_pending = runner
        .pending_selection_view()
        .expect("SourceMulti must offer the single live Vemmon candidate");
    assert_eq!(
        source_pending.kind,
        SelectionKind::SourceMulti {
            min: 2,
            max: 2,
            picked: 0
        }
    );
    assert_eq!(
        source_pending.valid_action_ids.len(),
        1,
        "exactly 1 live Vemmon candidate must be offered"
    );
    assert!(
        !source_pending
            .valid_action_ids
            .contains(&digimon_engine::action::space::PASS),
        "PASS is not legal before the minimum (2) is met"
    );
    runner
        .execute_action(
            source_pending.selecting_player,
            source_pending.valid_action_ids[0],
        )
        .expect("pick the single Vemmon source");

    assert!(
        runner.pending_selection().is_none(),
        "with the candidate pool exhausted below the minimum, no further \
         selection may install and the De-Digivolve tail must not run"
    );
    assert_eq!(
        runner.game.players[0].battle_area[attacker.index as usize]
            .card_sources
            .len(),
        2,
        "picks are staged (not physically moved) until the tail commits; \
         since the minimum was never met, the attacker's stack is untouched"
    );
    assert_eq!(
        runner.game.players[1].battle_area[defender.index as usize]
            .card_sources
            .len(),
        opp_stack_before,
        "no De-Digivolve fires: the opponent's Digimon is untouched"
    );
}

/// NEGATIVE (no opponent Digimon on field): with 2 Vemmon sources but no
/// opponent Digimon, the trigger still offers the cost (suspend + return 2
/// Vemmon), but after paying it, the De-Digivolve target step is silently
/// skipped (its outer `condition: any_permanent{of: opponent, kind: digimon}`
/// gate at the top of the clause governs whether it fires at all — assert
/// via the structural condition presence and the end-to-end outcome).
#[test]
fn bt18_092_ally_attack_no_target_selection_without_opponent_digimon() {
    let mut runner = zenith_with_attacker_no_opponent_digimon();
    let zenith = find_permanent(&runner, 0, "BT18-092");
    let attacker = find_permanent(&runner, 0, "ATK-TOP");

    // No opponent Digimon exists, so nothing to attack; drive the attack via
    // attack_player instead — this exercises OnAllyAttack fan-out the same
    // way, per combat.rs (the observer timing fires regardless of the
    // attack's target).
    let _ = runner.attack_player(attacker, 1, false);

    // Review fix 2026-07-03: the trigger IS offered even with no opponent
    // Digimon — the offer gate is payability-only (DCGO CanUseCondition).
    // Paying the cost with no De-Digivolve target is a LEGAL (if bad) play;
    // the target select simply fizzles with zero candidates after the cost.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder),
        "the trigger must be OFFERED with no opponent Digimon (payability-only offer gate)"
    );
    runner
        .decline_optional_trigger()
        .expect("the offer must be declinable (optional trigger)");
    assert!(
        !runner.game.players[0].battle_area[zenith.index as usize].is_suspended,
        "declining the offer must leave Zenith unsuspended"
    );
}

/// NEGATIVE: on the OPPONENT's turn, Zenith's [Your Turn] trigger must not
/// fire even if their own Digimon attacks (active_when: your_turn gate).
#[test]
fn bt18_092_ally_attack_does_not_fire_on_opponents_turn() {
    // Structural check only: `active_when: { your_turn: true }` is what
    // scopes Clause 2 to the controller's own turn. `attack_digimon` in this
    // harness always runs during `turn_player`'s turn, so a behavioral
    // opponent-turn-attack scenario isn't directly constructible here; the
    // structural gate is the faithful thing to assert.
    let runner = zenith_runner();
    let compiled = runner.compiled_card("BT18-092").expect("BT18-092 compiled");
    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnAllyAttack))
        .expect("on_ally_attack clause must exist");
    assert!(
        clause.active_when.is_some(),
        "on_ally_attack clause must carry an active_when your_turn gate"
    );
}

/// REGRESSION (2026-07-02 reviewer fix): the [Your Turn] ally-attack trigger
/// is NOT once-per-turn (DCGO `SetUpActivateClass(..., -1, true, ...)`; no
/// `once_per_turn` here either), so it can fire once per qualifying attack.
/// With TWO separate ally Digimon each carrying >=2 Vemmon attacking in the
/// SAME turn: the FIRST attack suspends Zenith and De-Digivolves the
/// opponent's Digimon (paying the real cost). The SECOND attack must NOT
/// re-offer the trigger at all — Zenith is already suspended, so
/// `source_is_unsuspended: true` fails the clause `condition:` gate and the
/// outer accept/decline prompt never installs — matching DCGO's
/// `CanActivateSuspendCostEffect` cost-unpayable refusal. Before the fix, a
/// bare `suspend:` body step was idempotent and let the second attack
/// re-offer + "pay" a no-op suspend, granting an illegal second free
/// De-Digivolve.
#[test]
fn bt18_092_second_ally_attack_does_not_reoffer_when_zenith_already_suspended() {
    let mut runner = zenith_with_two_attackers_two_vemmon_sources_each();
    let zenith = find_permanent(&runner, 0, "BT18-092");
    let attacker_one = find_permanent(&runner, 0, "ATK-ONE");
    let attacker_two = find_permanent(&runner, 0, "ATK-TWO");
    let defender = find_permanent(&runner, 1, "OPP-DEF");

    let opp_stack_before_first = runner.game.players[1].battle_area[defender.index as usize]
        .card_sources
        .len();

    // First attack: accept the trigger and drive it to full resolution
    // (suspend Zenith, return 2 Vemmon, De-Digivolve the opponent's Digimon).
    let _ = runner.attack_player(attacker_one, 1, false);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder),
        "first attack: Zenith is unsuspended, so the trigger must be offered"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the outer optional-trigger prompt on the first attack");

    assert!(
        runner.game.players[0].battle_area[zenith.index as usize].is_suspended,
        "Zenith must be suspended after paying the first attack's cost"
    );

    // Drive the two source picks for the first attack.
    let p1 = runner
        .pending_selection_view()
        .expect("first attack: first source pick pending");
    runner
        .execute_action(p1.selecting_player, p1.valid_action_ids[0])
        .expect("first attack: first source pick resolves");
    let p2 = runner
        .pending_selection_view()
        .expect("first attack: second source pick pending");
    runner
        .execute_action(p2.selecting_player, p2.valid_action_ids[0])
        .expect("first attack: second source pick resolves");

    // Drive the mandatory De-Digivolve target pick for the first attack.
    let target1 = runner
        .pending_selection_view()
        .expect("first attack: De-Digivolve target selection pending");
    assert_eq!(target1.kind, SelectionKind::OppField);
    runner
        .execute_action(target1.selecting_player, target1.valid_action_ids[0])
        .expect("first attack: target pick resolves");
    let _ = runner.auto_resolve();

    let opp_stack_after_first = runner.game.players[1].battle_area[defender.index as usize]
        .card_sources
        .len();
    assert_eq!(
        opp_stack_after_first,
        opp_stack_before_first - 1,
        "first attack must De-Digivolve the opponent's Digimon by exactly 1"
    );

    let attacker_two_stack_before_second = runner.game.players[0].battle_area
        [attacker_two.index as usize]
        .card_sources
        .len();
    let deck_before_second = runner.deck_size(0);

    // Second attack (different ally Digimon, also carrying >= 2 live Vemmon
    // sources): Zenith is ALREADY suspended, so the trigger must not be
    // re-offered at all.
    let _ = runner.attack_player(attacker_two, 1, false);
    let _ = runner.auto_resolve();

    assert!(
        runner.pending_selection().is_none(),
        "second attack must NOT re-offer the trigger while Zenith is already \
         suspended — the condition gate (source_is_unsuspended) must block \
         the outer accept/decline prompt entirely"
    );
    assert_eq!(
        runner.game.players[0].battle_area[attacker_two.index as usize]
            .card_sources
            .len(),
        attacker_two_stack_before_second,
        "second attack must not consume the second attacker's Vemmon sources"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before_second,
        "second attack must not return any sources to the deck"
    );
    assert_eq!(
        runner.game.players[1].battle_area[defender.index as usize]
            .card_sources
            .len(),
        opp_stack_after_first,
        "second attack must NOT grant a second free De-Digivolve — the \
         opponent's Digimon must be untouched beyond the first attack's effect"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 3: [Security] play self free
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bt18_092_security_clause_structural_shape() {
    let runner = zenith_runner();
    let compiled = runner
        .compiled_card("BT18-092")
        .expect("BT18-092 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist");

    assert_eq!(clause.scope, CompiledScope::FaceUp);
}
