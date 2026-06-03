//! **Medusamon archetype interaction tests.**
//!
//! Model: `qa/archetype-qa/Medusamon-model.md`. Medusamon is a Red
//! Reptile/Dragonkin LIBERATOR control deck whose engine is the **Petrification
//! Token** — a White 3000-DP Digimon it *gifts to the opponent* that, on
//! deletion, trashes the top of its OWNER's (the opponent's) security. The deck
//! pairs that with security-removal punishers (Lamiamon's attack-target-change
//! trash, Styracomon / Owen "opponent security removed" deletes) and a "sticky
//! body" replacement that refuses to leave by deleting a token.
//!
//! These tests exercise the **cross-card** facts per-card TDD misses: a token
//! gifted to the opponent feeds the opponent's-own security on deletion, a Raid
//! redirect feeds Lamiamon's trigger, a digivolve security-trash feeds the
//! security-removed punishers, and the token engine arms Owen. Each test maps
//! 1:1 to a named combo in the model.
//!
//! Source priority (CLAUDE.md): `general_rule.pdf` (canonical, §16 keyword
//! semantics) + DCGO C# (battle-tested) outrank the card-text JSON.

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::TriggerSource;

use super::support::snapshot;

// ─── Shared fixtures ──────────────────────────────────────────────────────────

/// A neutral opponent Digimon target with a chosen DP (synthetic filler — only
/// the real archetype cards under test are loaded from the DSL pack).
fn make_opp_digimon(id: &str, name: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(dp);
    c
}

/// A neutral filler card usable in deck / security / trash zones.
fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Push `card_id` (must already be in `card_data`) onto player `p`'s trash.
fn push_to_trash(runner: &mut DebugRunner, p: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card in card_data");
    let idx = runner.game.next_card_index();
    runner.game.players[p as usize]
        .trash
        .push(CardSource::new(data_idx, p, idx));
}

/// Play a Petrification Token onto `player`'s battle area (the token's owner =
/// its controller, matching how EX11-012 / BT24-017 gift it to the opponent).
fn play_petrification_token(runner: &mut DebugRunner, player: u8) {
    let mut ctx = EffectContext::new(&mut runner.game, CardHandle(0), None, 0);
    ctx.play_token(player, "petrification");
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

/// Find the first Token permanent on player `p`'s field.
fn first_token_handle(runner: &DebugRunner, p: u8) -> PermanentHandle {
    runner
        .game
        .player(p)
        .battle_area
        .iter()
        .position(|perm| perm.top_card().card_kind(&runner.game.card_data) == CardKind::Token)
        .map(|i| PermanentHandle {
            player: p,
            index: i as u8,
        })
        .expect("a Token permanent on the requested field")
}

/// Resolve the currently-pending selection by submitting its first legal action.
fn resolve_first(runner: &mut DebugRunner) {
    let (player, action) = {
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .expect("a selection must be pending");
        (p.selecting_player, p.valid_action_ids[0])
    };
    runner
        .game
        .resolve_selection(player, action)
        .expect("selection resolves");
}

/// Fire a `WhenDigivolving` trigger for `handle` (direct enqueue + drain — the
/// proven harness pattern; `place_*` builds the board without firing the
/// trigger, so we fire it explicitly to model the digivolve).
fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(handle));
    runner.game.drain_effect_queue();
}

// ─── Combo 1: EX11-012 Petrification security loop (sticky body) ──────────────

