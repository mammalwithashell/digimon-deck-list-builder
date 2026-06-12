//! Medusamon — archetype interaction tests.
//!
//! Model: `qa/archetype-qa/medusamon-model.md`. Medusamon is a Red
//! [LIBERATOR]/[Reptile]·[Dragonkin] aggro-combo deck whose whole engine keys
//! off "[When] your opponent's security stack is removed from", and which
//! *manufactures* that condition via Petrification Tokens. These tests pin the
//! cross-card consequences the per-card `cards_behavioral/` tests cannot see —
//! each `#[test]` maps 1:1 to a named combo in the model.
//!
//! **All Medusamon cards in these combos are the REAL implemented DSL cards**
//! (loaded via `dsl_card`). Synthetic `make_test_card` fixtures are used ONLY
//! for generic *opponent-side* fillers — the opponent is not playing Medusamon,
//! so their security cards / bodies / hand cards have no archetype identity. No
//! synthetic stand-in is ever used for one of our own effect-bearing cards.

#![allow(dead_code)]

use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use super::support::snapshot;

// ─── Opponent-side neutral fillers (the opponent does NOT play Medusamon) ─────

/// A generic opponent Digimon / security / hand card with a chosen DP. Used only
/// for the opponent's board — never as a stand-in for one of our cards.
fn opp_filler(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

/// Spawn a Petrification Token on `controller`'s battle area; return its handle.
/// (`ctx.play_token` idiom — proven in `cards_behavioral/ex11/ex11_012.rs`.)
fn spawn_petrification_token(runner: &mut DebugRunner, controller: u8) -> PermanentHandle {
    let mut ctx = EffectContext::new(&mut runner.game, CardHandle(0), None, controller);
    ctx.play_token(controller, "petrification")
        .expect("petrification token spawns")
}

/// Count Token permanents on player `p`'s battle area.
fn token_count(runner: &DebugRunner, p: u8) -> usize {
    runner
        .game
        .player(p)
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_kind(&runner.game.card_data) == CardKind::Token)
        .count()
}

/// Resolve any chained prompts: opponent-side mandatory selections via their
/// first legal action; our own optional gates (e.g. Owen's buff offer) declined
/// via PASS so a combo under test isn't perturbed by an unrelated optional.
fn settle(runner: &mut DebugRunner) {
    for _ in 0..12 {
        let Some(v) = runner.pending_selection_view() else {
            break;
        };
        let act = if v.selecting_player == 0 && v.is_optional {
            PASS
        } else {
            v.valid_action_ids[0]
        };
        if runner.game.resolve_selection(v.selecting_player, act).is_err() {
            break;
        }
    }
}

/// Accept the currently-pending optional gate (the first non-PASS action), so a
/// "you may" clause under test actually runs its body.
fn accept_optional(runner: &mut DebugRunner) {
    let v = runner
        .pending_selection_view()
        .expect("an optional gate must be pending to accept");
    let accept = v
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .unwrap_or(v.valid_action_ids[0]);
    runner
        .game
        .resolve_selection(v.selecting_player, accept)
        .expect("accept the optional gate");
}

/// Fire an OnDigivolve event for `target` (placement helpers do not fire it).
fn enqueue_digivolve_event_for(runner: &mut DebugRunner, target: PermanentHandle) {
    let card = runner.game.players[target.player as usize].battle_area[target.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnDigivolve,
        TriggerSource::Digivolved {
            player: target.player,
            permanent: target,
            card,
            effect_initiated: false,
            dna_origin: false,
        },
    );
    runner.game.drain_effect_queue();
}

/// Fire an OnAttackTargetChange event sourced from `attacker`.
fn enqueue_attack_target_change_for(runner: &mut DebugRunner, attacker: PermanentHandle) {
    let card = runner.game.players[attacker.player as usize].battle_area[attacker.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::OnAttackTargetChange,
        TriggerSource::EventObserved {
            player: attacker.player,
            permanent: attacker,
            card,
            effect_initiated: false,
        },
    );
    runner.game.drain_effect_queue();
}

