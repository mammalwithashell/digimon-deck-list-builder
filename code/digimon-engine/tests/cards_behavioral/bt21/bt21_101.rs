//! BT21-101 Gaiamon — Digimon, Lv.6, White/Red (dual), DP 13000, Play Cost 13.
//! Traits: God, Appmon, Creation. Attribute: God.
//! Official Bandai DB: "Colors: White Red" (per-card JSON card_colors [4, 0]).
//!
//! # Card text (card image — authoritative)
//!
//! Digivolve: standard evo_costs (Lv.5, RED, cost 5) + alt "Ult." trait → cost 5.
//! App Fusion [Globemon] & [Charismon]: Cost 0.
//! <Blocker>
//! <Link +1> (Add 1 to this Digimon's maximum links.)
//! [When Digivolving][When Attacking] You may link 1 Digimon card with the
//!   [Appmon] trait from your hand or this Digimon's digivolution cards to 1 of
//!   your Digimon without paying the cost.
//! [Your Turn][Once Per Turn] When your Digimon get linked, by unsuspending this
//!   Digimon, trash your opponent's top security card.
//!
//! # DCGO C# reference (READ-ONLY)
//! C:/Users/james/Documents/digimon-deck-list-builder-1/DCGO/Assets/Scripts/
//! CardEffect/BT21/White/BT21_101.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - Structural: Blocker keyword grant; Link+1 aura; Ult. alt-digivolve; App Fusion
//! - WD link from hand to chosen own Digimon (facet #9 + ChosenOwnDigimon)
//! - WA link variant (shared clause covering both timings)
//! - Link from digivolution sources
//! - Decline the optional link (PASS)
//! - on_any_link board-wide observer: ANOTHER of your Digimon gets linked →
//!   Gaiamon unsuspends + opponent security -1 (event-log assertion)
//! - Negative: observer does NOT fire when Gaiamon already unsuspended
//! - Negative: observer does NOT fire on opponent's turn
//! - Negative: observer does NOT fire when an OPPONENT Digimon gets linked
//! - OPT lockout: observer fires once, is locked, unlocked after end_turn
//! - Link+1 raises effective max-link cap by 1 (structural)
//!
//! # New DSL vocabulary
//! `on_any_link` timing — board-wide OnLink observer, no forced self/host filter.
//! First card to exercise this timing in tests.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT21-101";

// ─── Helper builders ─────────────────────────────────────────────────────────

fn make_digimon(id: &str, level: u8, dp: i32, cost: u16, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.play_cost = cost;
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn make_appmon(id: &str) -> CardData {
    make_digimon(id, 4, 4000, 4, &["Appmon"])
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Build a standard runner: Gaiamon on P0's field (index 0), memory=10.
fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-101 YAML parses and compiles")
        .add_card(make_appmon("APPMON-HAND"))
        .add_card(make_appmon("APPMON-HAND2"))
        .add_card(make_appmon("APPMON-HOST"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 12])
        .deck(1, &["FILLER"; 12])
        // Give P1 a starting security stack so security_count(1) > 0.
        .security(1, &["FILLER", "FILLER", "FILLER", "FILLER", "FILLER"])
}

/// Push card_id into player `pid`'s hand (direct injection).
fn add_to_hand(runner: &mut DebugRunner, pid: usize, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} not found in card_data"));
    let iid = runner.game.next_card_index();
    let src = CardSource::new(data_idx, pid as u8, iid);
    runner.game.players[pid].hand.push(src);
}

/// Place card_id on player `pid`'s battle area at battle_area slot (auto-assigned
/// by `place_on_field`). Returns the PermanentHandle.
fn place(runner: &mut DebugRunner, pid: usize, card_id: &str) -> PermanentHandle {
    runner.place_on_field(pid as PlayerId, card_id, Some(0u16))
}