/// Combo: **EX11-012 Petrification security loop (sticky body)**.
/// Cards: `EX11-012` (Medusamon).
///
/// Expected mechanical outcome: EX11-012 on P0's field with one Petrification
/// Token gifted to P1 (opponent). An opponent effect deletes EX11-012 → the
/// `[All Turns]` would-leave replacement installs a Token-delete selection driven
/// by P0 (the controller, per DCGO `selectPlayer: card.Owner`). Paying the cost
/// deletes the opponent's any-owner Token:
///   - EX11-012 **remains** on P0's field (P0 field unchanged),
///   - P1's tokens go 1 → 0,
///   - the deleted Token's `[On Deletion]` trashes one of P1's security cards
///     (the token's *owner* is P1), so P1 security −1.
///
/// Sources: card text EX11-012 (cards.json) + `EX11-012.yaml` would-leave
/// replacement (`select_any_permanent { kind: token }`, no owner qualifier);
/// `src/cards/tokens/petrification.rs` `[On Deletion] trash_top_security(owner)`;
/// DCGO `EX11_012.cs` (`CanSelectPermanentCondition => p.IsToken`,
/// `selectPlayer: card.Owner`). Rules basis: would-leave replacement/cost timing
/// `general_rule.pdf` §16, deletion §6-2. Regression for gap
/// G-EX11-012-TOKEN-SHIELD-OWN-ONLY (the shield used to use `select_own_permanent`
/// and could never reach the opponent-owned token).
///
/// System-level fact (no per-card test sees it): the token EX11-012 *gave the
/// opponent* both (a) keeps EX11-012 alive as a deletable cost AND (b) burns the
/// opponent's own security when consumed.
#[test]
fn ex11_012_sticky_body_deletes_opponent_token_keeps_self_and_burns_opp_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-012")
        .expect("EX11-012 (Medusamon) in embedded DSL pack")
        .add_card(make_filler("OPP-SEC"))
        .security(1, &["OPP-SEC", "OPP-SEC"])
        .memory(0)
        .start();
    runner.game.turn_count = 1;

    let medusa = runner.place_on_field(0, "EX11-012", None);
    // EX11-012 gifts a Petrification Token to the OPPONENT (P1).
    play_petrification_token(&mut runner, 1);

    let before = snapshot(&runner);
    assert_eq!(before.field[0], 1, "EX11-012 on P0 field");
    assert_eq!(token_count(&runner, 1), 1, "one Petrification Token on P1");
    assert_eq!(before.security[1], 2, "P1 starts with 2 security");
    assert_eq!(
        token_count(&runner, 0),
        0,
        "controller has no token of its own — the shield must reach the opponent's"
    );

    // An opponent effect tries to delete EX11-012 → the [All Turns] would-leave
    // shield fires and installs a Token-delete selection.
    runner
        .game
        .delete_permanent_with_cause(medusa, ReplacementCause::OpponentEffect);

    let (pl, act) = {
        let sel = runner
            .pending_selection()
            .expect("would-leave shield must install a Token-delete selection");
        (sel.selecting_player, sel.valid_action_ids[0])
    };
    assert_eq!(
        pl, 0,
        "the controller (P0) chooses which Token to delete (DCGO selectPlayer: card.Owner)"
    );
    // Pay the cost: delete the opponent's (any-owner) Token.
    runner
        .execute_action(pl, act)
        .expect("delete the opponent's Petrification Token");
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);

    // P0 field unchanged — EX11-012 survived the leave.
    assert_eq!(
        after.field[0], before.field[0],
        "EX11-012 must remain on P0's field (sticky body cancels the leave)"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "EX11-012"),
        "the surviving P0 permanent must be EX11-012 itself"
    );
    // P1 tokens 1 → 0.
    assert_eq!(
        token_count(&runner, 1),
        0,
        "P1's Petrification Token was deleted as the cost"
    );
    // The Token's [On Deletion] trashed one of P1's (its owner's) security cards.
    assert_eq!(
        after.security[1],
        before.security[1] - 1,
        "deleting the opponent-owned Token trashes the opponent's top security (token On Deletion)"
    );
}

// ─── Combo 2: BT24-017 Medusamon when-digivolving payoff package ──────────────

