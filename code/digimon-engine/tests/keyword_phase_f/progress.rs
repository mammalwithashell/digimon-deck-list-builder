//! Phase F — `Keyword::Progress` card-shaped behavioral tests.
//!
//! Printed text: "`<Progress>` (While attacking, your opponent's effects
//! don't affect this Digimon.)" — RULES_CONTEXT 16-XX. DCGO reference:
//! `KeyWordEffects/Progress.cs:42-58` (`CanActivateProgress` requires
//! attacker permanent matches the carrier; `ProgressProcess` installs a
//! `CanNotAffectedClass` for the duration of the attack).
//!
//! These tests exercise `Game::progress_excludes` (the canonical immunity
//! gate consumed by every opponent-sourced mutation site) through realistic
//! `has_keyword` paths:
//!
//!   1. Native printed Progress on the top card.
//!   2. Granted Progress via `ModifierType` keyword grant (modifier path).
//!   3. Inherited Progress: a digivolution source under the top card carries
//!      `<Progress>` in its inherited text — `has_keyword` walks the stack
//!      and finds it.
//!   4. Inherited-stack-depth: Progress on a deep source (multiple cards
//!      below the top) is still discovered.
//!   5. Modifier-granted Progress expiry: after the EndOfTurn modifier is
//!      cleared, Progress immunity goes away.
//!   6. Negative-scope: the carrier's own controller's effects are NEVER
//!      excluded by Progress — only opponent-sourced effects are gated.
//!   7. Source on the *top* of the stack does NOT count as inherited (only
//!      sources strictly below the top contribute inherited keywords).
//!
//! Stack-shape note: a card placed via `DebugRunner::place_on_field` lands
//! as a single-source stack (the top card). Tests that require sources
//! under the top push extra `CardSource`s into `permanent.card_sources` at
//! position 0 (bottom). Mirrors the `iceclad.rs` helper pattern.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{Expiry, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{AttackState, AttackTarget, PendingAttack};

use super::helpers::{plain_digimon, plain_tamer};

/// Build a CardData with `<Progress>` in its inherited text section.
/// Inherited keywords are parsed by `parse_printed_keywords` against the
/// `inherited_text` field. Filler cards under the top must carry the
/// keyword in their own inherited text to grant it to whichever Digimon
/// has them in its stack.
fn inherited_progress_filler(id: &str) -> CardData {
    let mut c = plain_digimon(id);
    c.inherited_text = "\u{ff1c}Progress\u{ff1e} (While attacking, your opponent's effects don't affect this Digimon.)".to_string();
    c
}

/// Push `extra_under_top` sources of `filler_id` UNDER the existing top of
/// `target`'s stack. Position 0 = bottom of stack; `top_card()` is
/// `card_sources.last()`. After this call:
/// `card_sources.len() == 1 + extra_under_top` and the original top card is
/// preserved as the visible top.
fn push_sources_under_top(
    r: &mut DebugRunner,
    target: PermanentHandle,
    filler_id: &str,
    extra_under_top: usize,
) {
    let game = r.game_mut();
    let filler_idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == filler_id)
        .unwrap_or_else(|| panic!("push_sources_under_top: unknown card_id {filler_id}"));
    for _ in 0..extra_under_top {
        let ci = game.next_card_index();
        let src = CardSource::new(filler_idx, target.player, ci);
        game.players[target.player as usize].battle_area[target.index as usize]
            .card_sources
            .insert(0, src);
    }
}

/// Inject a fake `PendingAttack` so the Progress gate's "carrier is the
/// current attacker" check passes. Mirrors the unit-test pattern in
/// `game.rs::progress_excludes_only_when_attacking_and_opponent_sourced`.
fn inject_attacker(r: &mut DebugRunner, attacker: PermanentHandle) {
    r.game.pending_attack = Some(PendingAttack {
        attacker,
        original_target: AttackTarget::Player(1 - attacker.player),
        effective_target: AttackTarget::Player(1 - attacker.player),
        is_blocked: false,
        blocker: None,
        is_vortex: false,
        is_overclock: false,
        declaration_committed: true,
        cancelled: false,
        battle_occurred: false,
        battle_defender_deleted: false,
        return_phase: digimon_engine::enums::GamePhase::Main,
        state: AttackState::Declared,
        counter_depth: 0,
    });
}

