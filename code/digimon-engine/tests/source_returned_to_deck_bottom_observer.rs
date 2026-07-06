//! Gap 1 — `OnDigivolutionCardReturnedToDeckBottom` observer proof.
//!
//! `EffectContext::return_card_source_to_deck(perm, card, to_bottom=true)`
//! (effect_context/action/sources.rs) now fires the
//! `OnDigivolutionCardReturnedToDeckBottom` observer, carrying the former host
//! + returned card as event context. This is the substrate the blocked cards
//! BT21-058 Snatchmon (inherited "[All Turns][OPT] when any [Vemmon] are
//! returned to the bottom of the deck from this Digimon's digivolution cards,
//! delete 1 opp Digimon cost 4 or less") and BT18-065 (same trigger →
//! unsuspend + Blocker) need.
//!
//! These prove the substrate WITHOUT authoring the blocked cards: an inline
//! DSL observer card carries the exact host-scoped + event-card-name gate the
//! real cards use (`event_host_permanent_is_source` + `event_card_name_contains`)
//! and reacts by deleting an opponent Digimon (the shape BT21-058 uses).

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::{Permanent, PermanentHandle};
use digimon_engine::selection::SelectionKind;

/// Inline observer card: an inherited-style observer that, when a [Vemmon] is
/// returned to the bottom of the deck FROM THIS Digimon's digivolution cards,
/// deletes 1 opponent Digimon. `scope: inherited` so it rides under a carrier
/// and `event_host_permanent_is_source` resolves against the carrier.
fn observer_yaml() -> &'static str {
    r#"
card: DX-RETURN-OBSERVER
name: Return Observer
kind: digimon
level: 4
color: [black]
cost: 5
dp: 6000
effects:
  - scope: inherited
    when: on_source_returned_to_deck_bottom
    once_per_turn: true
    condition:
      all_of:
        - event_host_permanent_is_source: true
        - event_card_name_contains: "Vemmon"
        - any_permanent:
            of: opponent
            zone: [battle_area]
            kind: digimon
    summary: "[All Turns][OPT] when a [Vemmon] is returned to deck bottom from this Digimon's digivolution cards, delete 1 opp Digimon"
    process:
      - select_opponent_permanent:
          bind_as: target
          filter: { kind: digimon }
          prompt: "Choose 1 opponent Digimon to delete"
      - delete_permanent: { target: target }
"#
}

fn make_digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

/// Seed a black host permanent whose bottom source is the DX-RETURN-OBSERVER
/// (the inherited observer), then the named `under` card stacked ABOVE it, then
/// a `top` card as the visible top. So the observer rides UNDER the top and its
/// `event_host_permanent_is_source` sees the whole permanent as the host.
///
/// Returns (host_handle, under_source_handle).
fn seed_stack(
    r: &mut DebugRunner,
    observer_id: &str,
    under_id: &str,
    top_id: &str,
) -> (PermanentHandle, digimon_engine::card_source::CardHandle) {
    let mut sources = Vec::new();
    for card_id in [observer_id, under_id, top_id] {
        let g = r.game_mut();
        let data_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == card_id)
            .unwrap_or_else(|| panic!("card {card_id} in registry"));
        let card_idx = g.next_card_index();
        sources.push(CardSource::new(data_idx, 0, card_idx));
    }
    let under_handle = sources[1].handle();
    let g = r.game_mut();
    let turn = g.turn_count;
    let bottom = sources.remove(0);
    let mut perm = Permanent::new(bottom, turn);
    perm.card_sources.extend(sources);
    g.players[0].battle_area.push(perm);
    (
        PermanentHandle {
            player: 0,
            index: 0,
        },
        under_handle,
    )
}

#[test]
fn returning_vemmon_source_to_deck_bottom_fires_host_scoped_observer() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(observer_yaml())
        .expect("observer YAML compiles")
        .add_card(make_digimon("VEMMON-SRC", "Vemmon"))
        .add_card(make_digimon("HOST-TOP", "Host Top"))
        .add_card(make_digimon("OPP-DIGI", "Opp Digimon"))
        .start();

    let (host, vemmon) = seed_stack(&mut r, "DX-RETURN-OBSERVER", "VEMMON-SRC", "HOST-TOP");
    r.place_on_field(1, "OPP-DIGI", None);

    let top = r.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(
            ctx.return_card_source_to_deck(host, vemmon, /* to_bottom */ true),
            "the Vemmon source must be found and returned"
        );
    }

    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OppField),
        "returning a [Vemmon] source to deck bottom must fire the inherited observer"
    );
    r.auto_resolve().expect("delete resolves");
    assert_eq!(
        r.game.player(1).battle_area.len(),
        0,
        "the observer deleted the opponent's Digimon"
    );
}