/// Manually fire an OnLink trigger as if `card` was just linked onto `host`.
/// This simulates `link_chosen_card_into_host` without going through the full
/// payment flow — used to test the on_any_link observer in isolation.
fn fire_link_event(runner: &mut DebugRunner, host: PermanentHandle, card_id: &str) {
    // Find the card handle for card_id in the game's card_data.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card_id in card_data");
    let iid = runner.game.next_card_index();
    let card_src = CardSource::new(data_idx, host.player as u8, iid);
    let card_handle = card_src.handle();
    // Attach the card as a link on the host.
    if let Some(host_perm) = runner.game.players[host.player as usize]
        .battle_area
        .get_mut(host.index as usize)
    {
        host_perm.linked_cards.push(card_src);
    }
    // Fire OnLink globally (mirrors game_actions/link.rs).
    for pid in 0..runner.game.players.len() {
        runner.game.enqueue_triggered(
            EffectTiming::OnLink,
            TriggerSource::Linked {
                player: pid as PlayerId,
                host,
                card: card_handle,
            },
        );
    }
    runner.game.drain_effect_queue();
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

/// BT21-101 must compile. Smoke: compiled_card returns Some.
#[test]
fn bt21_101_yaml_compiles() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("BT21-101 in DSL pack");
    assert_eq!(card.name, "Gaiamon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.dp, Some(13000));
}

/// Printed color is White/Red DUAL — official Bandai DB "Colors: White Red";
/// per-card JSON card_colors [4, 0] = [White, Red]. Regression for the
/// mono-white drift found in the faithfulness audit.
#[test]
fn bt21_101_is_white_red_dual_color() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    assert_eq!(
        card.color,
        vec![CompiledColor::White, CompiledColor::Red],
        "BT21-101 must be White/Red dual color (official DB: \"Colors: White Red\")"
    );
}

/// Must declare a Blocker keyword grant.
#[test]
fn bt21_101_has_blocker_keyword() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    let has_blocker = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword, ..
            }) if keyword == "Blocker"
        )
    });
    assert!(has_blocker, "BT21-101 must have a Blocker keyword grant");
}

/// Must declare a ChangeLinkMax aura with modifier_value = 1 (Link +1).
#[test]
fn bt21_101_has_link_max_aura() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    let has_aura = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                modifier, modifier_value, ..
            }) if modifier.as_deref() == Some("ChangeLinkMax")
                && *modifier_value == Some(1)
        )
    });
    assert!(
        has_aura,
        "BT21-101 must have a ChangeLinkMax aura with modifier_value=1 (Link+1)"
    );
}

/// Alt-digivolve from "Ult." trait host at cost 5 must be registered.
#[test]
fn bt21_101_registers_ult_alt_digivolve() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::Digivolve)
            && matches!(p.cost, Some(CompiledCost::Literal(5)))
    });
    assert!(
        has,
        "BT21-101 must register a cost-5 alt-digivolve (Ult. host)"
    );
}

/// App Fusion (Globemon & Charismon) at cost 0 must be registered.
#[test]
fn bt21_101_registers_app_fusion() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    let has = card.alt_paths.iter().any(|p| {
        matches!(p.kind, CompiledAltPathKind::AppFusion)
            && matches!(p.cost, Some(CompiledCost::Literal(0)))
    });
    assert!(
        has,
        "BT21-101 must register a cost-0 App Fusion path (Globemon & Charismon)"
    );
}

/// [WD][WA] clause must be a triggered clause covering both timings, optional,
/// once_per_turn: false (it has no OPT tag in printed text).
#[test]
fn bt21_101_wd_wa_clause_shape() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    let wd_wa: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        wd_wa.len(),
        1,
        "exactly 1 triggered clause must cover WhenDigivolving+WhenAttacking"
    );
    let clause = wd_wa[0];
    assert!(
        clause.optional,
        "WD/WA clause must be optional (\"you may\")"
    );
    assert!(
        !clause.once_per_turn,
        "WD/WA clause must NOT be once_per_turn (no [Once Per Turn] tag)"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "WD/WA clause must be FaceUp scope"
    );
}

/// [Your Turn][OPT] on_any_link clause must be a triggered clause with OnAnyLink
/// timing, optional: true, once_per_turn: true.
#[test]
fn bt21_101_on_any_link_clause_shape() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).unwrap();
    let oal: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyLink) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(
        oal.len(),
        1,
        "exactly 1 triggered clause must use OnAnyLink timing"
    );
    let clause = oal[0];
    assert!(
        clause.optional,
        "on_any_link clause must be optional (skippable in DCGO)"
    );
    assert!(
        clause.once_per_turn,
        "on_any_link clause must be once_per_turn ([Once Per Turn])"
    );
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "on_any_link clause must be FaceUp (own-Digimon observer)"
    );
}

// ─── Section 2: [WD] link from hand to chosen own Digimon ────────────────────

