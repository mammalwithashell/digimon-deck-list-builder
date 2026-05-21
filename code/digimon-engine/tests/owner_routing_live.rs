//! Owner-routing live-coverage harness.
//!
//! Track E's PR #453 made `Game::return_to_hand`, `Game::return_to_deck_inner`,
//! and several `EffectContext` zone helpers route by `CardSource.owner` instead
//! of `PermanentHandle.player` (the controller). The fix is dormant today —
//! no card mechanic produces `owner != controller` — so the existing
//! coverage in `tests/zone_manipulation.rs:2070-2215` directly mutates field
//! state to seed the shape.
//!
//! This harness exercises the same routing surface through `EffectContext`
//! call shapes that real card scripts use, with the `owner != controller`
//! shape installed via the synthetic `DebugRunner::transfer_control`
//! helper. When a future control-transfer card lands, this harness gets
//! migrated to use it directly and the synthetic helper is deprecated.
//!
//! See `.claude/plans/pre-scaling-cleanup-batch.md` §1 for the full plan.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef, StackPosition};
use digimon_engine::permanent::{Permanent, PermanentHandle};

fn plain_digimon(card_id: &str, name: &str) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 1,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

/// Fixture: a 1-source permanent owned by player 0, controlled by player 1
/// (transferred via the synthetic helper). Returns the new handle.
fn fixture_p0_owned_p1_controlled(r: &mut DebugRunner, card_id: &str) -> PermanentHandle {
    let h = r.place_on_field(0, card_id, Some(0));
    r.transfer_control(h, 1)
}

// ─── Sanity: helper preserves CardSource.owner ─────────────────────────────

#[test]
fn transfer_control_preserves_top_card_owner() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ALPHA", "Alpha"))
        .start();
    let h0 = r.place_on_field(0, "ALPHA", Some(0));
    let pre_top_owner = {
        let g = r.game_mut();
        g.player(0).battle_area[0]
            .card_sources
            .last()
            .unwrap()
            .owner
    };
    assert_eq!(pre_top_owner, 0);

    let h1 = r.transfer_control(h0, 1);
    assert_eq!(h1.player, 1, "new handle is on the recipient side");

    let post_top_owner = {
        let g = r.game_mut();
        g.player(1).battle_area[h1.index as usize]
            .card_sources
            .last()
            .unwrap()
            .owner
    };
    assert_eq!(
        post_top_owner, 0,
        "transfer_control must preserve CardSource.owner across the move"
    );
    assert_eq!(r.battle_area_size(0), 0, "source side empty");
    assert_eq!(r.battle_area_size(1), 1, "destination side now hosts");
}

#[test]
fn transfer_control_preserves_every_source_owner_in_multi_source_stack() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base"))
        .add_card(plain_digimon("MID", "Mid"))
        .add_card(plain_digimon("TOP", "Top"))
        .start();
    let h = r.place_stack(0, &["BASE", "MID", "TOP"]);
    let pre_owners: Vec<u8> = {
        let g = r.game_mut();
        g.player(0).battle_area[0]
            .card_sources
            .iter()
            .map(|c| c.owner)
            .collect()
    };
    assert_eq!(pre_owners, vec![0, 0, 0]);

    let h1 = r.transfer_control(h, 1);
    let post_owners: Vec<u8> = {
        let g = r.game_mut();
        g.player(1).battle_area[h1.index as usize]
            .card_sources
            .iter()
            .map(|c| c.owner)
            .collect()
    };
    assert_eq!(
        post_owners, pre_owners,
        "every source's CardSource.owner is preserved through transfer"
    );
}

// ─── EffectContext::return_to_hand ─────────────────────────────────────────