#[test]
fn returning_non_vemmon_source_does_not_fire_observer() {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(observer_yaml())
        .expect("observer YAML compiles")
        .add_card(make_digimon("AGUMON-SRC", "Agumon"))
        .add_card(make_digimon("HOST-TOP", "Host Top"))
        .add_card(make_digimon("OPP-DIGI", "Opp Digimon"))
        .start();

    let (host, agumon) = seed_stack(&mut r, "DX-RETURN-OBSERVER", "AGUMON-SRC", "HOST-TOP");
    r.place_on_field(1, "OPP-DIGI", None);

    let top = r.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, agumon, true));
    }

    assert!(
        r.pending_selection().is_none(),
        "a non-[Vemmon] return must NOT fire the name-gated observer"
    );
    assert_eq!(r.game.player(1).battle_area.len(), 1);
}

#[test]
fn returning_vemmon_source_to_deck_top_does_not_fire_observer() {
    // The printed observer is scoped to deck-BOTTOM returns; a top-of-deck
    // return fires nothing.
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(observer_yaml())
        .expect("observer YAML compiles")
        .add_card(make_digimon("VEMMON-SRC", "Vemmon"))
        .add_card(make_digimon("HOST-TOP", "Host Top"))
        .add_card(make_digimon("OPP-DIGI", "Opp Digimon"))
        .start();

    let (host, vemmon) = seed_stack(&mut r, "DX-RETURN-OBSERVER", "VEMMON-SRC", "HOST-TOP");
    r.place_on_field(1, "OPP-DIGI", None);

    let top = r.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, vemmon, /* to_bottom */ false));
    }

    assert!(
        r.pending_selection().is_none(),
        "a deck-TOP return must NOT fire the bottom-scoped observer"
    );
    assert_eq!(r.game.player(1).battle_area.len(), 1);
}

#[test]
fn observer_is_once_per_turn_across_multiple_returned_vemmon() {
    // BT21-062 returns 4 [Vemmon] at once; the inherited BT21-058 observer is
    // [Once Per Turn], so it fires (and deletes) only once even across a
    // multi-source return. Prove the OPT accounting by returning two Vemmon
    // sources one after another and asserting only ONE delete prompt.
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(observer_yaml())
        .expect("observer YAML compiles")
        .add_card(make_digimon("VEMMON-A", "Vemmon"))
        .add_card(make_digimon("VEMMON-B", "Vemmon"))
        .add_card(make_digimon("HOST-TOP", "Host Top"))
        .add_card(make_digimon("OPP-A", "Opp A"))
        .add_card(make_digimon("OPP-B", "Opp B"))
        .start();

    // Stack: observer (bottom) / VEMMON-A / VEMMON-B / HOST-TOP (top).
    let mut sources = Vec::new();
    for id in ["DX-RETURN-OBSERVER", "VEMMON-A", "VEMMON-B", "HOST-TOP"] {
        let g = r.game_mut();
        let data_idx = g.card_data.iter().position(|c| c.card_id == id).unwrap();
        let card_idx = g.next_card_index();
        sources.push(CardSource::new(data_idx, 0, card_idx));
    }
    let vem_a_h = sources[1].handle();
    let vem_b_h = sources[2].handle();
    {
        let g = r.game_mut();
        let turn = g.turn_count;
        let bottom = sources.remove(0);
        let mut perm = Permanent::new(bottom, turn);
        perm.card_sources.extend(sources);
        g.players[0].battle_area.push(perm);
    }
    let host = PermanentHandle {
        player: 0,
        index: 0,
    };

    r.place_on_field(1, "OPP-A", None);
    r.place_on_field(1, "OPP-B", None);

    let top = r.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut r.game, top, Some(host), 0);
        // First return fires the observer → a delete prompt.
        assert!(ctx.return_card_source_to_deck(host, vem_a_h, true));
    }
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OppField),
        "first [Vemmon] return fires the OPT observer"
    );
    r.auto_resolve().expect("first delete resolves");
    assert_eq!(r.game.player(1).battle_area.len(), 1);

    // Second return in the SAME turn must NOT fire again (once_per_turn).
    let top = r.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut r.game, top, Some(host), 0);
        assert!(ctx.return_card_source_to_deck(host, vem_b_h, true));
    }
    assert!(
        r.pending_selection().is_none(),
        "the once-per-turn observer must not fire a second time this turn"
    );
    assert_eq!(
        r.game.player(1).battle_area.len(),
        1,
        "no second deletion — OPT consumed its single use"
    );
}