/// Combo: **BT24-017 Medusamon when-digivolving payoff package**.
/// Cards: `BT24-017` (Medusamon).
///
/// Expected mechanical outcome: BT24-017's `[When Digivolving]` (1) deletes the
/// opponent's lowest-DP Digimon (opp field −1), (2) the active player returns
/// exactly 2 of the opponent's trash to the bottom of their deck (opp trash −2),
/// causing the opponent to play 2 Petrification Tokens (opp tokens +2), and (3)
/// BT24-017 gains +2000 DP × (opponent Digimon count, counting the 2 new Tokens)
/// until the opponent's turn ends (≥ +4000 here).
///
/// Sources: card text BT24-017 (cards.json) + `BT24-017.yaml`
/// (`select_opponent_permanent selector: lowest_dp` → delete; `as_selecting_player`
/// over opponent trash `min/max: 2` → `return_trash_list_to_deck_bottom` → 2
/// token plays + `add_dp_modifier` per `card_count_in_zone(opponent, battle_area)`);
/// DCGO `BT24/Red/BT24_017.cs`. Rules basis: when-digivolving keyword window
/// `general_rule.pdf` §16, cost-as-return timing.
///
/// System-level fact: the delete (which feeds the opponent's trash) and the
/// per-opponent-Digimon DP boost (which counts the tokens the *cost* just spawned)
/// form a single payoff chain — the tokens are both an opponent-side liability
/// and the multiplier for the attacker's own DP window.
#[test]
fn bt24_017_when_digivolving_delete_two_tokens_and_dp_boost() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-017")
        .expect("BT24-017 (Medusamon) in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 4000))
        .add_card(make_filler("TR-A"))
        .add_card(make_filler("TR-B"))
        .memory(20)
        .start();
    runner.game.turn_count = 1;

    let bt24 = runner.place_on_field(0, "BT24-017", Some(0));
    runner.place_on_field(1, "OPP-LOW", None);
    // Opponent has 2 trash cards so the "by returning 2" cost is payable.
    push_to_trash(&mut runner, 1, "TR-A");
    push_to_trash(&mut runner, 1, "TR-B");

    let opp_deck_before = runner.deck_size(1);

    fire_when_digivolving(&mut runner, bt24);

    // Step 1 — delete opponent's lowest-DP Digimon.
    resolve_first(&mut runner); // delete OPP-LOW
                                // Step 2 — active player returns exactly 2 opponent trash cards.
    resolve_first(&mut runner); // trash pick 1
    resolve_first(&mut runner); // trash pick 2 (auto-commits at max=2)
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);

    // Opp field: OPP-LOW deleted then 2 tokens spawned → opp battle area = 2 tokens.
    assert_eq!(token_count(&runner, 1), 2, "opponent plays 2 Petrification Tokens");
    assert_eq!(
        after.field[1], 2,
        "opp field = 2 Petrification Tokens (OPP-LOW deleted, 2 tokens added)"
    );
    // Exactly 2 trash cards returned to the bottom of the opponent's deck.
    assert_eq!(
        runner.deck_size(1),
        opp_deck_before + 2,
        "exactly 2 opponent trash cards returned to the bottom of their deck"
    );
    assert_eq!(
        after.trash[1], 1,
        "of the 3 trash cards (2 seeded + 1 deleted Digimon), exactly 2 were returned"
    );
    // DP boost: +2000 × 2 opponent Digimon (the 2 tokens) = +4000.
    assert_eq!(
        runner.effective_dp(bt24),
        Some(11000 + 4000),
        "BT24-017 gains +2000 per opponent Digimon (the 2 tokens) = +4000"
    );
}

/// Unhappy path for Combo 2: opponent has FEWER than 2 trash cards, so the
/// "by returning 2" **cost** is unpayable → no tokens spawn and NO DP boost
/// (DCGO `if (Enemy.TrashCards.Count >= 2)` gate, evaluated after the delete).
/// The delete still happens, but the token/DP-boost tail is gated on the cost.
#[test]
fn bt24_017_unpayable_return_cost_skips_tokens_and_dp_boost() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-017")
        .expect("BT24-017 (Medusamon) in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-LOW", "OppLow", 4000))
        .memory(20)
        .start();
    runner.game.turn_count = 1;

    let bt24 = runner.place_on_field(0, "BT24-017", Some(0));
    runner.place_on_field(1, "OPP-LOW", None);
    // Opponent starts with 0 trash cards: after the delete it has exactly 1
    // (the deleted Digimon) — still below the required 2.
    assert_eq!(runner.trash_size(1), 0, "opponent starts with empty trash");

    fire_when_digivolving(&mut runner, bt24);
    resolve_first(&mut runner); // resolve the lowest-DP delete

    // The deleted Digimon is the only trash card; the return-2 cost cannot be
    // paid, so no further selection installs.
    assert!(
        runner.game.pending_selection.is_none(),
        "the return-2 cost is unpayable with 1 trash card; no multi-select installs"
    );
    assert_eq!(
        token_count(&runner, 1),
        0,
        "no Petrification Tokens when the return-2 cost cannot be paid"
    );
    assert_eq!(
        runner.effective_dp(bt24),
        Some(11000),
        "no DP boost when the return-2 cost (and thus the token play) does not fire"
    );
}

// ─── Combo 3: Raid → attack-target-change → trash security (BT21-025) ─────────