/// Push a registered card id into player `p`'s trash.
fn push_to_trash(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card in card_data");
    let next = runner.game.next_card_index();
    runner.game.players[p as usize]
        .trash
        .push(digimon_engine::card_source::CardSource::new(data_idx, p, next));
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 1 — Petrification security-trash loop  (signature)
//
// BT24-017 Medusamon hands the OPPONENT Petrification Tokens. The token's
// `[On Deletion]` trashes its controller's (= the opponent's) top security card;
// `trash_top_security` fires the opp-security-removed observers, which chains
// into BT24-018 Styracomon's `[All Turns][OPT]` "delete 1 of their Digimon".
// (effect_context/mod.rs:2416 fire_security_removed_observers;
//  src/cards/tokens/petrification.rs; DCGO BT24_017.cs / BT24_018.cs.)
// ═════════════════════════════════════════════════════════════════════════════

/// Deleting an opponent-controlled Petrification Token trashes the opponent's
/// top security card — the linchpin the engine spins on. (Token vanishes on
/// leaving play, so opp trash gains only the trashed security card: +1.)
#[test]
fn petrification_token_deletion_trashes_opponent_security() {
    let mut runner = DebugRunner::builder()
        .add_card(opp_filler("SEC-A", 2000))
        .add_card(opp_filler("SEC-B", 2000))
        .security(1, &["SEC-A", "SEC-B"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let token = spawn_petrification_token(&mut runner, 1);
    assert_eq!(token_count(&runner, 1), 1, "one token on the opponent");

    let before = snapshot(&runner);
    runner
        .game
        .delete_permanent_with_cause(token, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();
    let after = snapshot(&runner);

    assert_eq!(
        after.security[1],
        before.security[1] - 1,
        "deleting the opponent-controlled token trashes their top security"
    );
    assert_eq!(
        after.trash[1],
        before.trash[1] + 1,
        "opp trash gains the trashed security card only (the token vanishes)"
    );
    assert_eq!(token_count(&runner, 1), 0, "the token left the field");
}

/// Full loop: the token deletion's security trash auto-fires BT24-018 Styracomon's
/// opp-security-removed payoff (optional delete). Accepting it deletes an opponent
/// Digimon — the cross-card chain that defines the deck.
#[test]
fn petrification_token_deletion_chains_into_styracomon_removal() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-018")
        .expect("BT24-018 (Styracomon) in embedded DSL pack")
        .add_card(opp_filler("SEC-A", 2000))
        .add_card(opp_filler("OPP-DIGI", 3000))
        .security(1, &["SEC-A"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, "BT24-018", Some(0));
    runner.place_on_field(1, "OPP-DIGI", None);
    let token = spawn_petrification_token(&mut runner, 1);

    let before = snapshot(&runner);
    runner
        .game
        .delete_permanent_with_cause(token, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();

    let gate = runner
        .pending_selection_view()
        .expect("Styracomon's opp-security-removed gate must install after the security trash");
    assert!(gate.is_optional, "clause is 'you may delete' — declinable");
    runner
        .game
        .resolve_selection(gate.selecting_player, gate.valid_action_ids[0])
        .expect("accept the gate");
    let target = runner
        .pending_selection_view()
        .expect("delete-target selection installs after accept");
    assert_eq!(target.kind, SelectionKind::OppField, "selects an opp Digimon");
    runner
        .game
        .resolve_selection(target.selecting_player, target.valid_action_ids[0])
        .expect("delete the opp Digimon");
    runner.auto_resolve().ok();

    let after = snapshot(&runner);
    assert_eq!(
        after.security[1],
        before.security[1] - 1,
        "token deletion trashed opp security (loop entry)"
    );
    assert_eq!(
        after.field[1],
        before.field[1] - 2,
        "opp loses both the token and the Styracomon-deleted Digimon"
    );
}

/// Unhappy path: with the opponent at ZERO security, deleting the token is a
/// loop no-op — nothing to trash, no opp-security-removed event, Styracomon's
/// payoff never installs, the opponent's Digimon survives.
#[test]
fn petrification_token_deletion_no_security_does_not_chain() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-018")
        .expect("BT24-018 (Styracomon) in embedded DSL pack")
        .add_card(opp_filler("OPP-DIGI", 3000))
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, "BT24-018", Some(0));
    runner.place_on_field(1, "OPP-DIGI", None);
    let token = spawn_petrification_token(&mut runner, 1);
    assert_eq!(runner.security_count(1), 0, "opponent has no security");

    let before = snapshot(&runner);
    runner
        .game
        .delete_permanent_with_cause(token, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();
    let after = snapshot(&runner);

    assert!(
        runner.pending_selection_view().is_none(),
        "no security to trash → no opp-security-removed event → Styracomon gate never installs"
    );
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "only the token leaves; the opponent's Digimon survives"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 2 — Owen digivolve buff + extra attack  (real BT24-082 + real BT24-011)
//
// When one of your Digimon digivolves into a [Reptile]/[Dragonkin], BT24-082
// Owen Dreadnought suspends itself, grants that Digimon +3000 DP for the turn,
// and offers it an immediate optional attack. Gated on the digivolve target's
// trait. (DCGO BT24_082.cs.)
// ═════════════════════════════════════════════════════════════════════════════

/// Happy path: a real [Dragonkin] ally (BT24-011 Cyclonemon) digivolving makes
/// real Owen (BT24-082) suspend, buffs the ally +3000 DP, and the granted attack
/// resolves a real security check (opp security −1).
#[test]
fn owen_digivolve_buffs_dragonkin_ally_and_grants_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-082")
        .expect("BT24-082 (Owen) in embedded DSL pack")
        .dsl_card("BT24-011")
        .expect("BT24-011 (Cyclonemon) in embedded DSL pack")
        .add_card(opp_filler("SEC", 2000))
        .security(1, &["SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let owen = runner.place_on_field(0, "BT24-082", Some(0));
    let ally = runner.place_on_field(0, "BT24-011", Some(0));
    let dp_before = runner.effective_dp(ally).expect("ally DP");
    let sec_before = runner.security_count(1);

    enqueue_digivolve_event_for(&mut runner, ally);
    runner
        .accept_optional_trigger()
        .expect("accept Owen's optional activation");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.players[0].battle_area[owen.index as usize].is_suspended,
        "Owen pays the suspend cost on a Dragonkin digivolve"
    );
    assert_eq!(
        runner.effective_dp(ally).expect("ally DP"),
        dp_before + 3000,
        "the digivolved ally gains +3000 DP"
    );

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Owen installs a may-attack prompt");
    assert!(pending.is_optional, "'may attack' is optional");
    let attack = encode_attack(ally.index as u16, SECURITY_TARGET);
    assert!(
        pending.valid_action_ids.contains(&attack),
        "the buffed ally may attack the opponent"
    );
    runner
        .game
        .resolve_selection(0, attack)
        .expect("resolve the Owen-granted attack");
    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "the granted attack resolves a normal security check"
    );
}

/// Unhappy path: a real NON-Reptile/Dragonkin ally (BT21-024 Cyberdramon —
/// Cyborg) digivolving fails Owen's trait condition — Owen stays unsuspended, no
/// DP buff, no attack prompt. (Opponent given 6 security so Cyberdramon's own
/// ≤5-security clause does not fire and perturb the check.)
#[test]
fn owen_digivolve_does_not_buff_non_trait_ally() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-082")
        .expect("BT24-082 (Owen) in embedded DSL pack")
        .dsl_card("BT21-024")
        .expect("BT21-024 (Cyberdramon) in embedded DSL pack")
        .add_card(opp_filler("SEC", 2000))
        .security(1, &["SEC", "SEC", "SEC", "SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let owen = runner.place_on_field(0, "BT24-082", Some(0));
    let ally = runner.place_on_field(0, "BT21-024", Some(0));
    let dp_before = runner.effective_dp(ally).expect("ally DP");

    enqueue_digivolve_event_for(&mut runner, ally);
    settle(&mut runner);

    assert!(
        !runner.game.players[0].battle_area[owen.index as usize].is_suspended,
        "Owen must not suspend for a non-Reptile/Dragonkin digivolve"
    );
    assert_eq!(
        runner.effective_dp(ally).expect("ally DP"),
        dp_before,
        "no DP buff for a non-trait digivolve"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 3 — Raid target-change → Lamiamon security trash  (real BT24-011 + BT21-025)
//
// A Raid attacker (BT24-011 Cyclonemon) switching its attack target fires
// BT21-025 Lamiamon's [Your Turn][OPT] clause to trash the opponent's top
// security. Gated on having a [Reptile]/[Dragonkin] Digimon on your field.
// (general_rule.pdf §16-22 Raid; DCGO BT21_025.cs / BT24_011.cs.)
// ═════════════════════════════════════════════════════════════════════════════

/// Happy path: with Lamiamon (Dragonkin) on the field, a real Raid attacker
/// (Cyclonemon) target-changing trashes the opponent's top security.
#[test]
fn raid_target_change_with_lamiamon_trashes_opp_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-025")
        .expect("BT21-025 (Lamiamon) in embedded DSL pack")
        .dsl_card("BT24-011")
        .expect("BT24-011 (Cyclonemon, Raid) in embedded DSL pack")
        .add_card(opp_filler("SEC", 2000))
        .security(1, &["SEC", "SEC"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    runner.place_on_field(0, "BT21-025", Some(0));
    let raider = runner.place_on_field(0, "BT24-011", Some(0));
    let sec_before = runner.security_count(1);

    enqueue_attack_target_change_for(&mut runner, raider);

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "the Raid attacker's target change fires Lamiamon's trash-opp-security"
    );
}

/// Unhappy path: with NO [Reptile]/[Dragonkin] Digimon on our field (only a real
/// Cyborg attacker, Cyberdramon, and no Lamiamon), the same target-change event
/// satisfies no clause and trashes nothing.
#[test]
fn raid_target_change_without_trait_permanent_trashes_nothing() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-025")
        .expect("BT21-025 (Lamiamon) in embedded DSL pack — registered but NOT placed")
        .dsl_card("BT21-024")
        .expect("BT21-024 (Cyberdramon) in embedded DSL pack")
        .add_card(opp_filler("SEC", 2000))
        .security(1, &["SEC"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    // Only a non-Reptile/Dragonkin attacker on the field; Lamiamon is absent.
    let raider = runner.place_on_field(0, "BT21-024", Some(0));
    let sec_before = runner.security_count(1);

    enqueue_attack_target_change_for(&mut runner, raider);

    assert_eq!(
        runner.security_count(1),
        sec_before,
        "no Reptile/Dragonkin permanent on field → Lamiamon's clause cannot fire"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 4 — Security removal feeds the memory engine  (the snowball)
//
// A single opponent-security removal pays out the Elizamon/Dimetromon inherited
// "[Your Turn][OPT] gain 1 memory" ONCE PER buried source. A real
// Elizamon→Dimetromon→Lamiamon stack carries two such sources → one security
// removal = +2 memory. (BT24-008 + BT24-012 inherited gain_memory:1.)
// ═════════════════════════════════════════════════════════════════════════════

/// A real digivolution line [BT24-008 Elizamon, BT24-012 Dimetromon, BT21-025
/// Lamiamon] buries two inherited memory sources; one attack that removes an
/// opponent security card pays out both → +2 memory.
#[test]
fn real_stack_snowballs_two_memory_on_one_security_removal() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-008")
        .expect("BT24-008 (Elizamon)")
        .dsl_card("BT24-012")
        .expect("BT24-012 (Dimetromon)")
        .dsl_card("BT21-025")
        .expect("BT21-025 (Lamiamon)")
        .add_card(opp_filler("SEC", 2000))
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    // [Elizamon (L3), Dimetromon (L4), Lamiamon (L5)] — both lower cards are
    // inherited memory sources; Lamiamon is the active attacker on top.
    let attacker = runner.place_stack(0, &["BT24-008", "BT24-012", "BT21-025"]);
    let mem_before = runner.memory();
    let sec_before = runner.security_count(1);

    runner.attack_player(attacker, 1, true);
    runner.auto_resolve().ok();

    assert!(
        runner.security_count(1) < sec_before,
        "the attack must remove an opponent security card"
    );
    assert_eq!(
        runner.memory(),
        mem_before + 2,
        "both buried inherited sources (Elizamon + Dimetromon) gain 1 memory"
    );
}

/// Coupling: the Petrification-token loop is a security-removal source that feeds
/// the memory engine — deleting an opponent token trashes their security, paying
/// out a buried Elizamon inherited (+1 memory). Stack [BT24-008 Elizamon,
/// BT24-017 Medusamon] (Medusamon on top has no memory inherited).
#[test]
fn petrification_token_deletion_feeds_inherited_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-008")
        .expect("BT24-008 (Elizamon)")
        .dsl_card("BT24-017")
        .expect("BT24-017 (Medusamon)")
        .add_card(opp_filler("SEC", 2000))
        .security(1, &["SEC", "SEC"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    runner.place_stack(0, &["BT24-008", "BT24-017"]);
    let token = spawn_petrification_token(&mut runner, 1);

    let mem_before = runner.memory();
    let sec_before = runner.security_count(1);

    runner
        .game
        .delete_permanent_with_cause(token, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();
    runner.auto_resolve().ok();

    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "the token's on-deletion trashes the opponent's top security"
    );
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "the token-driven security removal pays out the buried Elizamon inherited (+1)"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 5 — Lamiamon inherited free-play (≤5000) on opp-security-removed
//
// BT21-025 Lamiamon's inherited: "[All Turns][OPT] when opp security removed, you
// may play 1 [Reptile]/[Dragonkin] Digimon card with 5000 DP or less from your
// hand without paying the cost." Driven via a REAL security removal (exercising
// the inherited dispatch the per-card bt21_025 clause3 test leaves `#[ignore]`d).
// Buried under a real Medusamon (BT24-017); the free-play target / DP-filter
// control are real archetype cards. (DCGO BT21_025.cs.)
// ═════════════════════════════════════════════════════════════════════════════

/// Happy path: a real ≤5000 [Dragonkin] card (BT24-011 Cyclonemon, 5000) in hand
/// is played FREE when a Petrification-token deletion removes opponent security
/// and Lamiamon's inherited (buried under Medusamon) fires. (Token deletion is
/// the clean, combat-free security-removal faucet — itself an on-theme source.)
#[test]
fn lamiamon_inherited_free_plays_small_dragonkin_on_security_removal() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-025")
        .expect("BT21-025 (Lamiamon) — buried inherited source")
        .dsl_card("BT24-017")
        .expect("BT24-017 (Medusamon) — stack top")
        .dsl_card("BT24-011")
        .expect("BT24-011 (Cyclonemon) — the ≤5000 Dragonkin played free")
        .add_card(opp_filler("SEC", 2000))
        .hand(0, &["BT24-011"])
        .security(1, &["SEC", "SEC"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    // [Lamiamon (L5), Medusamon (L6)] — Lamiamon's inherited is buried.
    runner.place_stack(0, &["BT21-025", "BT24-017"]);
    let token = spawn_petrification_token(&mut runner, 1);
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    // Delete the opponent's token → its on-deletion trashes opp security → fires
    // Lamiamon's inherited free-play (optional). Accept it, then play Cyclonemon.
    runner
        .game
        .delete_permanent_with_cause(token, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();
    accept_optional(&mut runner);
    let pick = runner
        .pending_selection_view()
        .expect("the ≤5000 Reptile/Dragonkin hand selection must install");
    runner
        .game
        .resolve_selection(pick.selecting_player, pick.valid_action_ids[0])
        .expect("play Cyclonemon free");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.battle_area_size(0),
        field_before + 1,
        "the ≤5000 Dragonkin (Cyclonemon) is played free onto our field"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "the played card leaves our hand"
    );
}

/// Unhappy path: only a real >5000 [Dragonkin] (BT24-018 Styracomon, 14000) in
/// hand — Lamiamon's inherited "5000 DP or less" filter excludes it, so nothing
/// is played even though the security-removed trigger fired.
#[test]
fn lamiamon_inherited_does_not_free_play_oversized_dragonkin() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-025")
        .expect("BT21-025 (Lamiamon)")
        .dsl_card("BT24-017")
        .expect("BT24-017 (Medusamon)")
        .dsl_card("BT24-018")
        .expect("BT24-018 (Styracomon) — 14000 DP, fails the ≤5000 filter")
        .add_card(opp_filler("SEC", 2000))
        .hand(0, &["BT24-018"])
        .security(1, &["SEC", "SEC"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    runner.place_stack(0, &["BT21-025", "BT24-017"]);
    let token = spawn_petrification_token(&mut runner, 1);
    let hand_before = runner.hand_size(0);
    let field_before = runner.battle_area_size(0);

    runner
        .game
        .delete_permanent_with_cause(token, ReplacementCause::OwnEffect);
    runner.game.drain_effect_queue();
    settle(&mut runner); // accept nothing playable / decline — no eligible card

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "a >5000 Dragonkin is outside the inherited's DP window — not played"
    );
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "no free play occurs when the only hand candidate fails the DP filter"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 6 — EX11-012 token-shield  (gap G-EX11-012-TOKEN-SHIELD-OWN-ONLY: FIXED)
//
// EX11-012 gives Petrification Tokens to the OPPONENT, and its [All Turns] shield
// is "When this would leave, by deleting 1 Token, it doesn't leave." Per DCGO
// (EX11_012.cs: CanSelectPermanentCondition = permanent.IsToken, selectPlayer =
// card.Owner) the deletable token is ANY token regardless of owner — so EX11-012
// survives by eating the opponent's Petrification token it created (which, via
// the token's [On Deletion], also trashes the opponent's security — a nested
// loop). The DSL originally used `select_own_permanent { kind: token }` (own-only)
// so the shield could never fire; fixed to `select_any_permanent` (scans both
// battle areas, controller picks), faithful to DCGO. This test now passes.
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn ex11_012_survives_by_deleting_opponents_petrification_token() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-012")
        .expect("EX11-012 (Medusamon) in embedded DSL pack")
        .add_card(opp_filler("SEC", 2000))
        .security(1, &["SEC", "SEC"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let medusa = runner.place_on_field(0, "EX11-012", None);
    spawn_petrification_token(&mut runner, 1);
    let sec_before = runner.security_count(1);

    runner
        .game
        .delete_permanent_with_cause(medusa, ReplacementCause::OpponentEffect);
    // Accept the optional shield and pick the opponent's token as the cost.
    accept_optional(&mut runner);
    runner.auto_resolve().ok();

    assert_eq!(
        runner.battle_area_size(0),
        1,
        "EX11-012 must survive by deleting the opponent's Petrification token"
    );
    assert_eq!(token_count(&runner, 1), 0, "the opponent's token is consumed");
    assert_eq!(
        runner.security_count(1),
        sec_before - 1,
        "the consumed token's on-deletion trashes the opponent's top security"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 7 — BT24-016 cheat-in chains into its swap AND nets +2 memory
//
// BT24-016's [Hand][Main] (with real Owen on field, a real Elizamon on field, a
// real Dimetromon in trash) digivolves Lamiamon onto the Elizamon, placing the
// Dimetromon as the bottom source. That effect-initiated digivolve fires
// Lamiamon's own [When Digivolving] swap (opp places a hand card as bottom
// security; their top security is trashed). Because the real Elizamon (BT24-008)
// and Dimetromon (BT24-012) are now buried inherited memory sources, the swap's
// security trash pays out BOTH → +2 memory.
//
// Memory accounting: the digivolution itself costs 3 (memory −3) + 2 inherited
// = net −1. (A −2 result would mean only one inherited fired; −3 means none.)
// (DCGO BT24_016.cs.)
// ═════════════════════════════════════════════════════════════════════════════

/// Build a cheat-in board: real Owen (BT24-082) + real Elizamon (BT24-008) on
/// field, real Dimetromon (BT24-012) in trash, BT24-016 in hand; opponent has
/// `opp_sec` security and a hand card. Returns the runner ready to activate.
fn cheat_in_runner() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-016")
        .expect("BT24-016 (Lamiamon)")
        .dsl_card("BT24-082")
        .expect("BT24-082 (Owen Dreadnought)")
        .dsl_card("BT24-008")
        .expect("BT24-008 (Elizamon) — digivolve base + inherited memory")
        .dsl_card("BT24-012")
        .expect("BT24-012 (Dimetromon) — placed source + inherited memory")
        .add_card(opp_filler("OPP-HAND", 2000))
        .add_card(opp_filler("SEC", 2000))
        .hand(0, &["BT24-016"])
        .hand(1, &["OPP-HAND"])
        .security(1, &["SEC", "SEC", "SEC"])
        .deck(0, &["SEC", "SEC"])
        .deck(1, &["SEC", "SEC"])
        .memory(5)
        .start();
    runner.game.turn_count = 1;
    runner.place_on_field(0, "BT24-082", Some(0));
    runner.place_on_field(0, "BT24-008", Some(1));
    push_to_trash(&mut runner, 0, "BT24-012");
    runner
}

/// Drive the cheat-in: activate [Hand][Main], pick the Elizamon, pick the
/// Dimetromon, then settle all chained triggers.
fn drive_cheat_in(runner: &mut DebugRunner) {
    assert!(
        runner.game.activate_hand_main(0, 0),
        "cheat-in must fire (Owen + Elizamon present)"
    );
    let v = runner.pending_selection_view().expect("Elizamon select");
    runner
        .game
        .resolve_selection(v.selecting_player, v.valid_action_ids[0])
        .expect("select Elizamon");
    let v = runner.pending_selection_view().expect("Dimetromon trash-select");
    runner
        .game
        .resolve_selection(v.selecting_player, v.valid_action_ids[0])
        .expect("select Dimetromon");
    runner.game.drain_effect_queue();
    settle(runner);
}

/// Happy path: the cheat-in lands Lamiamon, its chained swap pressures the
/// opponent's security (hand −1, security net 0, trash +1), AND the two buried
/// inherited memory sources fire → net memory −1 (−3 digivolve cost + 2 gain).
#[test]
fn cheat_in_chains_swap_and_nets_two_memory_from_real_inherits() {
    let mut runner = cheat_in_runner();

    let mem_before = runner.memory();
    let opp_hand_before = runner.hand_size(1);
    let opp_sec_before = runner.security_count(1);
    let opp_trash_before = runner.trash_size(1);

    drive_cheat_in(&mut runner);

    // Lamiamon arrived on top of the Elizamon stack.
    let stack = runner
        .game
        .player(0)
        .battle_area
        .iter()
        .find(|p| {
            p.card_sources
                .iter()
                .any(|s| s.card_id(&runner.game.card_data) == "BT24-008")
        })
        .expect("Elizamon permanent still on field");
    assert_eq!(
        stack.top_card().card_id(&runner.game.card_data),
        "BT24-016",
        "Lamiamon must be the top of the Elizamon stack after the cheat-in"
    );
    assert!(
        stack
            .card_sources
            .iter()
            .any(|s| s.card_id(&runner.game.card_data) == "BT24-012"),
        "the Dimetromon must be a digivolution source under the Elizamon stack"
    );

    // The chained [When Digivolving] swap pressured the opponent's security.
    assert_eq!(
        runner.hand_size(1),
        opp_hand_before - 1,
        "the chained swap makes the opponent place a hand card as bottom security"
    );
    assert_eq!(
        runner.security_count(1),
        opp_sec_before,
        "net opponent security unchanged (added bottom, trashed top)"
    );
    assert_eq!(
        runner.trash_size(1),
        opp_trash_before + 1,
        "the opponent's top security is trashed by the chained swap"
    );

    // Both buried inherited memory sources fired off that security trash.
    assert_eq!(
        runner.memory(),
        mem_before - 1,
        "memory nets −1 (−3 digivolve cost + 2 from the Elizamon and Dimetromon \
         inherited); −2 would mean only one inherited fired, −3 none"
    );
}

/// Unhappy path: without Owen Dreadnought the cheat-in is never offered, so the
/// whole ramp-plus-swap-plus-memory chain is gated off — Lamiamon stays in hand
/// and the opponent's security/hand and our memory are untouched.
#[test]
fn cheat_in_blocked_without_owen_silences_the_chain() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-016")
        .expect("BT24-016 (Lamiamon)")
        .dsl_card("BT24-008")
        .expect("BT24-008 (Elizamon)")
        .dsl_card("BT24-012")
        .expect("BT24-012 (Dimetromon)")
        .add_card(opp_filler("OPP-HAND", 2000))
        .add_card(opp_filler("SEC", 2000))
        .hand(0, &["BT24-016"])
        .hand(1, &["OPP-HAND"])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(5)
        .start();
    runner.game.turn_count = 1;
    // Elizamon + Dimetromon present, but NO Owen Dreadnought.
    runner.place_on_field(0, "BT24-008", Some(0));
    push_to_trash(&mut runner, 0, "BT24-012");

    let mem_before = runner.memory();
    let opp_hand_before = runner.hand_size(1);
    let opp_sec_before = runner.security_count(1);
    let hand_before = runner.hand_size(0);

    assert!(
        !runner.game.activate_hand_main(0, 0),
        "the cheat-in must not fire without Owen Dreadnought"
    );
    settle(&mut runner);

    assert_eq!(runner.hand_size(0), hand_before, "Lamiamon stays in hand");
    assert_eq!(
        runner.hand_size(1),
        opp_hand_before,
        "no security swap chains without the cheat-in"
    );
    assert_eq!(runner.security_count(1), opp_sec_before, "opponent security untouched");
    assert_eq!(runner.memory(), mem_before, "no inherited memory gained");
}

// ═════════════════════════════════════════════════════════════════════════════
// Combo 8 — Lamiamon (BT24-016) security swap feeds the memory engine
//
// With a real Elizamon (BT24-008) buried directly under BT24-016 Lamiamon, the
// swap's incidental top-security trash pays out the Elizamon inherited (+1
// memory): the on-attack security pressure is another faucet into the ramp.
// ═════════════════════════════════════════════════════════════════════════════

/// Stack [BT24-008 Elizamon, BT24-016 Lamiamon]; firing Lamiamon's [When
/// Attacking] swap trashes the opponent's top security, paying out the buried
/// Elizamon inherited (+1 memory).
#[test]
fn lamiamon_security_swap_feeds_inherited_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-016")
        .expect("BT24-016 (Lamiamon)")
        .dsl_card("BT24-008")
        .expect("BT24-008 (Elizamon)")
        .add_card(opp_filler("OPP-HAND", 2000))
        .add_card(opp_filler("SEC", 2000))
        .hand(1, &["OPP-HAND"])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    // Elizamon buried directly under the Lamiamon that fires the swap.
    let lamiamon = runner.place_stack(0, &["BT24-008", "BT24-016"]);

    let mem_before = runner.memory();
    let opp_trash_before = runner.trash_size(1);

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(lamiamon),
    );
    runner.game.drain_effect_queue();
    settle(&mut runner);

    assert_eq!(
        runner.trash_size(1),
        opp_trash_before + 1,
        "the swap trashes the opponent's top security"
    );
    assert_eq!(
        runner.memory(),
        mem_before + 1,
        "the trashed security pays out the buried Elizamon inherited (+1 memory)"
    );
}
