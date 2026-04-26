//! Phase C end-to-end test card — Fragment(2)-like multi-pick replacement.
//!
//! A Digimon with `<Fragment(2)>` on `WhenWouldBeDeleted` may pick 2 cards
//! from the controller's hand to trash to cancel deletion. Two cases:
//!   1. Hand size > N → outer accept up → pick 2 → cancel + hand shrinks by 2.
//!   2. Hand size ≤ N → outer optional-accept gated off via condition.
//!
//! NOTE: The plan originally called for trashing 2 of the carrier's own
//! digivolution sources via `CountCappedZone::Material(handle)`, but that
//! variant does not exist in the engine — only `Hand` and `Trash` are
//! supported by `select_count_capped_multi`. Test adapted to use `Hand`,
//! which exercises the same multi-pick parked-replacement substrate.

use std::sync::Arc;

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::CountCappedZone;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementSubject;

const FRAGMENT_N: u8 = 2;

fn fragment_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(5000),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: "FRAGMENT_LIKE".to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

fn filler_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// FragmentLike effect — multi-pick over the controller's hand. If N cards
/// can be trashed, deletion is cancelled. Each test installs this fresh on
/// its own DebugRunner via `r.register_effect`.
struct FragmentLike;
impl CardEffect for FragmentLike {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("<Fragment(2)>")
            .optional()
            // Phase D's auto-install will set this condition; for Phase C
            // end-to-end we set it inline.
            .condition(|ctx| {
                let h = match ctx.source_permanent {
                    Some(h) => h,
                    None => return false,
                };
                let p = &ctx.game.players[h.player as usize];
                // Need > N cards in hand so we can trash N.
                p.hand.len() > FRAGMENT_N as usize
            })
            .replacement_process(|rctx| {
                let _me = match rctx.subject {
                    ReplacementSubject::Permanent(h) => h,
                    _ => return,
                };
                let controller = rctx.effect.player;
                rctx.effect.select_count_capped_multi(
                    controller,
                    CountCappedZone::Hand,
                    FRAGMENT_N,
                    "trash N cards from hand",
                    false,
                    |_g, _src| true,
                    move |ctx, picks| {
                        // Trash picked cards from hand. picks is a Vec<CardHandle>.
                        // Map handles back to current hand indices and remove
                        // (highest index first so removals don't shift).
                        let mut indices: Vec<usize> = Vec::with_capacity(picks.len());
                        {
                            let p = &ctx.game.players[controller as usize];
                            for h in &picks {
                                if let Some(idx) = p
                                    .hand
                                    .iter()
                                    .position(|c| c.handle() == *h)
                                {
                                    indices.push(idx);
                                }
                            }
                        }
                        indices.sort_by(|a, b| b.cmp(a));
                        for idx in indices {
                            ctx.game.trash_from_hand_by_index(controller, idx);
                        }
                        ctx.cancel_leave();
                    },
                );
            })
            .build()]
    }
}

#[test]
fn fragment_2_picks_two_cards_and_cancels() {
    let mut r = DebugRunner::builder()
        .add_card(fragment_card("FRAG"))
        .add_card(filler_card("FILL1"))
        .add_card(filler_card("FILL2"))
        .add_card(filler_card("FILL3"))
        .hand(0, &["FILL1", "FILL2", "FILL3"])
        .start();
    r.register_effect("FRAG", Arc::new(FragmentLike));
    let frag = r.place_on_field(0, "FRAG", None);

    let hand_before = r.game.players[0].hand.len();
    let trash_before = r.game.players[0].trash.len();
    assert_eq!(hand_before, 3, "preconditions: 3 cards in hand");

    r.game.delete_permanent_with_effects(frag);
    assert!(r.game.pending_selection.is_some(), "outer accept up");
    r.game.resolve_selection(0, REPLACEMENT_ACCEPT).expect("accept");

    // Multi-pick chain: 2 picks. Each pick uses the first valid action ID.
    for _ in 0..FRAGMENT_N {
        let pending = r.game.pending_selection.as_ref().expect("inner step");
        let action = pending.valid_action_ids[0];
        let player = pending.selecting_player;
        r.game.resolve_selection(player, action).expect("pick");
    }

    // Carrier survived; hand shrank by 2; trash gained 2.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "FRAG survived (cancel_leave)"
    );
    assert_eq!(
        r.game.players[0].hand.len(),
        hand_before - FRAGMENT_N as usize,
        "hand shrank by N"
    );
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_before + FRAGMENT_N as usize,
        "trash gained N"
    );
}

#[test]
fn fragment_n_too_few_cards_does_not_offer() {
    // Hand size 1 — condition gates the outer accept off.
    let mut r = DebugRunner::builder()
        .add_card(fragment_card("FRAG"))
        .add_card(filler_card("FILL1"))
        .hand(0, &["FILL1"])
        .start();
    r.register_effect("FRAG", Arc::new(FragmentLike));
    let frag = r.place_on_field(0, "FRAG", None);
    let hand_size = r.game.players[0].hand.len();
    assert_eq!(hand_size, 1, "preconditions: 1 card in hand (≤ N)");

    r.game.delete_permanent_with_effects(frag);

    // Outer accept should NOT have been offered (condition false: 1 ≤ N).
    // The deletion should have proceeded directly.
    assert!(
        r.game.pending_selection.is_none(),
        "no PendingSelection — condition gated the outer accept"
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "FRAG was deleted normally (no Fragment offered)"
    );
}
