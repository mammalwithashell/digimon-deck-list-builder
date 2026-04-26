//! Behavioral tests for `ctx.de_digivolve` — the generalized
//! De-Digivolve N primitive. Covers:
//!
//!   * `Some(amount)` with `Some(3)` stop-at-level (standard De-Digivolve N wording)
//!   * `None` amount with `None` stop-at-level (TS Olympos Ikkakumon:
//!     pop until stack empty / until single base)
//!   * Trash routing — popped sources land in owner's trash
//!   * Stack-depth-1 no-op (cannot trash past a single-card stack)
//!
//! Each test exercises the primitive through a dedicated test card so we
//! go through the full play pipeline — not just the method directly.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn lvl_digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.dp = Some(dp);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.play_cost = 0;
    c
}

/// Push a fresh CardSource with `card_id` onto the owner's existing permanent's stack.
fn push_on_stack(r: &mut DebugRunner, owner: u8, perm_idx: usize, card_id: &str) {
    let data_idx = r.game.card_data.iter().position(|c| c.card_id == card_id).unwrap();
    let next = r.game.next_card_index();
    let cs = CardSource::new(data_idx, owner, next);
    let turn = r.game.turn_count;
    r.game.players[owner as usize].battle_area[perm_idx].digivolve(cs, turn);
}

#[test]
fn de_digivolve_2_pops_two_and_stops_at_lvl3() {
    // Stack on P1: [Lv3, Lv4, Lv5]. TEST-024 OnPlay asks to pop 2
    // (stop_at_level=Some(3), amount=Some(2)). Expected: pop LVL5 and
    // LVL4, land both in P1's trash. Lv3 remains as the base.
    let mut r = DebugRunner::builder()
        .add_card(lvl_digimon("LVL3", "Three", 3, 2000))
        .add_card(lvl_digimon("LVL4", "Four", 4, 4000))
        .add_card(lvl_digimon("LVL5", "Five", 5, 7000))
        .add_card(make_test_card("TEST-024", "DeDig2StopAt3"))
        .hand(0, &["TEST-024"])
        .memory(5)
        .start();

    let base = r.place_on_field(1, "LVL3", Some(0));
    push_on_stack(&mut r, 1, base.index as usize, "LVL4");
    push_on_stack(&mut r, 1, base.index as usize, "LVL5");
    assert_eq!(r.game.player(1).battle_area[base.index as usize].stack_size(), 3);
    let trash_before = r.trash_size(1);

    r.play(0, 0);

    let perm = &r.game.player(1).battle_area[base.index as usize];
    assert_eq!(perm.stack_size(), 1, "popped 2, left the Lv3 base");
    assert_eq!(perm.top_card().card_id(&r.game.card_data), "LVL3");
    assert_eq!(r.trash_size(1), trash_before + 2,
        "both popped sources land in P1's trash");
}

#[test]
fn de_digivolve_unbounded_pops_whole_stack() {
    // Stack on P1: [Lv3, Lv4]. TEST-025 OnPlay: (amount=None, stop_at_level=None).
    // Expected: pops LVL4; LVL3 stays because stack_size must be >= 1.
    let mut r = DebugRunner::builder()
        .add_card(lvl_digimon("LVL3", "Three", 3, 2000))
        .add_card(lvl_digimon("LVL4", "Four", 4, 4000))
        .add_card(make_test_card("TEST-025", "DeDigUnbounded"))
        .hand(0, &["TEST-025"])
        .memory(5)
        .start();

    let base = r.place_on_field(1, "LVL3", Some(0));
    push_on_stack(&mut r, 1, base.index as usize, "LVL4");
    assert_eq!(r.game.player(1).battle_area[base.index as usize].stack_size(), 2);

    r.play(0, 0);

    let perm = &r.game.player(1).battle_area[base.index as usize];
    assert_eq!(perm.stack_size(), 1, "unbounded pop leaves the base (stack_size >= 1 invariant)");
    assert_eq!(perm.top_card().card_id(&r.game.card_data), "LVL3");
}

#[test]
fn de_digivolve_on_single_card_stack_is_noop() {
    // Stack on P1: [Lv4] only (single-card stack). TEST-024 asks to pop 2
    // with stop_at_level=Some(3). Expected: zero pops, no trash entries.
    let mut r = DebugRunner::builder()
        .add_card(lvl_digimon("LVL4", "Four", 4, 4000))
        .add_card(make_test_card("TEST-024", "DeDig2StopAt3"))
        .hand(0, &["TEST-024"])
        .memory(5)
        .start();
    let base = r.place_on_field(1, "LVL4", Some(0));

    r.play(0, 0);

    assert_eq!(r.game.player(1).battle_area[base.index as usize].stack_size(), 1);
    assert_eq!(r.trash_size(1), 0, "no pops, no trash entries");
}