/// [WD]: Gaiamon enters field (WhenDigivolving). Hand has an [Appmon] card.
/// Another of your Digimon is on the field. The link card selection + host
/// selection must both install.
#[test]
fn bt21_101_wd_link_from_hand_to_chosen_host() {
    let mut r = base().memory(10).start();
    let gaia = place(&mut r, 0, CARD_ID);
    let host = place(&mut r, 0, "APPMON-HOST"); // the link target
    add_to_hand(&mut r, 0, "APPMON-HAND"); // the card to link

    // Simulate [When Digivolving] by manually enqueuing it.
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(gaia),
    );
    r.game.drain_effect_queue();

    // Expect a selection for the card to link (from hand).
    assert!(
        r.pending_selection().is_some(),
        "[WD] must install a card selection when an Appmon is in hand"
    );
    let sel = r.pending_selection_view().unwrap();
    // Pick the Appmon from hand.
    let _ = r.game.resolve_selection(0, sel.valid_action_ids[0]);

    // Now a selection for the host Digimon must follow (to: chosen_own_digimon).
    assert!(
        r.pending_selection().is_some(),
        "[WD] must install a host selection after picking the link card"
    );
    let host_sel = r.pending_selection_view().unwrap();
    let _ = r.game.resolve_selection(0, host_sel.valid_action_ids[0]);
    let _ = r.auto_resolve();

    // The linked card should now be on one of P0's Digimon.
    let gaia_links = r.game.players[0].battle_area[gaia.index as usize]
        .linked_cards
        .len();
    let host_links = r.game.players[0].battle_area[host.index as usize]
        .linked_cards
        .len();
    assert_eq!(
        gaia_links + host_links,
        1,
        "exactly 1 link card must have been attached to one of P0's Digimon"
    );
}

/// Decline the [WD] optional link — no link card attached, hand unchanged.
#[test]
fn bt21_101_wd_link_decline_does_nothing() {
    let mut r = base().memory(10).start();
    let gaia = place(&mut r, 0, CARD_ID);
    let _host = place(&mut r, 0, "APPMON-HOST");
    add_to_hand(&mut r, 0, "APPMON-HAND");

    let hand_before = r.hand_size(0);

    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(gaia),
    );
    r.game.drain_effect_queue();

    // Decline via PASS.
    assert!(r.pending_selection().is_some(), "must prompt");
    use digimon_engine::action::space::PASS;
    let _ = r.game.resolve_selection(0, PASS);
    let _ = r.auto_resolve();

    // Hand unchanged, no links.
    assert_eq!(
        r.hand_size(0),
        hand_before,
        "declining must leave hand unchanged"
    );
    let all_links: usize = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.linked_cards.len())
        .sum();
    assert_eq!(all_links, 0, "declining must not attach any link card");
}

/// No Appmon in hand and no Appmon in digivolution sources → no selection.
#[test]
fn bt21_101_wd_no_selection_when_no_eligible_appmon() {
    let mut r = base().memory(10).start();
    let gaia = place(&mut r, 0, CARD_ID);
    let _host = place(&mut r, 0, "APPMON-HOST");
    // Add a non-Appmon to hand.
    add_to_hand(&mut r, 0, "FILLER");

    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(gaia),
    );
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();

    assert!(
        r.pending_selection().is_none(),
        "no selection when no eligible Appmon in hand or digivolution sources"
    );
}

// ─── Section 3: [WA] link variant ────────────────────────────────────────────

/// [When Attacking]: same optional link behaviour as WD.
#[test]
fn bt21_101_wa_link_from_hand_to_chosen_host() {
    let mut r = base().memory(10).start();
    let gaia = place(&mut r, 0, CARD_ID);
    let host = place(&mut r, 0, "APPMON-HOST");
    add_to_hand(&mut r, 0, "APPMON-HAND");

    // Fire WhenAttacking trigger for Gaiamon.
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(gaia));
    r.game.drain_effect_queue();

    assert!(
        r.pending_selection().is_some(),
        "[WA] must install a card selection when an Appmon is in hand"
    );
    let sel = r.pending_selection_view().unwrap();
    let _ = r.game.resolve_selection(0, sel.valid_action_ids[0]);

    assert!(
        r.pending_selection().is_some(),
        "[WA] must install a host selection after picking the link card"
    );
    let host_sel = r.pending_selection_view().unwrap();
    let _ = r.game.resolve_selection(0, host_sel.valid_action_ids[0]);
    let _ = r.auto_resolve();

    let total_links: usize = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.linked_cards.len())
        .sum();
    assert_eq!(
        total_links, 1,
        "[WA] must have attached exactly 1 link card"
    );
}

// ─── Section 4: [WD] link from digivolution sources ──────────────────────────

