//! Unsuspend / Draw / Breeding phases — surface R. Ledger rows US-2/3, DR-2, BR-*.
//!
//! "During this phase, do I also unsuspend my opponent's Digimon and Tamers?
//! No. Only the player whose turn it is gets to unsuspend." / "Can I choose not
//! to unsuspend Digimon and/or Tamers during this phase? No, suspended cards
//! must be unsuspended." / "Is there a maximum hand size? No." (General
//! Rules/FAQ.) Rules manual §C-2 (Unsuspend Phase).
//!
//! Vehicle: AD1-001 Greymon (real implemented DSL card). DR-1 (mandatory draw /
//! deck-out loss) cross-links `tests/infra/headless_runner.rs:257`. Breeding
//! rows (BR-*) remain TBD (tasks.md §5.1).

#![allow(unused_imports)]

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::PlayerId;

fn opponent(p: PlayerId) -> PlayerId {
    if p == 0 {
        1
    } else {
        0
    }
}

/// US-2 + US-3 — at the start of a player's turn, the turn player's suspended
/// Digimon are unsuspended (mandatory), and the OPPONENT's suspended Digimon are
/// NOT unsuspended.
#[test]
fn us_unsuspend_is_turn_player_only_and_mandatory() {
    let mut r = DebugRunner::builder()
        .dsl_card("AD1-001")
        .expect("AD1-001 (Greymon) in embedded DSL pack")
        .deck(0, &["AD1-001", "AD1-001", "AD1-001", "AD1-001"])
        .deck(1, &["AD1-001", "AD1-001", "AD1-001", "AD1-001"])
        .start();

    let p0 = r.turn_player();
    let p1 = opponent(p0);
    let h0 = r.place_on_field(p0, "AD1-001", Some(0));
    let h1 = r.place_on_field(p1, "AD1-001", Some(0));

    // Suspend both Digimon while it is p0's turn.
    r.game.players[p0 as usize].battle_area[h0.index as usize].is_suspended = true;
    r.game.players[p1 as usize].battle_area[h1.index as usize].is_suspended = true;

    // End p0's turn → p1's turn begins → p1's unsuspend phase runs.
    r.end_turn();
    assert_eq!(r.turn_player(), p1, "turn advanced to the opponent");

    assert!(
        !r.game.players[p1 as usize].battle_area[h1.index as usize].is_suspended,
        "FAQ: the turn player's suspended Digimon MUST be unsuspended (mandatory)"
    );
    assert!(
        r.game.players[p0 as usize].battle_area[h0.index as usize].is_suspended,
        "FAQ: the opponent's Digimon are NOT unsuspended on the turn player's turn"
    );
}

/// DR-2 — there is no maximum hand size; a large hand is not truncated.
#[test]
fn dr2_no_maximum_hand_size() {
    let big_hand: Vec<&str> = std::iter::repeat("AD1-001").take(12).collect();
    let r = DebugRunner::builder()
        .dsl_card("AD1-001")
        .expect("AD1-001 (Greymon) in embedded DSL pack")
        .hand(0, &big_hand)
        .start();

    assert!(
        r.hand_size(0) >= 12,
        "FAQ: no maximum hand size — a 12-card hand is not truncated (got {})",
        r.hand_size(0)
    );
}

/// BR-4 — running out of Digi-Eggs does NOT lose the game (you just can't hatch).
#[test]
fn br4_empty_egg_deck_does_not_lose_the_game() {
    let mut r = DebugRunner::builder()
        .dsl_card("AD1-001")
        .expect("AD1-001 (Greymon) in embedded DSL pack")
        .start();
    let p = r.turn_player();
    assert!(
        r.game.player(p).digitama_deck.is_empty(),
        "precondition: the Digi-Egg deck is empty"
    );

    let hatched = r.game.hatch(p);
    assert!(!hatched, "hatch fails when the Digi-Egg deck is empty");
    assert!(
        !r.game.game_over,
        "FAQ BR-4: an empty Digi-Egg deck does NOT lose the game"
    );
}
