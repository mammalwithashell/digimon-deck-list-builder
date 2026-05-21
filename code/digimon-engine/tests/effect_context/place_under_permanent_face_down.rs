//! Task A1.1 — `Game::place_as_bottom_source` (and its private
//! `place_as_bottom_source_observed` helper) gain a `face_down: bool` axis.
//!
//! Today only `<Training>` can write face-down digivolution sources. This
//! generalizes the engine helper so future Tamer-stash callers can place a
//! face-down source under any permanent (battle-area Digimon or Tamer).
//!
//! Behavioral contract:
//!   - `place_as_bottom_source(source, target, true)`  → inserted
//!     `CardSource.face_down == true`.
//!   - `place_as_bottom_source(source, target, false)` → inserted
//!     `CardSource.face_down == false` (preserves prior face-up default).

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, CardSourceRef};
use digimon_engine::permanent::{Permanent, PermanentHandle};

fn make_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

fn make_tamer(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
    }
}

/// Push a card into player 0's hand directly, returning its handle.
fn push_to_hand(r: &mut DebugRunner, card_id: &str) -> CardHandle {
    let g = r.game_mut();
    let data_idx = g
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_hand: unknown card_id {}", card_id));
    let card_idx = g.next_card_index();
    let card = CardSource::new(data_idx, 0, card_idx);
    let handle = card.handle();
    g.players[0].hand.push(card);
    handle
}

/// Seed a single-card permanent on player 0's battle area.
fn seed_permanent(r: &mut DebugRunner, card_id: &str) -> PermanentHandle {
    let g = r.game_mut();
    let turn = g.turn_count;
    let data_idx = g
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("seed_permanent: unknown card_id {}", card_id));
    let card_idx = g.next_card_index();
    let card = CardSource::new(data_idx, 0, card_idx);
    g.players[0].battle_area.push(Permanent::new(card, turn));
    PermanentHandle {
        player: 0,
        index: (g.players[0].battle_area.len() - 1) as u8,
    }
}

/// Placing a deck-top card under a Tamer with `face_down: true` marks the
/// inserted source face-down.
#[test]
fn place_deck_top_under_tamer_face_down_sets_flag() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TAMER"))
        .add_card(make_digimon("STASH"))
        .deck(0, &["STASH"])
        .start();

    let tamer = seed_permanent(&mut r, "TAMER");
    assert_eq!(r.deck_size(0), 1, "deck has the stash card before");

    let ok = r
        .game_mut()
        .place_as_bottom_source(CardSourceRef::DeckTop(0), tamer, true);
    assert!(ok, "place_as_bottom_source should succeed");
    assert_eq!(r.deck_size(0), 0, "deck emptied after placement");

    let perm = &r.game.player(0).battle_area[tamer.index as usize];
    assert_eq!(perm.card_sources.len(), 2, "tamer now has 2 sources");
    let bottom = &perm.card_sources[0];
    assert_eq!(
        bottom.card_id(&r.game.card_data),
        "STASH",
        "stash card lands at the bottom of the tamer stack"
    );
    assert!(
        bottom.face_down,
        "the placed source must be face-down when face_down=true was passed"
    );
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "TAMER",
        "tamer's own card stays at the top"
    );
}

/// Placing a hand card under a Digimon with `face_down: false` leaves the
/// inserted source face-up (preserves the prior default behavior).
#[test]
fn place_hand_card_under_digimon_face_up_leaves_flag_clear() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("FUEL"))
        .hand(0, &["FUEL"])
        .start();

    let base = seed_permanent(&mut r, "BASE");

    let ok = r
        .game_mut()
        .place_as_bottom_source(CardSourceRef::Hand(0, 0), base, false);
    assert!(ok, "place_as_bottom_source should succeed");
    assert_eq!(r.hand_size(0), 0, "hand emptied after placement");

    let perm = &r.game.player(0).battle_area[base.index as usize];
    assert_eq!(perm.card_sources.len(), 2, "base now has 2 sources");
    let bottom = &perm.card_sources[0];
    assert_eq!(
        bottom.card_id(&r.game.card_data),
        "FUEL",
        "fuel card lands at the bottom of the stack"
    );
    assert!(
        !bottom.face_down,
        "the placed source must stay face-up when face_down=false was passed"
    );
}

/// Task A1.2 — `EffectContext::place_card_under_permanent_bottom` gains a
/// `face_down` axis. Tucking a hand card under a Digimon with `face_down: true`
/// marks the inserted source face-down.
#[test]
fn place_card_under_permanent_bottom_face_down_sets_flag() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BASE"))
        .add_card(make_digimon("TUCK"))
        .start();

    let base = seed_permanent(&mut r, "BASE");
    let tuck = push_to_hand(&mut r, "TUCK");
    assert_eq!(r.hand_size(0), 1, "hand has the tuck card before");

    {
        let mut ctx = EffectContext::new(&mut r.game, tuck, None, 0);
        ctx.place_card_under_permanent_bottom(tuck, base, true);
    }

    assert_eq!(r.hand_size(0), 0, "hand emptied after placement");

    let perm = &r.game.player(0).battle_area[base.index as usize];
    assert_eq!(perm.card_sources.len(), 2, "base now has 2 sources");
    let bottom = &perm.card_sources[0];
    assert_eq!(
        bottom.card_id(&r.game.card_data),
        "TUCK",
        "tuck card lands at the bottom of the stack"
    );
    assert!(
        bottom.face_down,
        "the placed source must be face-down when face_down=true was passed"
    );
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "BASE",
        "base's own card stays at the top"
    );
}