/// `EffectContext::return_to_hand` against a permanent owned by P0 but
/// controlled by P1 returns the top card to **P0's** hand, not P1's.
#[test]
fn effect_return_to_hand_routes_top_card_to_owner_after_control_transfer() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OWNED_BY_P0", "OwnedByP0"))
        .start();
    let h = fixture_p0_owned_p1_controlled(&mut r, "OWNED_BY_P0");

    let p0_hand_before = r.hand_size(0);
    let p1_hand_before = r.hand_size(1);

    let returned = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 1);
        ctx.return_to_hand(h)
    };
    assert!(returned.is_some(), "return_to_hand must succeed");

    assert_eq!(
        r.hand_size(0),
        p0_hand_before + 1,
        "owner's hand grew by exactly 1"
    );
    assert_eq!(
        r.hand_size(1),
        p1_hand_before,
        "controller's hand unchanged"
    );
    assert_eq!(r.battle_area_size(1), 0, "permanent left the field");
}

/// In a transferred multi-source stack, **each** below-top source returns
/// to **its own** owner's trash, not the controller's. Builds a stack
/// with mixed owners, transfers control, calls `return_to_hand`, and
/// asserts per-source routing.
#[test]
fn effect_return_to_hand_routes_below_top_sources_to_each_owners_trash() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base"))
        .add_card(plain_digimon("MID", "Mid"))
        .add_card(plain_digimon("TOP", "Top"))
        .start();
    // Initial: P0 owns + controls a single-source TOP.
    let h0 = r.place_on_field(0, "TOP", Some(0));
    // Build mixed-owner stack: BASE owned by P1, MID owned by P0, TOP owned by P0.
    let _ = r.push_source_owned(h0, "BASE", /* owner */ 1);
    let _ = r.push_source_owned(h0, "MID", /* owner */ 0);
    // Transfer control to P1 (controller now P1; owners stay P1, P0, P0).
    let h1 = r.transfer_control(h0, 1);

    let p0_hand_before = r.hand_size(0);
    let p0_trash_before = r.trash_size(0);
    let p1_trash_before = r.trash_size(1);

    let returned = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 1);
        ctx.return_to_hand(h1)
    };
    assert!(returned.is_some());

    assert_eq!(
        r.hand_size(0),
        p0_hand_before + 1,
        "TOP (owner=P0) returns to P0's hand"
    );
    assert_eq!(
        r.trash_size(0),
        p0_trash_before + 1,
        "MID (owner=P0) goes to P0's trash"
    );
    assert_eq!(
        r.trash_size(1),
        p1_trash_before + 1,
        "BASE (owner=P1) goes to P1's trash"
    );
}

// ─── EffectContext::return_to_deck (top + bottom) ──────────────────────────

#[test]
fn effect_return_to_deck_top_routes_top_card_to_owners_deck_top() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OWNED_BY_P0", "OwnedByP0"))
        .start();
    let h = fixture_p0_owned_p1_controlled(&mut r, "OWNED_BY_P0");
    let p0_deck_before = r.deck_size(0);
    let p1_deck_before = r.deck_size(1);

    let ok = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 1);
        ctx.return_to_deck(h, StackPosition::Top)
    };
    assert!(ok);

    assert_eq!(
        r.deck_size(0),
        p0_deck_before + 1,
        "owner's deck grew by 1 (Top position)"
    );
    assert_eq!(
        r.deck_size(1),
        p1_deck_before,
        "controller's deck unchanged"
    );
    let top_owner = {
        let g = r.game_mut();
        g.player(0).deck.last().unwrap().owner
    };
    assert_eq!(top_owner, 0, "deck-top card retains its owner");
}

#[test]
fn effect_return_to_deck_bottom_routes_top_card_to_owners_deck_bottom() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OWNED_BY_P0", "OwnedByP0"))
        .start();
    let h = fixture_p0_owned_p1_controlled(&mut r, "OWNED_BY_P0");
    let p0_deck_before = r.deck_size(0);
    let p1_deck_before = r.deck_size(1);

    let ok = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 1);
        ctx.return_to_deck(h, StackPosition::Bottom)
    };
    assert!(ok);

    assert_eq!(r.deck_size(0), p0_deck_before + 1);
    assert_eq!(r.deck_size(1), p1_deck_before);
    // Deck-bottom inserted at index 0.
    let bottom_owner = {
        let g = r.game_mut();
        g.player(0).deck[0].owner
    };
    assert_eq!(bottom_owner, 0);
}