/// [WD]: If Gaiamon has an Appmon in its digivolution sources (and none in hand),
/// the card selection is offered from sources.
#[test]
fn bt21_101_wd_link_from_digisources() {
    let mut r = base().memory(10).start();
    let gaia = place(&mut r, 0, CARD_ID);
    let host = place(&mut r, 0, "APPMON-HOST");
    // Inject an Appmon into Gaiamon's digivolution sources (below Gaiamon as a source).
    // `push_under` inserts at position 0 (bottom); the Gaiamon top card stays at the
    // last position and is excluded from the candidate scan (top_pos = last index).
    {
        let appmon_data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "APPMON-HAND")
            .expect("APPMON-HAND in card_data");
        let iid = r.game.next_card_index();
        let src = CardSource::new(appmon_data_idx, 0, iid);
        r.game.players[0].battle_area[gaia.index as usize].push_under(src);
    }

    // Fire WhenDigivolving.
    r.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(gaia),
    );
    r.game.drain_effect_queue();

    // Expect the card selection (from digivolution sources).
    assert!(
        r.pending_selection().is_some(),
        "[WD] must install a card selection when an Appmon is in digivolution sources"
    );
}

// ─── Section 5: on_any_link observer — positive cases ────────────────────────

/// When an opponent's security card is trashed via the observer, the opponent's
/// security zone shrinks by 1.
#[test]
fn bt21_101_on_any_link_trashes_opponent_security() {
    let mut r = base().memory(10).start();
    let gaia = place(&mut r, 0, CARD_ID);
    let other_own = place(&mut r, 0, "APPMON-HOST"); // P0's second Digimon

    // Suspend Gaiamon (prerequisite for the observer to fire).
    r.game.players[0].battle_area[gaia.index as usize].is_suspended = true;

    let sec_before = r.security_count(1);

    // Fire a link event: APPMON-HAND2 gets linked onto P0's second Digimon.
    fire_link_event(&mut r, other_own, "APPMON-HAND2");

    // Observer should have prompted; accept (unsuspend + trash security).
    if r.pending_selection().is_some() {
        let view = r.pending_selection_view().unwrap();
        let _ = r.game.resolve_selection(0, view.valid_action_ids[0]);
    }
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(1),
        sec_before - 1,
        "opponent's security must shrink by 1 when on_any_link fires"
    );
    // Gaiamon must be unsuspended.
    assert!(
        !r.game.players[0].battle_area[gaia.index as usize].is_suspended,
        "Gaiamon must be unsuspended after the observer body runs"
    );
}

/// Declining the observer (PASS) does NOT trash security and does NOT consume OPT.
#[test]
fn bt21_101_on_any_link_decline_does_not_trash_security() {
    let mut r = base().memory(10).start();
    let gaia = place(&mut r, 0, CARD_ID);
    let other_own = place(&mut r, 0, "APPMON-HOST");

    r.game.players[0].battle_area[gaia.index as usize].is_suspended = true;

    let sec_before = r.security_count(1);

    fire_link_event(&mut r, other_own, "APPMON-HAND2");

    // Decline the observer.
    if r.pending_selection().is_some() {
        use digimon_engine::action::space::PASS;
        let _ = r.game.resolve_selection(0, PASS);
    }
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(1),
        sec_before,
        "declining must NOT trash opponent's security"
    );
}

// ─── Section 6: on_any_link observer — negative cases ────────────────────────

/// Observer must NOT fire when Gaiamon is already unsuspended (condition gate).
#[test]
fn bt21_101_on_any_link_does_not_fire_when_gaiamon_unsuspended() {
    let mut r = base().memory(10).start();
    let gaia = place(&mut r, 0, CARD_ID);
    let other_own = place(&mut r, 0, "APPMON-HOST");

    // Gaiamon starts unsuspended (default after place_on_field — it IS unsuspended
    // when it first enters the field; the condition requires it to be SUSPENDED).
    assert!(
        !r.game.players[0].battle_area[gaia.index as usize].is_suspended,
        "precondition: Gaiamon must be unsuspended"
    );

    let sec_before = r.security_count(1);

    fire_link_event(&mut r, other_own, "APPMON-HAND2");
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(1),
        sec_before,
        "observer must NOT fire when Gaiamon is unsuspended"
    );
    assert!(
        r.pending_selection().is_none(),
        "no selection must install when Gaiamon is unsuspended"
    );
}