// ─── Test 1: native printed Progress on the top card ────────────────────────

/// Carrier with `keywords: vec![Keyword::Progress]` (face-printed) is
/// excluded from opponent-sourced mutations while attacking. Sanity check
/// that the `progress_excludes` gate fires for the simplest face-printed
/// case.
#[test]
fn native_printed_progress_excludes_opponent_effects_while_attacking() {
    let mut prog_card = plain_digimon("PROG");
    prog_card.keywords = vec![Keyword::Progress];
    let opp = plain_digimon("OPP");

    let mut r = DebugRunner::builder()
        .add_card(prog_card)
        .add_card(opp)
        .start();
    let progress = r.place_on_field(0, "PROG", None);
    let _opp_perm = r.place_on_field(1, "OPP", None);

    inject_attacker(&mut r, progress);

    assert!(
        r.game.has_keyword(progress, Keyword::Progress),
        "native printed Progress must be discovered by has_keyword"
    );
    assert!(
        r.game.progress_excludes(progress, Some(1)),
        "while attacking + opponent-sourced, native Progress excludes the mutation"
    );
}

// ─── Test 2: granted Progress via modifier ──────────────────────────────────

/// A plain Digimon (no native Progress) gains Progress via a keyword-grant
/// modifier (e.g. an aura granting `<Progress>` to all your Dinosaur
/// Digimon). The same gate applies until the modifier expires.
#[test]
fn granted_progress_via_modifier_excludes_opponent_effects() {
    let plain = plain_digimon("CARRIER");
    let opp = plain_digimon("OPP");
    let mut r = DebugRunner::builder().add_card(plain).add_card(opp).start();

    let carrier = r.place_on_field(0, "CARRIER", None);
    let _opp_perm = r.place_on_field(1, "OPP", None);

    // Pre-grant: no Progress, no exclusion.
    assert!(
        !r.game.has_keyword(carrier, Keyword::Progress),
        "no native Progress before grant"
    );

    r.game
        .modifiers
        .grant_keyword(carrier, Keyword::Progress, Expiry::EndOfTurn, 0);

    assert!(
        r.game.has_keyword(carrier, Keyword::Progress),
        "modifier-granted Progress must be discovered by has_keyword"
    );

    inject_attacker(&mut r, carrier);
    assert!(
        r.game.progress_excludes(carrier, Some(1)),
        "modifier-granted Progress gates opponent effects identically"
    );
}

// ─── Test 3: inherited Progress (source under top card) ─────────────────────