#[test]
fn effect_return_to_deck_routes_below_top_sources_to_each_owners_trash() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base"))
        .add_card(plain_digimon("MID", "Mid"))
        .add_card(plain_digimon("TOP", "Top"))
        .start();
    let h0 = r.place_on_field(0, "TOP", Some(0));
    let _ = r.push_source_owned(h0, "BASE", /* owner */ 1);
    let _ = r.push_source_owned(h0, "MID", /* owner */ 0);
    let h1 = r.transfer_control(h0, 1);

    let p0_deck_before = r.deck_size(0);
    let p0_trash_before = r.trash_size(0);
    let p1_trash_before = r.trash_size(1);

    let ok = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 1);
        ctx.return_to_deck(h1, StackPosition::Bottom)
    };
    assert!(ok);

    assert_eq!(
        r.deck_size(0),
        p0_deck_before + 1,
        "TOP returns to owner's deck"
    );
    assert_eq!(
        r.trash_size(0),
        p0_trash_before + 1,
        "MID (owner P0) → P0's trash"
    );
    assert_eq!(
        r.trash_size(1),
        p1_trash_before + 1,
        "BASE (owner P1) → P1's trash"
    );
}

// ─── EffectContext::bounce_self ────────────────────────────────────────────

/// `bounce_self` is sugar over `return_to_hand(self.source_permanent)` and
/// must inherit the same owner-routing rule.
#[test]
fn effect_bounce_self_routes_to_owner_when_source_was_transferred() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OWNED_BY_P0", "OwnedByP0"))
        .start();
    let h = fixture_p0_owned_p1_controlled(&mut r, "OWNED_BY_P0");

    let p0_hand_before = r.hand_size(0);
    let p1_hand_before = r.hand_size(1);

    let returned = {
        // EffectContext is bound to the transferred permanent as the
        // resolving source — `bounce_self` reads `self.source_permanent`.
        let card_handle = r.game_mut().player(1).battle_area[h.index as usize]
            .card_sources
            .last()
            .unwrap()
            .handle();
        let mut ctx = EffectContext::new(
            r.game_mut(),
            card_handle,
            Some(h),
            /* effect_player */ 1,
        );
        ctx.bounce_self()
    };
    assert!(returned.is_some(), "bounce_self must succeed");

    assert_eq!(
        r.hand_size(0),
        p0_hand_before + 1,
        "bounce_self routes to OWNER's hand (P0), not the controller's (P1)"
    );
    assert_eq!(r.hand_size(1), p1_hand_before);
}

// ─── EffectContext::trash_top_source ───────────────────────────────────────

/// `trash_top_source` peels the top digivolution source off a stack and
/// routes it by **owner**, distinct from full-permanent `delete_permanent`
/// which routes by controller (per `Player::delete_permanent`).
#[test]
fn effect_trash_top_source_routes_to_source_owner_after_control_transfer() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base"))
        .add_card(plain_digimon("TOP", "Top"))
        .start();
    let h0 = r.place_on_field(0, "TOP", Some(0));
    let _ = r.push_source_owned(h0, "BASE", /* owner */ 0);
    let h1 = r.transfer_control(h0, 1);
    // Stack now: BASE(owner=P0) ← TOP(owner=P0) (under controller P1).

    let p0_trash_before = r.trash_size(0);
    let p1_trash_before = r.trash_size(1);

    let ok = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 1);
        // trash_top_source pops TOP (owner P0).
        ctx.trash_top_source(h1)
    };
    assert!(ok, "trash_top_source must succeed");

    assert_eq!(
        r.trash_size(0),
        p0_trash_before + 1,
        "TOP routes to its OWNER's trash (P0), not the controller's (P1)"
    );
    assert_eq!(
        r.trash_size(1),
        p1_trash_before,
        "controller's trash unchanged"
    );
    assert_eq!(
        r.game_mut().player(1).battle_area[h1.index as usize]
            .card_sources
            .len(),
        1,
        "stack popped down to BASE only"
    );
}

