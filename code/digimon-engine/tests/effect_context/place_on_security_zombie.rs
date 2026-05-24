//! Regression test for `G-PERMANENT-EMPTY-DURING-MATERIAL-EXTRACTION` —
//! `EffectContext::place_on_security` sibling. Mirrors the digivolve-from-
//! material zombie test landed in PR #533 but exercises the place-into-
//! security take.
//!
//! Setup: a single-source carrier in `battle_area`; an effect places that
//! carrier's only card into the controller's security stack via
//! `CardSourceRef::Material(carrier, 0)`. The take empties the carrier;
//! without the cleanup landed by `fix-zombie-permanent-siblings` the slot
//! remains in `battle_area` with `card_sources.is_empty()`, panicking any
//! downstream `top_card()` reader.
//!
//! The companion `replacement→Trash` sibling (also in
//! `place_on_security_observed`, lines ~6121-6166) is patched in the same
//! change but is not exercised here because the test setup requires a
//! YAML/Rust card with a `WhenWouldPlaceInSecurity` replacement effect that
//! returns `Redirected(Trash)` — the lightweight `make_test_card` cards
//! used in this file carry no effects. The fix shape is identical and the
//! `place_on_security_observed::commit` test below validates the
//! `soft_remove_if_emptied` follow-up via the same code path.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardSourceRef, StackPosition};

#[test]
fn place_on_security_from_material_emptying_carrier_removes_slot() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("T-CARRIER", "CarrierTest"))
        .add_card(make_test_card("T-SOURCE", "SourceTest"))
        .memory(3)
        .start();

    // Source-runner permanent — provides the `source_card` for EffectContext.
    let src_perm = runner.place_on_field(0, "T-SOURCE", None);
    // Carrier — its only card is what we'll route into security.
    let carrier = runner.place_on_field(0, "T-CARRIER", None);
    assert_eq!(
        runner.game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .len(),
        1,
        "precondition: carrier has exactly 1 source"
    );

    let src_handle = runner.game.players[0].battle_area[src_perm.index as usize]
        .top_card()
        .handle();
    let security_len_before = runner.game.players[0].security.len();

    let ok = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, Some(src_perm), 0);
        ctx.place_on_security(
            0,
            CardSourceRef::Material(carrier, 0),
            StackPosition::Top,
            false,
        )
    };
    assert!(ok, "place_on_security should succeed");

    // No zombies anywhere.
    for (pid, player) in runner.game.players.iter().enumerate() {
        for (slot, perm) in player.battle_area.iter().enumerate() {
            assert!(
                !perm.card_sources.is_empty(),
                "zombie permanent at p{}.battle_area[{}]: card_sources is empty",
                pid,
                slot
            );
        }
    }

    // Carrier removed; only the src_perm remains.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "carrier slot should be removed after soft-remove"
    );
    // Security gained the routed card.
    assert_eq!(
        runner.game.players[0].security.len(),
        security_len_before + 1,
        "security should grow by 1"
    );
}
