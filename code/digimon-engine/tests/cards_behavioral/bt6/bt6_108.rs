//! BT6-108 Underworld's Call — Option, Purple, cost 4.
//!
//! Printed text (official Bandai DB — data/card_bundles/BT6-108.md; card image
//! agrees):
//!   When you trash this card in your hand using one of your effects, trigger
//!   <Draw 1>. (Draw 1 card from your deck.)
//!   [Main] You may play 1 purple level 4 or lower Digimon card from your
//!   trash without paying its memory cost.
//!   [Security] Activate this card's [Main] effect.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT6/Purple/BT6_108.cs —
//! OnDiscardHand `CanTriggerOnTrashSelfHand(hashtable, IsOwnerEffect, card)`
//! (this card, trashed from the hand by an effect of its owner) → DrawClass 1;
//! OptionSkill: canNoSelect pick over IsDigimon && purple && HasLevel &&
//! Level <= 4 && CanPlayAsNewPermanent(payCost false) → PlayPermanentCards
//! (Root.Trash, free); SecuritySkill = AddActivateMainOptionSecurityEffect.
//!
//! Pattern rows: self "trashed from the hand" trigger (BT26-069 idiom) with an
//! own-effect gate, Option [Main] free play from trash, [Security] re-runs
//! [Main].

#[path = "../bt26/support.rs"]
mod support;

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};
use support::*;

const CARD_ID: &str = "BT6-108";

/// Test-only [On Play] "trash 1 card in your opponent's hand" — an effect
/// controlled by the OTHER player that trashes this card's owner's hand.
const OPP_HAND_TRASHER_YAML: &str = r#"
card: T-OPPHANDTRASH
name: OppHandTrasher
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] Trash 1 card in your opponent's hand"
    process:
      - select_hand:
          of: opponent
          bind_as: gone
          filter: {}
          prompt: "Trash 1 card in your opponent's hand"
      - trash_from_hand_by_index: { of: opponent, hand_index: gone }
"#;

/// Test-only [On Play] "By trashing 1 card in your hand, gain 1 memory" — a
/// cost-style ("by trashing", CR 4.3 §15-7) hand trash by your own effect.
const COST_TRASHER_YAML: &str = r#"
card: T-COSTTRASH
name: CostTrasher
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
traits: []
effects:
  - when: on_play
    summary: "[On Play] By trashing 1 card in your hand, gain 1 memory"
    process:
      - select_hand:
          of: you
          bind_as: paid
          optional: true
          cost: true
          filter: {}
          prompt: "By trashing 1 card in your hand, gain 1 memory"
      - trash_from_hand_by_index: { of: you, hand_index: paid }
      - gain_memory: 1
"#;

fn purple_option() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("PURPLE-OPT", "Purple Option");
    c.card_kind = CardKind::Option;
    c.colors = vec![CardColor::Purple];
    c.level = None;
    c.dp = None;
    c.play_cost = 1;
    c
}

fn setup() -> DebugRunner {
    let mut attacker = digimon("ATTACKER", "Attacker", CardColor::Red, 4, 5, &[]);
    attacker.dp = Some(9000);
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT6-108")
        .from_dsl_yaml(HAND_TRASHER_YAML)
        .expect("trasher")
        .from_dsl_yaml(OPP_HAND_TRASHER_YAML)
        .expect("opp trasher")
        .from_dsl_yaml(COST_TRASHER_YAML)
        .expect("cost trasher")
        .add_card(filler("FILLER"))
        .add_card(attacker)
        .add_card(digimon("PURPLE4", "Purple Four", CardColor::Purple, 4, 6, &[]))
        .add_card(digimon("PURPLE3", "Purple Three", CardColor::Purple, 3, 4, &[]))
        .add_card(digimon("PURPLE5", "Purple Five", CardColor::Purple, 5, 8, &[]))
        .add_card(digimon("RED3", "Red Three", CardColor::Red, 3, 4, &[]))
        .add_card(purple_option())
        .add_card(digimon("HOSTP", "Purple Host", CardColor::Purple, 3, 3, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(10)
        .start();
    r.set_first_player(0);
    r
}

/// Main phase with a purple Digimon in play, so the purple Option's colour
/// requirement is met and it can be used from the hand.
fn ready_to_use(r: &mut DebugRunner) {
    r.skip_mulligan();
    r.game.turn_player_idx = 0;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, "HOSTP", Some(0));
}

fn hand_idx(r: &DebugRunner, p: u8, id: &str) -> usize {
    r.game.players[p as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} not in hand"))
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt6_108_is_purple_option_cost_4_with_three_clauses() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled BT6-108");
    assert_eq!(card.kind, CompiledCardKind::Option);
    assert_eq!(card.cost, Some(4));
    assert!(card.color.contains(&digimon_dsl::compiled::CompiledColor::Purple));
    let trig: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(trig.len(), 3);
    let draw = trig
        .iter()
        .find(|t| t.scope == CompiledScope::Trash)
        .expect("trashed-from-hand clause");
    assert_eq!(draw.when, vec![CompiledTiming::OnDiscardHand]);
    assert!(!draw.optional && !draw.once_per_turn, "mandatory, no per-turn cap");
    let main = trig
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("[Main]");
    assert_eq!(main.scope, CompiledScope::FaceUp);
    let sec = trig
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("[Security]");
    assert_eq!(sec.scope, CompiledScope::Inherited);
}

// ─── Section 2 — When trashed from the hand by your effect → <Draw 1> ────────

#[test]
fn bt6_108_trashed_from_hand_by_own_effect_draws_one() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    let deck_before = r.deck_size(0);
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, CARD_ID);
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(r.deck_size(0), deck_before - 1, "<Draw 1>");
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt6_108_trashed_from_hand_as_your_effects_cost_draws_one() {
    // "By trashing 1 card in your hand, ..." is still trashing it from the
    // hand using one of your effects (DCGO: the discard goes through the same
    // OnDiscardHand / IsOwnerEffect path).
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    r.game.memory = 3; // below the cap, so the +1 is observable
    let deck_before = r.deck_size(0);
    let memory_before = r.memory();
    let t = r.place_on_field(0, "T-COSTTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    let v = r.pending_selection_view().expect("the optional cost pick");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, CARD_ID);
    let _ = r.auto_resolve();
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(r.memory(), memory_before + 1, "the cost was paid: +1 memory");
    assert_eq!(r.deck_size(0), deck_before - 1, "<Draw 1>");
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt6_108_trashing_another_card_does_not_draw() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    let deck_before = r.deck_size(0);
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before, "only THIS card being trashed draws");
}