// ─── delete_permanent — negative case (controller-routed by design) ───────

/// **Negative** owner-routing: `Game::delete_permanent_with_effects` /
/// `Player::delete_permanent` route the deleted stack to the **controller's**
/// trash, NOT the owner's. This is the intentional v1 behavior; verifying
/// it here pins the asymmetry against accidental "fixes".
///
/// Rationale: the deletion path (`Player::delete_permanent`) is shared
/// between rule-driven battle deletion and effect-driven deletion; the
/// printed "Digimon is deleted → trash" rule hits the controller's zone.
/// The owner-routed helpers (`return_to_hand`, `return_to_deck`,
/// `trash_top_source`) are zone-movement primitives, not deletion. If a
/// future engine change unifies them, update both this test and the
/// affected helpers in lockstep.
#[test]
fn delete_permanent_routes_to_controllers_trash_not_owners() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OWNED_BY_P0", "OwnedByP0"))
        .start();
    let h = fixture_p0_owned_p1_controlled(&mut r, "OWNED_BY_P0");

    let p0_trash_before = r.trash_size(0);
    let p1_trash_before = r.trash_size(1);

    {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 1);
        ctx.delete_permanent(h);
    }

    assert_eq!(
        r.trash_size(1),
        p1_trash_before + 1,
        "deletion routes to CONTROLLER's trash (P1) — owner-routing does \
         NOT apply to whole-stack deletion"
    );
    assert_eq!(
        r.trash_size(0),
        p0_trash_before,
        "owner's trash unchanged on deletion"
    );
}

// ─── Game::place_permanent_on_security_observed ────────────────────────────

/// `place_permanent_on_security_observed` places the top card in the
/// `target_player`'s security stack (parameter, not owner-derived). But
/// the **sources below the top** route to each source's own owner's
/// trash. With a transferred multi-source permanent, source routing is
/// the owner-correctness check.
#[test]
fn place_permanent_on_security_observed_routes_below_top_sources_to_each_owners_trash() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base"))
        .add_card(plain_digimon("MID", "Mid"))
        .add_card(plain_digimon("TOP", "Top"))
        .start();
    let h0 = r.place_on_field(0, "TOP", Some(0));
    let _ = r.push_source_owned(h0, "BASE", /* owner */ 1);
    let _ = r.push_source_owned(h0, "MID", /* owner */ 0);
    let h1 = r.transfer_control(h0, 1);
    // Stack: BASE(owner=P1), MID(owner=P0), TOP(owner=P0); controller=P1.

    let p0_trash_before = r.trash_size(0);
    let p1_trash_before = r.trash_size(1);

    // Place into P1's (controller's) security stack at top.
    let ok = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), Some(h1), 1);
        // Use place_self_at_security via the bound source_permanent.
        ctx.place_self_at_security(StackPosition::Top, /* face_up */ false)
    };
    assert!(ok, "place_self_at_security must succeed");

    assert_eq!(
        r.trash_size(0),
        p0_trash_before + 1,
        "MID (owner P0) → P0's trash (owner-routed)"
    );
    assert_eq!(
        r.trash_size(1),
        p1_trash_before + 1,
        "BASE (owner P1) → P1's trash (owner-routed)"
    );
    // TOP went to P1's security (controller-targeted, by design).
    let p1_security: Vec<u8> = r
        .game_mut()
        .player(1)
        .security
        .iter()
        .map(|c| c.owner)
        .collect();
    assert!(
        p1_security.contains(&0),
        "TOP card retains its OWNER (P0) inside the security stack: \
         security card .owner is preserved across the placement, even \
         though the slot is in P1's security zone. Found: {p1_security:?}"
    );
}