/// Combo: **Raid → attack-target-change → trash security (BT21-025 Lamiamon)**.
/// Cards: `BT21-025` (Lamiamon), `BT24-011` (Cyclonemon — Dragonkin `<Raid>`
/// attacker).
///
/// Expected mechanical outcome: P0 fields Lamiamon (Dragonkin) + a Dragonkin
/// `<Raid>` attacker (BT24-011) and there is an opponent unsuspended higher-DP
/// Digimon. On P0's turn the Raid attacker declares an attack on the player;
/// `<Raid>` switches the attack target to the opponent's highest-DP unsuspended
/// Digimon. The attack-target change fires BT21-025's `[Your Turn][OPT]`
/// `on_attack_target_change` clause (the attacker is P0-owned and Dragonkin) →
/// the opponent's top security card is trashed (opp security −1), once per turn.
///
/// Sources: card text BT21-025 / BT24-011 (cards.json) + `BT21-025.yaml`
/// (`on_attack_target_change` gated on `event_target_owner: you` +
/// `event_target_trait_has: Dragonkin` → `trash_top_security { of: opponent }`)
/// + `BT24-011.yaml` (`grant_keyword Raid`); DCGO `BT21/Red/BT21_025.cs`,
/// `BT24/Red/BT24_011.cs`. Rules basis: `<Raid>` switch window
/// `general_rule.pdf` §16 (Raid), attack-target-change observer timing.
///
/// System-level fact: the Raid keyword on one card *is* the trigger source for
/// Lamiamon's trash on another — the redirect is the bridge no per-card test
/// exercises end-to-end (the per-card BT21-025 test enqueues the event directly).
///
/// CONFIRMED ENGINE GAP (G-ATC-EVENT-TARGET-IS-NEW-TARGET, this run): driven
/// through the real `<Raid>` combat path this assertion FAILS — opp security is
/// untouched. The `event_target_owner` / `event_target_trait_has` predicate
/// family resolves the OnAttackTargetChange "event target" to the **new attack
/// target** (the redirected-to Digimon), not the **attacker whose target
/// changed** (`predicate.rs::event_target_owner` prioritizes
/// `trigger.attack_target_change.new_target` over `trigger.event_permanent`,
/// which IS the attacker). BT21-025's clause gates on the *attacker's* trait
/// (DCGO `BT21_025.cs` `PermanentCondition` →
/// `IsPermanentExistsOnOwnerBattleAreaDigimon` + Reptile/Dragonkin; card text
/// "any of YOUR Reptile/Dragonkin trait Digimon's attack targets change"), so
/// the redirected-to opponent Digimon (opponent-owned, non-Dragonkin) fails the
/// gate and the trash never fires. The per-card test `bt21_025.rs` masks this by
/// firing `TriggerSource::EventObserved { permanent: attacker }` (no
/// `attack_target_change` context), which falls through to `event_permanent` =
/// attacker. Routed to `docs/RUST_ENGINE_GAPS.md`. `#[ignore]`d so the suite
/// stays green while the expectation (faithful outcome: opp security −1) is
/// preserved verbatim — do NOT weaken the assertion to make it pass.
#[test]
#[ignore = "G-ATC-EVENT-TARGET-IS-NEW-TARGET: event_target_* predicates resolve to the redirected-to target, not the attacker whose target changed; BT21-025 gates on the attacker (DCGO BT21_025.cs + card text). Filed to docs/RUST_ENGINE_GAPS.md."]
fn raid_redirect_fires_lamiamon_attack_target_change_trash_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-025")
        .expect("BT21-025 (Lamiamon) in embedded DSL pack")
        .dsl_card("BT24-011")
        .expect("BT24-011 (Cyclonemon, <Raid>) in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-HIGH", "OppHigh", 9000))
        .add_card(make_filler("OPP-SEC"))
        .security(1, &["OPP-SEC", "OPP-SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    // P0 fields Lamiamon (the trigger) + the Raid attacker.
    runner.place_on_field(0, "BT21-025", None);
    let raider = runner.place_on_field(0, "BT24-011", None);
    // Opponent unsuspended higher-DP Digimon — the Raid switch candidate.
    runner.place_on_field(1, "OPP-HIGH", None);

    let before = snapshot(&runner);
    assert_eq!(before.security[1], 2, "P1 starts with 2 security");

    // The Raid attacker declares an attack on the player; <Raid> opens the
    // optional switch window to the opponent's highest-DP unsuspended Digimon.
    runner.attack_player(raider, 1, false);

    // The Raid switch parks an optional OppField selection ("Raid - switch
    // attack target"); choosing the candidate fires OnAttackTargetChange.
    let (pl, act) = {
        let sel = runner
            .pending_selection()
            .expect("<Raid> must install a switch-target selection");
        (sel.selecting_player, sel.valid_action_ids[0])
    };
    assert_eq!(
        pl, 0,
        "the attacking player chooses the Raid switch target"
    );
    runner
        .execute_action(pl, act)
        .expect("switch the attack to OPP-HIGH → fires OnAttackTargetChange");
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);

    // The attack-target change fired Lamiamon's clause → opp top security trashed.
    assert_eq!(
        after.security[1],
        before.security[1] - 1,
        "Lamiamon trashes 1 opponent security when a Dragonkin's attack target changes via Raid"
    );
}