#[test]
fn bt6_108_already_in_trash_does_not_draw_on_other_hand_trash() {
    let mut r = setup();
    push_trash(&mut r, 0, CARD_ID);
    push_hand(&mut r, 0, "FILLER");
    let deck_before = r.deck_size(0);
    let t = r.place_on_field(0, "T-HANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), deck_before);
}

#[test]
fn bt6_108_opponents_effect_trashing_it_does_not_draw() {
    let mut r = setup();
    push_hand(&mut r, 0, CARD_ID);
    let deck_before = r.deck_size(0);
    let t = r.place_on_field(1, "T-OPPHANDTRASH", Some(0));
    fire(&mut r, EffectTiming::OnPlay, t);
    drain_first(&mut r);
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()), "it was trashed");
    assert_eq!(r.deck_size(0), deck_before, "not \"one of YOUR effects\": no draw");
}

#[test]
fn bt6_108_using_the_option_is_not_a_hand_trash() {
    let mut r = setup();
    ready_to_use(&mut r);
    let idx = r.add_to_hand(0, CARD_ID);
    let deck_before = r.deck_size(0);
    // Empty trash: nothing to play; the card resolves and goes to the trash.
    let res = r.game.play_option_from_hand(0, idx);
    assert!(
        matches!(res, OptionPlayResult::Trashed | OptionPlayResult::Pending),
        "{res:?}"
    );
    drain_first(&mut r);
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()));
    assert_eq!(r.deck_size(0), deck_before, "using it is not trashing it from the hand");
}

// ─── Section 3 — [Main] ───────────────────────────────────────────────────────

#[test]
fn bt6_108_main_plays_purple_level_4_or_lower_from_trash_free() {
    let mut r = setup();
    for id in ["PURPLE4", "PURPLE3", "PURPLE5", "RED3", "PURPLE-OPT"] {
        push_trash(&mut r, 0, id);
    }
    ready_to_use(&mut r);
    let idx = r.add_to_hand(0, CARD_ID);
    assert_eq!(r.game.play_option_from_hand(0, idx), OptionPlayResult::Pending);
    assert_eq!(r.memory(), 10 - 4, "the Option's use cost is paid");
    let v = r.pending_selection_view().expect("trash pick");
    assert_eq!(v.kind, SelectionKind::Trash);
    assert!(v.is_optional, "\"You may play\"");
    assert_eq!(non_pass(&r).len(), 2, "PURPLE4 and PURPLE3 only (not Lv.5, red, or an Option)");
    pick_trash(&mut r, 0, "PURPLE4");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"PURPLE4".to_string()));
    assert!(!trash_ids(&r, 0).contains(&"PURPLE4".to_string()));
    assert_eq!(r.memory(), 10 - 4, "played without paying its memory cost");
    assert!(trash_ids(&r, 0).contains(&CARD_ID.to_string()), "the Option is trashed after use");
}

#[test]
fn bt6_108_main_decline_plays_nothing() {
    let mut r = setup();
    push_trash(&mut r, 0, "PURPLE3");
    ready_to_use(&mut r);
    let idx = r.add_to_hand(0, CARD_ID);
    assert_eq!(r.game.play_option_from_hand(0, idx), OptionPlayResult::Pending);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"PURPLE3".to_string()));
    assert!(trash_ids(&r, 0).contains(&"PURPLE3".to_string()));
}

// ─── Section 4 — [Security] activates the [Main] effect ──────────────────────

#[test]
fn bt6_108_security_plays_purple_level_4_or_lower_from_trash_free() {
    let mut r = setup();
    r.game.players[1].security.clear();
    {
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == CARD_ID)
            .unwrap();
        let next = r.game.next_card_index();
        r.game.players[1]
            .security
            .push(digimon_engine::card_source::CardSource::new(data_idx, 1, next));
    }
    for id in ["PURPLE3", "PURPLE5", "RED3"] {
        push_trash(&mut r, 1, id);
    }
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    r.attack_player(attacker, 1, false);
    let v = r.pending_selection_view().expect("[Security] trash pick");
    assert_eq!(v.selecting_player, 1, "the defender resolves its security effect");
    assert!(v.is_optional, "\"You may play\"");
    assert_eq!(non_pass(&r).len(), 1, "only PURPLE3");
    pick_trash(&mut r, 1, "PURPLE3");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"PURPLE3".to_string()));
    assert!(
        trash_ids(&r, 1).contains(&CARD_ID.to_string()),
        "the Option goes to the trash after its [Security] effect"
    );
    assert!(r.game.players[1].security.is_empty());
    let _ = hand_idx;
}