// ─── EffectContext::security_place_stacked_card ────────────────────────────

/// Extracting a digivolution source via `security_place_stacked_card`
/// preserves that source's `CardSource.owner` through the security
/// placement. The placement-zone player is parameterized; the source's
/// owner travels with the card.
#[test]
fn security_place_stacked_card_preserves_source_owner_after_control_transfer() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base"))
        .add_card(plain_digimon("TOP", "Top"))
        .start();
    let h0 = r.place_on_field(0, "TOP", Some(0));
    let base_handle = r.push_source_owned(h0, "BASE", /* owner */ 0);
    let h1 = r.transfer_control(h0, 1);
    // Stack: BASE(owner=P0) ← TOP(owner=P0); controller=P1.

    let p1_security_before = r.game_mut().player(1).security.len();

    let ok = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), Some(h1), 1);
        // Move BASE (the bottom source) into P1's security top.
        ctx.security_place_stacked_card(
            h1,
            base_handle,
            /* target */ 1,
            StackPosition::Top,
            false,
        )
    };
    assert!(ok, "security_place_stacked_card must succeed");

    assert_eq!(
        r.game_mut().player(1).security.len(),
        p1_security_before + 1
    );
    // The placed BASE retains owner = 0.
    let placed_owner = r
        .game_mut()
        .player(1)
        .security
        .iter()
        .find(|c| c.handle() == base_handle)
        .expect("BASE must be in P1's security stack")
        .owner;
    assert_eq!(
        placed_owner, 0,
        "extracted source's CardSource.owner is preserved through security placement"
    );
}

#[test]
fn security_place_top_stacked_card_preserves_source_owner_after_control_transfer() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("BASE", "Base"))
        .add_card(plain_digimon("TOP", "Top"))
        .start();
    let h0 = r.place_on_field(0, "TOP", Some(0));
    // The "top stacked card" is `card_sources[len - 2]` — the card just
    // below the visible top. Push BASE owned by P0 first, then TOP stays
    // visible.
    let _ = r.push_source_owned(h0, "BASE", /* owner */ 0);
    let h1 = r.transfer_control(h0, 1);
    // Stack: BASE(owner=P0) ← TOP(owner=P0); controller=P1.

    let ok = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), Some(h1), 1);
        ctx.security_place_top_stacked_card(h1, /* target */ 1, StackPosition::Top, false)
    };
    assert!(ok, "security_place_top_stacked_card must succeed");

    let placed_owner = r.game_mut().player(1).security.last().unwrap().owner;
    assert_eq!(
        placed_owner, 0,
        "stacked source's owner preserved across placement"
    );
}

// ─── EffectContext::return_all_trash_to_deck_bottom ────────────────────────

