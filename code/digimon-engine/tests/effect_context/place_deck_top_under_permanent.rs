//! Task A2.1 — `EffectContext::place_deck_top_under_permanent` convenience
//! helper.
//!
//! Generalizes the `<Training>`-keyword helper
//! `training_place_deck_top_under_self_face_down` to an arbitrary target
//! permanent (Tamer or Digimon, in either player's battle area).
//!
//! Behavioral contract:
//!   - Pops the top card of `target.player`'s deck and inserts it as the
//!     BOTTOM digivolution source of `target`.
//!   - `face_down: true`  → inserted `CardSource.face_down == true`.
//!   - Returns `Some(card_handle)` on success, `None` when the controller's
//!     deck is empty (silent no-op, mirroring `Player::draw`).

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::{Permanent, PermanentHandle};

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
        also_treated_as: Vec::new(),
    }
}

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
        also_treated_as: Vec::new(),
    }
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

/// Placing the deck top under a chosen Tamer with `face_down: true` returns
/// `Some(_)` and marks the inserted source face-down.
#[test]
fn place_deck_top_under_chosen_tamer_face_down() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TAMER"))
        .add_card(make_digimon("STASH"))
        .deck(0, &["STASH"])
        .start();

    let tamer = seed_permanent(&mut r, "TAMER");
    assert_eq!(r.deck_size(0), 1, "deck has the stash card before");

    let tamer_card = r.game.player(0).battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let placed = {
        let mut ctx = EffectContext::new(&mut r.game, tamer_card, Some(tamer), 0);
        ctx.place_deck_top_under_permanent(tamer, true)
    };

    assert!(
        placed.is_some(),
        "place_deck_top_under_permanent returns Some on success"
    );
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
        bottom.handle(),
        placed.unwrap(),
        "returned handle matches the placed source"
    );
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "TAMER",
        "tamer's own card stays at the top"
    );
}

/// Placing the deck top under a chosen Tamer with `face_down: false` returns
/// `Some(_)` and leaves the inserted source face-up (preserves the prior
/// face-up default).
#[test]
fn place_deck_top_under_permanent_face_up_leaves_flag_clear() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TAMER"))
        .add_card(make_digimon("STASH"))
        .deck(0, &["STASH"])
        .start();

    let tamer = seed_permanent(&mut r, "TAMER");
    assert_eq!(r.deck_size(0), 1, "deck has the stash card before");

    let tamer_card = r.game.player(0).battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let placed = {
        let mut ctx = EffectContext::new(&mut r.game, tamer_card, Some(tamer), 0);
        ctx.place_deck_top_under_permanent(tamer, false)
    };

    assert!(
        placed.is_some(),
        "place_deck_top_under_permanent returns Some on success"
    );
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
        !bottom.face_down,
        "the placed source must stay face-up when face_down=false was passed"
    );
    assert_eq!(
        bottom.handle(),
        placed.unwrap(),
        "returned handle matches the placed source"
    );
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "TAMER",
        "tamer's own card stays at the top"
    );
}

/// With an empty deck, `place_deck_top_under_permanent` returns `None` and
/// leaves the target's stack untouched.
#[test]
fn place_deck_top_under_permanent_empty_deck_returns_none() {
    let mut r = DebugRunner::builder()
        .add_card(make_tamer("TAMER"))
        .start();

    let tamer = seed_permanent(&mut r, "TAMER");
    assert_eq!(r.deck_size(0), 0, "deck is empty before");

    let tamer_card = r.game.player(0).battle_area[tamer.index as usize]
        .top_card()
        .handle();
    let placed = {
        let mut ctx = EffectContext::new(&mut r.game, tamer_card, Some(tamer), 0);
        ctx.place_deck_top_under_permanent(tamer, true)
    };

    assert!(
        placed.is_none(),
        "place_deck_top_under_permanent returns None on empty deck"
    );

    let perm = &r.game.player(0).battle_area[tamer.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        1,
        "tamer stack unchanged — only its own card"
    );
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "TAMER",
        "tamer's own card remains"
    );
}