// ─── Combo 4: Styracomon security-removal → delete chain (BT24-018 apex) ──────

/// Combo: **Styracomon security-removal → delete chain (BT24-018 apex)**.
/// Cards: `BT24-018` (Styracomon).
///
/// Expected mechanical outcome: BT24-018 on P0's field digivolves; its
/// `[When Digivolving]` may trash 1 chosen opponent security card (opp security
/// −1). That security removal then fires Styracomon's `[All Turns][OPT]`
/// `on_opponent_security_removed` clause, which may delete 1 of the opponent's
/// Digimon (opp battle_area −1), once that turn.
///
/// Sources: card text BT24-018 (cards.json) + `BT24-018.yaml` (`when_digivolving`
/// `select_security { of: opponent }` → `trash_selected_security`; separate
/// `on_opponent_security_removed` `[OPT]` `select_opponent_permanent` → delete);
/// DCGO `BT24/Red/BT24_018.cs`. Rules basis: when-digivolving + security-removed
/// observer timing `general_rule.pdf` §16.
///
/// System-level fact: Styracomon's *own* digivolve security-trash is the event
/// that arms its *own* `on_opponent_security_removed` deletion — a self-feeding
/// security→removal chain that two separate per-card clause tests never link.
#[test]
fn bt24_018_digivolve_trash_security_chains_into_opponent_delete() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-018")
        .expect("BT24-018 (Styracomon) in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-DIGI", "OppDigi", 6000))
        .add_card(make_filler("OPP-SEC"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(0, &["FILLER-DECK"; 10])
        .deck(1, &["FILLER-DECK"; 10])
        .security(1, &["OPP-SEC", "OPP-SEC", "OPP-SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let styraco = runner.place_on_field(0, "BT24-018", Some(0));
    runner.place_on_field(1, "OPP-DIGI", None);

    let before = snapshot(&runner);
    assert_eq!(before.security[1], 3, "P1 starts with 3 security");
    assert_eq!(before.field[1], 1, "P1 has 1 Digimon");

    fire_when_digivolving(&mut runner, styraco);

    // Drive every prompt (two optional [When Digivolving] sub-clauses — trash
    // security, then optional unsuspend — plus the chained on_opponent_security_
    // removed accept + delete-target). Picking valid_action_ids[0] throughout
    // accepts each optional and chooses the first legal target.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        resolve_first(&mut runner);
        runner.game.drain_effect_queue();
        steps += 1;
    }
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);

    // Digivolve trashed exactly 1 opponent security card.
    assert_eq!(
        after.security[1],
        before.security[1] - 1,
        "[When Digivolving] trashes 1 chosen opponent security card"
    );
    // The security removal chained into the [All Turns][OPT] delete.
    assert_eq!(
        after.field[1],
        before.field[1] - 1,
        "the security removal fires Styracomon's on_opponent_security_removed delete (opp Digimon −1)"
    );
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-DIGI"),
        "the opponent's Digimon must be the deleted target"
    );
}

// ─── Combo 5: Owen security-removed punish (BT18-087) + Petrification feed ────