/// CRITICAL test: a Digimon whose top card has NO Progress, but whose
/// digivolution stack contains a source carrying inherited `<Progress>`,
/// is treated as having Progress. Mirrors the inherited-keyword semantics
/// already implemented in `Game::has_keyword`'s stack walk.
#[test]
fn inherited_progress_on_source_under_top_excludes_opponent_effects() {
    let top = plain_digimon("TOP-NOPROG");
    let inherited = inherited_progress_filler("INH-PROG");
    let opp = plain_digimon("OPP");

    let mut r = DebugRunner::builder()
        .add_card(top)
        .add_card(inherited)
        .add_card(opp)
        .start();

    let carrier = r.place_on_field(0, "TOP-NOPROG", None);
    let _opp_perm = r.place_on_field(1, "OPP", None);

    // No Progress yet — top card has no Progress, no source has been added.
    assert!(
        !r.game.has_keyword(carrier, Keyword::Progress),
        "no Progress before inherited source is added"
    );

    // Push the inherited-Progress source UNDER the existing top.
    push_sources_under_top(&mut r, carrier, "INH-PROG", 1);
    assert_eq!(
        r.game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .len(),
        2,
        "stack: [INH-PROG, TOP-NOPROG] (bottom → top)"
    );
    assert_eq!(
        r.game.players[0].battle_area[carrier.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "TOP-NOPROG",
        "top is still the original placed card"
    );

    assert!(
        r.game.has_keyword(carrier, Keyword::Progress),
        "inherited Progress on a source under the top must be discovered"
    );

    inject_attacker(&mut r, carrier);
    assert!(
        r.game.progress_excludes(carrier, Some(1)),
        "inherited Progress gates opponent effects identically to native"
    );
}

// ─── Test 4: stack-depth — Progress on a DEEP source still discovered ───────

/// The inherited-keyword walk in `Game::has_keyword` visits every source
/// strictly below the top, regardless of depth. A Progress source at the
/// bottom of a 4-card stack is just as discoverable as a source
/// immediately under the top.
#[test]
fn inherited_progress_at_stack_bottom_still_excludes() {
    let top = plain_digimon("TOP-NOPROG");
    let inherited = inherited_progress_filler("INH-PROG");
    let filler = plain_digimon("FILLER");
    let opp = plain_digimon("OPP");

    let mut r = DebugRunner::builder()
        .add_card(top)
        .add_card(inherited)
        .add_card(filler)
        .add_card(opp)
        .start();

    let carrier = r.place_on_field(0, "TOP-NOPROG", None);
    let _opp_perm = r.place_on_field(1, "OPP", None);

    // Build stack: [INH-PROG (bottom), FILLER, FILLER, TOP-NOPROG (top)].
    push_sources_under_top(&mut r, carrier, "FILLER", 1);
    push_sources_under_top(&mut r, carrier, "FILLER", 1);
    push_sources_under_top(&mut r, carrier, "INH-PROG", 1);

    assert_eq!(
        r.game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .len(),
        4,
        "4-card stack with INH-PROG at the bottom"
    );

    assert!(
        r.game.has_keyword(carrier, Keyword::Progress),
        "inherited Progress on a deep source must be discovered"
    );

    inject_attacker(&mut r, carrier);
    assert!(
        r.game.progress_excludes(carrier, Some(1)),
        "deep inherited Progress gates opponent effects"
    );
}

// ─── Test 5: source on the TOP does not contribute as inherited ─────────────

/// `inherited_keywords()` is consulted only for sources strictly below the
/// top. A card whose ONLY occurrence in the stack is the top card does not
/// count as inherited — its inherited text only fires while it sits under
/// a higher digivolution. Sanity check that the stack walk respects this
/// rule.
#[test]
fn inherited_progress_on_top_card_alone_is_not_progress() {
    // Place the inherited-Progress card AS THE TOP, with no sources beneath.
    // Its inherited text is silent because nothing sits above it.
    let inherited = inherited_progress_filler("INH-PROG");
    let opp = plain_digimon("OPP");

    let mut r = DebugRunner::builder()
        .add_card(inherited)
        .add_card(opp)
        .start();

    let carrier = r.place_on_field(0, "INH-PROG", None);
    let _opp_perm = r.place_on_field(1, "OPP", None);

    assert_eq!(
        r.game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .len(),
        1,
        "single-card stack: just the top, no sources below"
    );

    assert!(
        !r.game.has_keyword(carrier, Keyword::Progress),
        "inherited keywords on a top-only card MUST NOT count — \
         inherited text fires only when the card sits under another"
    );

    inject_attacker(&mut r, carrier);
    assert!(
        !r.game.progress_excludes(carrier, Some(1)),
        "no Progress means no exclusion"
    );
}

// ─── Test 6: modifier-granted Progress expiry ──────────────────────────────

/// A keyword grant with `Expiry::EndOfTurn` is cleared by the end-of-turn
/// modifier sweep. After the sweep, `has_keyword` must no longer report
/// Progress and `progress_excludes` must release the exclusion.
#[test]
fn modifier_granted_progress_expires_at_end_of_turn() {
    let plain = plain_digimon("CARRIER");
    let opp = plain_digimon("OPP");
    let mut r = DebugRunner::builder().add_card(plain).add_card(opp).start();

    let carrier = r.place_on_field(0, "CARRIER", None);
    let _opp_perm = r.place_on_field(1, "OPP", None);

    // Grant Progress for this turn only (controller is player 0).
    r.game
        .modifiers
        .grant_keyword(carrier, Keyword::Progress, Expiry::EndOfTurn, 0);
    assert!(r.game.has_keyword(carrier, Keyword::Progress));

    // Trigger end-of-turn modifier sweep for player 0.
    r.game.modifiers.expire_end_of_turn(0);

    assert!(
        !r.game.has_keyword(carrier, Keyword::Progress),
        "EndOfTurn-granted Progress must NOT survive the end-of-turn sweep"
    );

    inject_attacker(&mut r, carrier);
    assert!(
        !r.game.progress_excludes(carrier, Some(1)),
        "expired Progress no longer excludes opponent effects"
    );
}

// ─── Test 7: negative scope — Progress does NOT block own effects ───────────

/// `progress_excludes` returns `false` for own-controller effects. Progress
/// is an opponent-only immunity gate; the carrier's own controller can
/// always target their own Progress carrier.
#[test]
fn progress_does_not_exclude_own_effects() {
    let mut prog_card = plain_digimon("PROG");
    prog_card.keywords = vec![Keyword::Progress];

    let mut r = DebugRunner::builder().add_card(prog_card).start();
    let progress = r.place_on_field(0, "PROG", None);

    inject_attacker(&mut r, progress);

    // Own-side mutation (source = player 0, target's controller = player 0).
    assert!(
        !r.game.progress_excludes(progress, Some(0)),
        "Progress is opponent-only — own controller's effects pass through"
    );
    // None source player (rule cleanup, no effect attribution): no exclusion.
    assert!(
        !r.game.progress_excludes(progress, None),
        "no source player → no exclusion"
    );
    // Sanity: opponent source IS excluded.
    assert!(
        r.game.progress_excludes(progress, Some(1)),
        "opponent-sourced effect IS excluded"
    );
}

// ─── Test 8: not-attacking carrier — Progress is dormant ────────────────────

/// Progress is conditional on the carrier being the current attacker
/// (DCGO `CanActivateProgress` checks `AttackingPermanent ==
/// PermanentOfThisCard`). When no attack is in flight, the carrier is NOT
/// excluded even if it has the keyword.
#[test]
fn progress_dormant_when_carrier_is_not_attacking() {
    let mut prog_card = plain_digimon("PROG");
    prog_card.keywords = vec![Keyword::Progress];

    let mut r = DebugRunner::builder().add_card(prog_card).start();
    let progress = r.place_on_field(0, "PROG", None);

    // No injected pending_attack — carrier is not the attacker.
    assert!(
        r.game.has_keyword(progress, Keyword::Progress),
        "keyword is on the card regardless of attack state"
    );
    assert!(
        !r.game.progress_excludes(progress, Some(1)),
        "not attacking → Progress is dormant → no exclusion"
    );
}

// ─── Test 9: Tamer carrier of inherited Progress ────────────────────────────

/// Sanity check that the inherited-keyword walk uses the card's
/// inherited_text field directly — not a kind-specific path. A Tamer card
/// with `<Progress>` in its inherited_text would still grant Progress when
/// placed under a Digimon (the Tamer is mid-stack as a source). This is a
/// thin sanity-check that the walk is kind-agnostic; printed Tamer
/// inherited text is rare for Progress specifically, but the engine must
/// not gate the walk on `CardKind`.
#[test]
fn inherited_progress_walk_is_card_kind_agnostic() {
    let top = plain_digimon("TOP-NOPROG");
    // Tamer with `<Progress>` in inherited_text. Same parse path; same
    // discovery. Tamers cannot normally exist as digivolution sources, but
    // engine effects (e.g. MindLink) can place a Tamer under a Digimon.
    let mut tamer = plain_tamer("TAMER-PROG");
    tamer.inherited_text = "\u{ff1c}Progress\u{ff1e}".to_string();
    let opp = plain_digimon("OPP");

    let mut r = DebugRunner::builder()
        .add_card(top)
        .add_card(tamer)
        .add_card(opp)
        .start();

    let carrier = r.place_on_field(0, "TOP-NOPROG", None);
    let _opp_perm = r.place_on_field(1, "OPP", None);

    push_sources_under_top(&mut r, carrier, "TAMER-PROG", 1);

    assert!(
        r.game.has_keyword(carrier, Keyword::Progress),
        "inherited keyword discovery walks all sources irrespective of card kind"
    );
}