/// Trash drained to deck-bottom routes each card to its **own** owner's
/// deck. This duplicates `tests/zone_manipulation.rs:2070` shape but is
/// included here so the live-coverage harness exercises the helper end
/// to end through a control-transfer flow.
#[test]
fn return_all_trash_to_deck_bottom_routes_each_card_via_transfer_flow() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("OWNED_BY_P0", "OwnedByP0"))
        .add_card(plain_digimon("OWNED_BY_P1", "OwnedByP1"))
        .start();
    // Step 1: place a P0-owned permanent, transfer to P1, then DELETE it
    // through the controller path. Per `Player::delete_permanent`, the
    // top card lands in P1's trash even though its `owner = P0`. The
    // CardSource.owner is preserved on the trashed copy.
    let h0_p0_owned = r.place_on_field(0, "OWNED_BY_P0", Some(0));
    let _h0_p1_owned = r.place_on_field(1, "OWNED_BY_P1", Some(0));
    let h0_p0_owned = r.transfer_control(h0_p0_owned, 1);
    {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 1);
        ctx.delete_permanent(h0_p0_owned);
    }
    // Account for index shift after delete: re-fetch the still-present permanent.
    let h0_p1_owned = PermanentHandle {
        player: 1,
        index: 0,
    };
    {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 1);
        ctx.delete_permanent(h0_p1_owned);
    }
    // Now P1's trash holds [card_owned_by_P0, card_owned_by_P1].
    assert_eq!(r.trash_size(1), 2);
    assert_eq!(r.trash_size(0), 0);

    let p0_deck_before = r.deck_size(0);
    let p1_deck_before = r.deck_size(1);

    // Drain P1's trash; each card routes to its OWN owner's deck bottom.
    let _handles = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 1);
        ctx.return_all_trash_to_deck_bottom(/* player */ 1)
    };

    assert_eq!(r.trash_size(1), 0, "P1's trash drained");
    assert_eq!(
        r.deck_size(0),
        p0_deck_before + 1,
        "card with owner=P0 routes to P0's deck (owner-routed across drain)"
    );
    assert_eq!(
        r.deck_size(1),
        p1_deck_before + 1,
        "card with owner=P1 routes to P1's deck"
    );
}

// ─── Negative case: owner == controller (the common case) ──────────────────

/// When `owner == controller` (every card in the engine today), routing
/// is unchanged and the common-case path stays controller-equivalent.
/// This test guards against an over-eager "if owner != controller"
/// branch that accidentally degrades the common case.
#[test]
fn return_to_hand_with_owner_equals_controller_routes_to_that_player() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("LOCAL", "Local"))
        .start();
    // No transfer. P0 owns and controls.
    let h = r.place_on_field(0, "LOCAL", Some(0));
    let p0_hand_before = r.hand_size(0);
    let p1_hand_before = r.hand_size(1);

    let returned = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 0);
        ctx.return_to_hand(h)
    };
    assert!(returned.is_some());

    assert_eq!(
        r.hand_size(0),
        p0_hand_before + 1,
        "owner == controller — card returns to P0's hand"
    );
    assert_eq!(
        r.hand_size(1),
        p1_hand_before,
        "P1 unaffected when owner == controller == P0"
    );
}

#[test]
fn return_to_deck_with_owner_equals_controller_routes_to_that_player() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("LOCAL", "Local"))
        .start();
    let h = r.place_on_field(0, "LOCAL", Some(0));
    let p0_deck_before = r.deck_size(0);
    let p1_deck_before = r.deck_size(1);

    let ok = {
        let mut ctx = EffectContext::new(r.game_mut(), CardHandle(0), None, 0);
        ctx.return_to_deck(h, StackPosition::Bottom)
    };
    assert!(ok);

    assert_eq!(r.deck_size(0), p0_deck_before + 1);
    assert_eq!(r.deck_size(1), p1_deck_before);
}

// ─── Defensive: helper itself does not corrupt synthetic state ─────────────

/// Compile-time + run-time sanity that the helper builds a well-formed
/// permanent on the destination side: handle resolves, top card is a
/// real `CardSource`, `Permanent` invariants hold.
#[test]
fn transfer_control_yields_a_well_formed_permanent_on_destination() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("ALPHA", "Alpha"))
        .start();
    let h0 = r.place_on_field(0, "ALPHA", Some(0));
    let h1 = r.transfer_control(h0, 1);

    let perm: &Permanent = &r.game_mut().player(1).battle_area[h1.index as usize];
    assert_eq!(perm.card_sources.len(), 1);
    assert!(!perm.card_sources[0].is_token);
    assert_eq!(perm.card_sources[0].owner, 0);
    // CardSourceRef::Material idx must match.
    let _verify: CardSourceRef = CardSourceRef::Material(h1, 0);
}
