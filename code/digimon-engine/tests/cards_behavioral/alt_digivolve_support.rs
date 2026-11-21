//! Shared fixtures for the "alternate digivolution printed only in `xros_req`"
//! cards (EX11-007 Agumon, BT20-038 Falcomon, BT11-043 KingSukamon, EX11-025
//! FunBeemon, BT22-044 Palmon, BT22-046 Gargomon, BT16-070 Sethmon).
//!
//! Each of these prints a standard digivolution circle AND a second, cheaper
//! special route ("Digivolve [Koromon]: Cost 0", "Lv.2 w/[CS] trait: Cost 0",
//! "Lv.4 w/[Sukamon] in name: Cost 3", ...). cards.json carries that second
//! route only as the free-text `xros_req`, so before these cards had a YAML
//! spec the engine saw ONE cost route: a base satisfying both routes never got
//! DCGO's "Which digivolution cost do you pay?" prompt, and a base satisfying
//! only the special route had no legal digivolve at all (CR 4.3 17-1).
//!
//! The helpers drive the REAL production YAML (`dsl_card`) over synthetic
//! bases — no recording data.

use digimon_engine::action::{build_action_mask, space::encode_digivolve};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::selection::SelectionKind;

/// A synthetic field Digimon: `level`, one or more colours, named `name`,
/// carrying `traits`.
pub fn base(id: &str, name: &str, level: u8, colors: &[CardColor], traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c.colors = colors.to_vec();
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// Runner with `card_id` (production YAML) in hand slot 0 and `base_card` on
/// the field (slot returned), main phase, plenty of memory, a filler deck.
pub fn runner(card_id: &str, base_card: CardData) -> (DebugRunner, usize) {
    let base_id = base_card.card_id.clone();
    let mut filler = make_test_card("FILL", "Filler");
    filler.level = Some(3);
    filler.dp = Some(3000);
    let mut r = DebugRunner::builder()
        .dsl_card(card_id)
        .unwrap_or_else(|e| panic!("{card_id} must load from the embedded pack: {e}"))
        .add_card(base_card)
        .add_card(filler)
        .hand(0, &[card_id])
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(15)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, &base_id, Some(0)).index as usize;
    (r, slot)
}

/// Is the digivolve of hand slot 0 onto field `slot` offered by the action mask?
pub fn digivolve_offered(r: &DebugRunner, slot: usize) -> bool {
    build_action_mask(&r.game, 0)[encode_digivolve(0, slot as u16) as usize] > 0.0
}

/// The base reaches NO route of `card_id`: the mask must not offer the digivolve.
pub fn assert_not_offered(card_id: &str, base_card: CardData) {
    let (r, slot) = runner(card_id, base_card);
    assert!(
        !digivolve_offered(&r, slot),
        "{card_id}: this base satisfies no printed digivolution route — the mask must not offer it"
    );
}

/// The base reaches exactly one distinct-cost route: the mask offers it, the
/// digivolve raises NO cost-choice prompt, and it pays exactly `cost`.
/// Only for cards whose When-Digivolving effect does not change memory or park
/// a selection on the staged board.
pub fn assert_single_route(card_id: &str, base_card: CardData, cost: i16) {
    let (mut r, slot) = runner(card_id, base_card);
    assert!(
        digivolve_offered(&r, slot),
        "{card_id}: the printed route must be offered by the mask"
    );
    let before = r.game.memory;
    r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(
        !matches!(r.pending_kind(), Some(SelectionKind::EffectChoice)),
        "{card_id}: a single distinct-cost route must not raise a cost-choice prompt"
    );
    assert_eq!(
        r.game.players[0].battle_area[slot]
            .top_card()
            .card_id(&r.game.card_data),
        card_id,
        "{card_id}: the evolver must land on top"
    );
    assert_eq!(
        before - r.game.memory,
        cost,
        "{card_id}: the lone applicable route costs {cost} memory"
    );
}

fn cost_token(cost: i16) -> String {
    format!("cost {cost}")
}

/// The base reaches TWO distinct-cost routes (circle vs special alt): the
/// digivolve PAUSES on an `EffectChoice` with exactly the `{cheaper, costlier}`
/// options (DCGO CostList -> SelectCountEffect "Which digivolution cost do you
/// pay?", CR 4.3 17-1), and picking each option pays exactly that cost.
pub fn assert_cost_choice(card_id: &str, base_card: CardData, cheaper: i16, costlier: i16) {
    let (mut r, slot) = runner(card_id, base_card.clone());
    assert!(
        digivolve_offered(&r, slot),
        "{card_id}: the digivolve must be offered"
    );
    let proceeded = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(
        !proceeded,
        "{card_id}: base satisfies BOTH routes — digivolve must PAUSE for a cost choice, \
         not auto-pay the cheaper ({cheaper})"
    );
    let view = r
        .pending_selection_view()
        .unwrap_or_else(|| panic!("{card_id}: a cost-choice selection must be pending"));
    assert_eq!(view.kind, SelectionKind::EffectChoice, "{card_id}: cost choice");
    let choices = view
        .effect_choices
        .as_ref()
        .unwrap_or_else(|| panic!("{card_id}: the cost choice must carry effect_choices"));
    let labels: Vec<String> = choices.iter().map(|c| c.label.to_lowercase()).collect();
    assert_eq!(
        choices.len(),
        2,
        "{card_id}: exactly {{{cheaper}, {costlier}}} must be offered; got {labels:?}"
    );
    assert!(labels.iter().any(|l| l.contains(&cost_token(cheaper))), "{labels:?}");
    assert!(labels.iter().any(|l| l.contains(&cost_token(costlier))), "{labels:?}");

    let cheap = pick_returns_memory(card_id, base_card.clone(), cheaper);
    let dear = pick_returns_memory(card_id, base_card, costlier);
    assert_eq!(
        cheap - dear,
        costlier - cheaper,
        "{card_id}: choosing cost {costlier} must cost exactly {} more than cost {cheaper}",
        costlier - cheaper
    );
}

fn pick_returns_memory(card_id: &str, base_card: CardData, cost: i16) -> i16 {
    let (mut r, slot) = runner(card_id, base_card);
    assert!(!r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand));
    let view = r.pending_selection_view().expect("cost choice pending");
    let choices = view.effect_choices.clone().expect("effect_choices");
    let token = cost_token(cost);
    let opt = choices
        .iter()
        .find(|c| c.label.to_lowercase().contains(&token))
        .unwrap_or_else(|| panic!("{card_id}: an option labelled with cost {cost}"));
    r.execute_action(0, opt.action_id)
        .unwrap_or_else(|e| panic!("{card_id}: resolve cost choice (cost {cost}): {e}"));
    assert_eq!(
        r.game.players[0].battle_area[slot]
            .top_card()
            .card_id(&r.game.card_data),
        card_id,
        "{card_id}: the evolver must be stacked on top after the digivolve resolves"
    );
    r.game.memory
}

/// Convenience colour sets.
pub const RED: CardColor = CardColor::Red;
pub const BLUE: CardColor = CardColor::Blue;
pub const GREEN: CardColor = CardColor::Green;
pub const YELLOW: CardColor = CardColor::Yellow;
pub const BLACK: CardColor = CardColor::Black;
pub const PURPLE: CardColor = CardColor::Purple;
pub const WHITE: CardColor = CardColor::White;