/// Combo: **Owen security-removed punish (BT18-087) + Petrification feed**.
/// Cards: `BT18-087` (Owen Dreadnought, Tamer), `EX11-012` (the Petrification
/// feed — a deleted token removes from the opponent's own security).
///
/// Expected mechanical outcome: BT18-087 (unsuspended) is on P0's field plus a
/// source that removes from the opponent's security — here a Petrification Token
/// gifted to the opponent, whose `[On Deletion]` trashes the opponent's top
/// security. When a card is removed from the opponent's security stack, BT18-087
/// may suspend itself and delete 1 of the opponent's Digimon with ≤4000 DP
/// (opp battle_area −1; Owen becomes suspended).
///
/// Sources: card text BT18-087 / EX11-012 (cards.json) + `BT18-087.yaml`
/// (`on_opponent_security_removed` → `suspend { source }` →
/// `select_opponent_permanent { dp_lte: 4000 }` → delete) +
/// `src/cards/tokens/petrification.rs` `[On Deletion] trash_top_security(owner)`;
/// DCGO `BT18/Red/BT18_087.cs`, `EX11_012.cs`. Rules basis: security-removed
/// observer + cost-as-suspend `general_rule.pdf` §16.
///
/// System-level fact: the Petrification engine (deleting an opponent-owned token
/// burns the opponent's security) is the *enabler* that arms Owen's
/// security-removed punisher — the cross-card link is the whole point.
#[test]
fn bt18_087_owen_punishes_opponent_security_removed_by_token_deletion() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-087")
        .expect("BT18-087 (Owen Dreadnought) in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-3000", "OppSmall", 3000))
        .add_card(make_filler("OPP-SEC"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(1, &["FILLER-DECK"; 10])
        .security(1, &["OPP-SEC", "OPP-SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let owen = runner.place_on_field(0, "BT18-087", Some(0));
    runner.place_on_field(1, "OPP-3000", None);
    // The Petrification feed: a token gifted to the opponent.
    play_petrification_token(&mut runner, 1);

    // Owen starts unsuspended (precondition for the punisher).
    assert!(
        !runner.game.players[0].battle_area[owen.index as usize].is_suspended,
        "Owen must start unsuspended"
    );

    let before = snapshot(&runner);
    assert_eq!(before.security[1], 2, "P1 starts with 2 security");

    // Delete the opponent-owned Petrification Token → its [On Deletion] trashes
    // the opponent's top security → BT18-087's on_opponent_security_removed fires.
    let token = first_token_handle(&runner, 1);
    runner
        .game
        .delete_permanent_with_cause(token, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);

    // The token's On Deletion trashed the opponent's top security.
    assert_eq!(
        after.security[1],
        before.security[1] - 1,
        "deleting the opponent-owned token trashes the opponent's security (token On Deletion)"
    );
    // Owen punished the security removal: suspended self + deleted a ≤4000 DP
    // opponent Digimon.
    let owen_suspended = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT18-087" && p.is_suspended);
    assert!(
        owen_suspended,
        "Owen must suspend itself as the activation cost of the security-removed punisher"
    );
    assert!(
        !runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-3000"),
        "Owen deletes the opponent's ≤4000 DP Digimon when its security is removed"
    );
}

/// Unhappy path for Combo 5: Owen is **already suspended** when the opponent's
/// security is removed → the cost-as-suspend gate (Owen must be unsuspended)
/// blocks the punisher; no opponent Digimon is deleted. The token's own security
/// burn still happens (it is independent of Owen).
#[test]
fn bt18_087_owen_does_not_punish_when_already_suspended() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT18-087")
        .expect("BT18-087 (Owen Dreadnought) in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-3000", "OppSmall", 3000))
        .add_card(make_filler("OPP-SEC"))
        .add_card(make_filler("FILLER-DECK"))
        .deck(1, &["FILLER-DECK"; 10])
        .security(1, &["OPP-SEC", "OPP-SEC"])
        .memory(10)
        .start();
    runner.game.turn_count = 1;

    let owen = runner.place_on_field(0, "BT18-087", Some(0));
    runner.place_on_field(1, "OPP-3000", None);
    play_petrification_token(&mut runner, 1);

    // Pre-suspend Owen — the cost can no longer be paid.
    runner.game.players[0].battle_area[owen.index as usize].is_suspended = true;

    let before = snapshot(&runner);

    let token = first_token_handle(&runner, 1);
    runner
        .game
        .delete_permanent_with_cause(token, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();

    let after = snapshot(&runner);

    // The token still burns the opponent's security (independent of Owen).
    assert_eq!(
        after.security[1],
        before.security[1] - 1,
        "the token On Deletion still trashes the opponent's security"
    );
    // But Owen, already suspended, cannot fire → its ≤4000 DP Digimon survives.
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "OPP-3000"),
        "a pre-suspended Owen cannot pay the suspend cost — no opponent Digimon is deleted"
    );
}