/// Observer must NOT fire on the opponent's turn.
#[test]
fn bt21_101_on_any_link_does_not_fire_on_opponent_turn() {
    let mut r = base().memory(10).start();
    let gaia = place(&mut r, 0, CARD_ID);
    let other_own = place(&mut r, 0, "APPMON-HOST");

    r.game.players[0].battle_area[gaia.index as usize].is_suspended = true;

    // Advance to opponent's turn.
    r.end_turn();
    assert_eq!(r.turn_player(), 1, "precondition: must be P1's turn");

    let sec_before = r.security_count(0); // P0 is now the "opponent"

    fire_link_event(&mut r, other_own, "APPMON-HAND2");
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(0),
        sec_before,
        "observer must NOT fire on opponent's turn"
    );
}

/// Observer must NOT fire when an OPPONENT Digimon gets linked (event_target_owner: you).
#[test]
fn bt21_101_on_any_link_does_not_fire_for_opponent_digimon_link() {
    let mut r = base().memory(10).start();
    let gaia = place(&mut r, 0, CARD_ID);
    let opp_digimon = place(&mut r, 1, "APPMON-HOST"); // P1's Digimon

    r.game.players[0].battle_area[gaia.index as usize].is_suspended = true;

    let sec_before = r.security_count(1);

    // Link onto the OPPONENT's Digimon.
    fire_link_event(&mut r, opp_digimon, "APPMON-HAND2");
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(1),
        sec_before,
        "observer must NOT fire when an opponent Digimon (P1's) gets linked"
    );
}

// ─── Section 7: OPT lockout and clear ────────────────────────────────────────

/// Observer fires once, then is locked for the turn.
#[test]
fn bt21_101_on_any_link_opt_lockout() {
    let mut r = base().memory(10).start();
    let gaia = place(&mut r, 0, CARD_ID);
    let host1 = place(&mut r, 0, "APPMON-HOST");
    let host2 = place(&mut r, 0, "APPMON-HAND2");

    r.game.players[0].battle_area[gaia.index as usize].is_suspended = true;

    let sec_before = r.security_count(1);

    // First link — observer fires, accept it.
    fire_link_event(&mut r, host1, "APPMON-HAND");
    if r.pending_selection().is_some() {
        let view = r.pending_selection_view().unwrap();
        let _ = r.game.resolve_selection(0, view.valid_action_ids[0]);
    }
    let _ = r.auto_resolve();

    let sec_after_first = r.security_count(1);
    assert_eq!(
        sec_after_first,
        sec_before - 1,
        "first link must trash 1 security"
    );

    // Re-suspend for the second test (the unsuspend from the first fire re-suspended is needed).
    r.game.players[0].battle_area[gaia.index as usize].is_suspended = true;

    // Second link — observer should be locked (OPT consumed).
    fire_link_event(&mut r, host2, "APPMON-HAND2");
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(1),
        sec_after_first,
        "second link must NOT trash additional security (OPT locked)"
    );
}

/// After end_turn, the OPT counter resets and the observer can fire again next turn.
#[test]
fn bt21_101_on_any_link_opt_resets_after_end_turn() {
    let mut r = base().memory(10).start();
    let gaia = place(&mut r, 0, CARD_ID);
    let other_own = place(&mut r, 0, "APPMON-HOST");

    r.game.players[0].battle_area[gaia.index as usize].is_suspended = true;

    // Fire once on P0's turn.
    fire_link_event(&mut r, other_own, "APPMON-HAND");
    if r.pending_selection().is_some() {
        let view = r.pending_selection_view().unwrap();
        let _ = r.game.resolve_selection(0, view.valid_action_ids[0]);
    }
    let _ = r.auto_resolve();
    let sec_after_t1 = r.security_count(1);

    // Cycle through opponent's turn and back to P0's turn.
    r.end_turn(); // P0 → P1
    r.end_turn(); // P1 → P0

    // Re-suspend Gaiamon for the test.
    r.game.players[0].battle_area[gaia.index as usize].is_suspended = true;

    // Fire again on P0's new turn — should work.
    fire_link_event(&mut r, other_own, "APPMON-HAND2");
    if r.pending_selection().is_some() {
        let view = r.pending_selection_view().unwrap();
        let _ = r.game.resolve_selection(0, view.valid_action_ids[0]);
    }
    let _ = r.auto_resolve();

    assert_eq!(
        r.security_count(1),
        sec_after_t1 - 1,
        "OPT must have reset; second turn link must trash 1 more security"
    );
}
