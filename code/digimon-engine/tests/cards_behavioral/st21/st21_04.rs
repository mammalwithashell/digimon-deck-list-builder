//! ST21-04 Zudomon — Digimon, Lv.5, Blue, DP 7000, Cost 7.
//! Traits: Sea Beast / ADVENTURE. Form: Ultimate. Attribute: Vaccine.
//!
//! # Card text (data/card_bundles/ST21-04.md — official Bandai DB; cards.json
//! # wording "From 1 of your opponent's Digimon" for the first sentence)
//!
//! ```text
//! Digivolve: Blue Lv.4 / cost 3
//! [Digivolve] Lv.4 w/[ADVENTURE] trait: Cost 3
//! [On Play] [When Digivolving] From 1 of your opponent's Digimon, trash any 1
//! digivolution card for every 2 colors your Tamers have. Then, return 1 of
//! their Digimon with 1 or fewer digivolution cards to the hand.
//! [Your Turn] [Once Per Turn] When your other Digimon are played or
//! digivolve, if any of them have the [ADVENTURE] trait, 1 of your Digimon
//! gains <Alliance> for the turn. Then, 1 of your Digimon may attack.
//! Inherited Effect
//! <Alliance> (When this Digimon attacks, by suspending 1 of your other
//! Digimon, this Digimon gains <Security A. +1> and the suspended Digimon's DP
//! for the attack.)
//!
//! Official Q&A (Alliance grant): "No, you can't. You must give to 1 of your
//! Digimon."
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/ST21/Blue/ST21_04.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - "[Digivolve] Lv.4 w/[ADVENTURE] trait: Cost 3" alt-path (ST20-10 exemplar)
//! - Shared [On Play]/[When Digivolving]: strip `floor(Tamer colors / 2)`
//!   digivolution cards from ONE chosen opponent Digimon (AD1-014 floor_div,
//!   EX11-057 clamp_to_available idiom), then a SEPARATE mandatory bounce pick
//!   (opp Digimon with <= 1 digivolution cards) evaluated AFTER the strip
//! - [Your Turn][OPT] ally play/digivolve observer gated on an [ADVENTURE]
//!   event target (BT16-017 `trigger_if` idiom): mandatory <Alliance> grant to
//!   one of YOUR Digimon for the turn, then an optional "may attack" pick
//!   (BT21-044 idiom)
//! - Inherited <Alliance> keyword through the digivolution stack
//! - Event-log cost test (digivolve cost 3 → MemoryChange -3)
//!
//! Least-certain assertions (could not be executed when authored): the exact
//! prompt count of the multi-card source pick, and the accept-path of the
//! optional "may attack" tail.

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledColor, CompiledCost,
    CompiledDeclarativeClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, Keyword, ModifierType, PlaySource};
use digimon_engine::events::GameEvent;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{PendingSelectionView, SelectionKind};

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-04";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn dig(id: &str, color: CardColor, level: u8, dp: i32, adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 1;
    c.colors = vec![color];
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        Vec::new()
    };
    c
}

fn tamer(id: &str, colors: &[CardColor], adventure: bool) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 1;
    c.colors = colors.to_vec();
    c.traits = if adventure {
        vec!["ADVENTURE".to_string()]
    } else {
        Vec::new()
    };
    c
}

fn base_builder() -> DebugRunnerBuilder {
    let mut adv4 = dig("ADV4", CardColor::Red, 4, 4000, true);
    adv4.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 0,
    }];
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-04 in embedded DSL pack")
        .add_card(dig("OPP", CardColor::Red, 4, 5000, false))
        .add_card(dig("S1", CardColor::Red, 3, 3000, false))
        .add_card(dig("S2", CardColor::Red, 3, 3000, false))
        .add_card(dig("S3", CardColor::Red, 3, 3000, false))
        .add_card(dig("S4", CardColor::Red, 3, 3000, false))
        .add_card(dig("BLUE4", CardColor::Blue, 4, 4000, false))
        .add_card(dig("RED4-ADV", CardColor::Red, 4, 4000, true))
        .add_card(dig("RED4-PLAIN", CardColor::Red, 4, 4000, false))
        .add_card(dig("BLUE3-ADV", CardColor::Blue, 3, 3000, true))
        .add_card(dig("ALLY", CardColor::Red, 3, 3000, true))
        .add_card(dig("ALLY2", CardColor::Red, 3, 3000, true))
        .add_card(dig("ALLY3", CardColor::Red, 3, 3000, true))
        .add_card(dig("PLAIN-ALLY", CardColor::Red, 3, 3000, false))
        .add_card(dig("BASE3", CardColor::Red, 3, 3000, false))
        .add_card(adv4)
        .add_card(dig("CARRIER", CardColor::Red, 5, 6000, false))
        .add_card(tamer("T-RED", &[CardColor::Red], false))
        .add_card(tamer("T-RED2", &[CardColor::Red], false))
        .add_card(tamer("T-BLUE", &[CardColor::Blue], false))
        .add_card(tamer("T-GREEN", &[CardColor::Green], false))
        .add_card(tamer("T-YELLOW", &[CardColor::Yellow], false))
        .add_card(tamer("T-DUAL", &[CardColor::Red, CardColor::Blue], false))
        .add_card(tamer("ADV-TAMER", &[CardColor::Red], true))
        .add_card(make_test_card("FILLER", "Filler"))
        .add_card(dig("SEC", CardColor::Red, 3, 1000, false))
}

fn runner() -> DebugRunner {
    let mut r = base_builder()
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .security(0, &["SEC", "SEC"])
        .security(1, &["SEC", "SEC"])
        .memory(10)
        .start();
    r.set_first_player(0);
    r
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn field_pick(h: PermanentHandle) -> u16 {
    encode_attack(0, h.index as u16)
}

fn pick(r: &mut DebugRunner, h: PermanentHandle) {
    let v = r.pending_selection_view().expect("selection pending");
    let a = field_pick(h);
    assert!(
        v.valid_action_ids.contains(&a),
        "{h:?} not a legal pick: {:?}",
        v.valid_action_ids
    );
    r.execute_action(v.selecting_player, a).unwrap();
}

fn source_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    let cards = &r.game.players[h.player as usize].battle_area[h.index as usize].card_sources;
    cards[..cards.len() - 1]
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn trashed_sources(r: &DebugRunner, player: u8) -> usize {
    r.game.players[player as usize]
        .trash
        .iter()
        .filter(|c| {
            let id = c.card_id(&r.game.card_data);
            id == "S1" || id == "S2" || id == "S3" || id == "S4"
        })
        .count()
}

fn in_hand(r: &DebugRunner, player: u8, id: &str) -> bool {
    r.game.players[player as usize]
        .hand
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

/// Add Zudomon to P0's hand and play it (fires [On Play]).
fn play_zudo(r: &mut DebugRunner) -> PermanentHandle {
    let hi = r.add_to_hand(0, CARD_ID);
    let idx = r.play(0, hi).expect("Zudomon plays from hand");
    PermanentHandle {
        player: 0,
        index: idx as u8,
    }
}

/// Add `id` to P0's hand and play it, topping memory up first so the play
/// never ends the turn.
fn play_card(r: &mut DebugRunner, id: &str) -> PermanentHandle {
    r.game.set_memory(10);
    let hi = r.add_to_hand(0, id);
    let idx = r.play(0, hi).expect("card plays from hand");
    PermanentHandle {
        player: 0,
        index: idx as u8,
    }
}

/// Resolve prompts (first legal action) until the bounce prompt — recognised
/// by `expect` being among its legal picks — is reached, and return it.
fn drive_to_bounce(r: &mut DebugRunner, expect: PermanentHandle) -> PendingSelectionView {
    for _ in 0..8 {
        let v = r
            .pending_selection_view()
            .expect("a prompt must still be pending before the bounce pick");
        if v.valid_action_ids.contains(&field_pick(expect)) {
            return v;
        }
        let a = v.valid_action_ids.first().copied().unwrap_or(PASS);
        r.execute_action(v.selecting_player, a).unwrap();
    }
    panic!("the bounce prompt never appeared");
}

fn digivolve_from(base: &str) -> (bool, i16) {
    let mut r = runner();
    let h = r.place_on_field(0, base, Some(0));
    let hi = r.add_to_hand(0, CARD_ID);
    let before = r.memory();
    let ok = r
        .game
        .digivolve_from_hand(0, hi, h.index as usize, PlaySource::ByHand);
    (ok, before - r.memory())
}

fn triggered<'a>(
    card: &'a digimon_dsl::compiled::CompiledCard,
    f: impl Fn(&CompiledTriggeredClause) -> bool,
) -> Option<&'a CompiledTriggeredClause> {
    card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if f(t) => Some(t),
        _ => None,
    })
}

/// Opponent board used by the strip/bounce tests:
///  A — 4 digivolution cards (never bounce-eligible after a strip of <= 2)
///  B — 1 digivolution card  (bounce-eligible)
///  C — 0 digivolution cards (bounce-eligible)
fn opp_board(r: &mut DebugRunner) -> (PermanentHandle, PermanentHandle, PermanentHandle) {
    let a = r.place_stack(1, &["S1", "S2", "S3", "S4", "OPP"]);
    let b = r.place_stack(1, &["S1", "OPP"]);
    let c = r.place_on_field(1, "OPP", Some(0));
    (a, b, c)
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_04_identity_matches_printed_card() {
    let c = compiled(CARD_ID);
    assert_eq!(c.name, "Zudomon");
    assert_eq!(c.kind, CompiledCardKind::Digimon);
    assert_eq!(c.level, Some(5));
    assert_eq!(c.cost, Some(7));
    assert_eq!(c.dp, Some(7000));
    assert_eq!(c.color, vec![CompiledColor::Blue]);
    assert!(c.traits.iter().any(|t| t == "Sea Beast"), "{:?}", c.traits);
    assert!(c.traits.iter().any(|t| t == "ADVENTURE"), "{:?}", c.traits);
}

#[test]
fn st21_04_has_two_cost_three_requirement_respecting_digivolve_paths() {
    let c = compiled(CARD_ID);
    let paths: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(paths.len(), 2, "printed blue Lv.4 circle + ADVENTURE box");
    for p in &paths {
        assert_eq!(p.cost, Some(CompiledCost::Literal(3)));
        assert!(!p.ignore_requirements);
    }
    let std_from = paths[0].from.as_ref().expect("blue Lv.4 `from:`");
    assert_eq!(std_from.level_eq, Some(4));
    assert_eq!(std_from.color_is, Some(CompiledColor::Blue));
    let adv_from = paths[1].from.as_ref().expect("ADVENTURE Lv.4 `from:`");
    assert_eq!(adv_from.level_eq, Some(4));
    assert!(format!("{adv_from:?}").contains("ADVENTURE"));
}

#[test]
fn st21_04_clause_shapes() {
    let c = compiled(CARD_ID);

    let shared = triggered(&c, |t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play][When Digivolving] clause");
    assert!(shared.when.contains(&CompiledTiming::WhenDigivolving));
    assert_eq!(shared.scope, CompiledScope::FaceUp);
    assert!(!shared.optional, "the strip + bounce body is mandatory");
    assert!(!shared.once_per_turn);

    let your_turn = triggered(&c, |t| t.when.contains(&CompiledTiming::OnAllyPlayed))
        .expect("[Your Turn] ally clause");
    assert!(your_turn.when.contains(&CompiledTiming::OnDigivolve));
    assert!(your_turn.once_per_turn, "[Once Per Turn]");
    assert!(
        !your_turn.optional,
        "the Alliance grant is mandatory (only the trailing attack is a 'may')"
    );
    assert_eq!(your_turn.scope, CompiledScope::FaceUp);
    assert!(
        your_turn.active_when.is_some(),
        "[Your Turn] must gate the clause"
    );
}

#[test]
fn st21_04_inherited_alliance_is_a_scope_inherited_keyword_grant() {
    let c = compiled(CARD_ID);
    let found = c.effects.iter().any(|e| {
        matches!(
            e,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword.eq_ignore_ascii_case("Alliance")
        )
    });
    assert!(found, "an inherited-scope GrantKeyword Alliance clause");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolve conditions + cost event
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_04_digivolves_from_blue_lv4_for_three() {
    let (ok, paid) = digivolve_from("BLUE4");
    assert!(ok);
    assert_eq!(paid, 3);
}

#[test]
fn st21_04_digivolves_from_non_blue_lv4_with_adventure_for_three() {
    let (ok, paid) = digivolve_from("RED4-ADV");
    assert!(ok, "[Digivolve] Lv.4 w/[ADVENTURE] lifts the colour gate");
    assert_eq!(paid, 3);
}

#[test]
fn st21_04_cannot_digivolve_from_non_blue_lv4_without_adventure() {
    let (ok, _) = digivolve_from("RED4-PLAIN");
    assert!(!ok);
}

#[test]
fn st21_04_cannot_digivolve_from_lv3_adventure() {
    let (ok, _) = digivolve_from("BLUE3-ADV");
    assert!(!ok, "both circles need Lv.4");
}

#[test]
fn st21_04_digivolve_cost_event_is_three_memory() {
    let mut r = runner();
    let h = r.place_on_field(0, "RED4-ADV", Some(0));
    let hi = r.add_to_hand(0, CARD_ID);
    let cp = r.event_checkpoint();
    assert!(r
        .game
        .digivolve_from_hand(0, hi, h.index as usize, PlaySource::ByHand));
    assert!(
        r.events_since(cp)
            .iter()
            .any(|e| matches!(e, GameEvent::MemoryChange { delta: -3, .. })),
        "the digivolution cost of 3 must hit the event log"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play]: strip floor(Tamer colors / 2), then bounce
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_04_with_no_tamers_nothing_is_stripped_and_the_bounce_is_offered_first() {
    let mut r = runner();
    let (a, b, c) = opp_board(&mut r);
    play_zudo(&mut r);

    let v = r.pending_selection_view().expect("bounce pick");
    assert_eq!(v.selecting_player, 0);
    assert!(!v.is_optional, "the bounce pick is mandatory");
    assert!(
        v.valid_action_ids.contains(&field_pick(b)),
        "B has 1 source"
    );
    assert!(
        v.valid_action_ids.contains(&field_pick(c)),
        "C has 0 sources"
    );
    assert!(
        !v.valid_action_ids.contains(&field_pick(a)),
        "A has 4 digivolution cards (> 1) — not a legal bounce target"
    );
    assert_eq!(
        source_ids(&r, a).len(),
        4,
        "0 colors ⇒ nothing was stripped"
    );

    pick(&mut r, c);
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(1), 2, "C left the battle area");
    assert!(in_hand(&r, 1, "OPP"), "C returned to its owner's hand");
}

#[test]
fn st21_04_two_tamer_colors_strip_one_card_from_the_chosen_digimon_then_bounce() {
    let mut r = runner();
    r.place_on_field(0, "T-RED", Some(0));
    r.place_on_field(0, "T-BLUE", Some(0));
    let (a, b, c) = opp_board(&mut r);
    play_zudo(&mut r);

    let v = r.pending_selection_view().expect("strip host pick");
    assert!(!v.is_optional, "the strip host pick is mandatory");
    for h in [a, b, c] {
        assert!(
            v.valid_action_ids.contains(&field_pick(h)),
            "DCGO: ANY opponent Digimon may be chosen to strip from"
        );
    }
    pick(&mut r, a);

    let bounce = drive_to_bounce(&mut r, c);
    assert_eq!(
        source_ids(&r, a).len(),
        3,
        "2 colors / 2 = exactly 1 digivolution card trashed from A"
    );
    assert_eq!(
        trashed_sources(&r, 1),
        1,
        "exactly one source reached the trash"
    );
    assert!(bounce.valid_action_ids.contains(&field_pick(b)));
    assert!(
        !bounce.valid_action_ids.contains(&field_pick(a)),
        "A still has 3 digivolution cards"
    );
    pick(&mut r, c);
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(1), 2);
}

#[test]
fn st21_04_three_tamer_colors_round_down_to_one_card() {
    let mut r = runner();
    r.place_on_field(0, "T-RED", Some(0));
    r.place_on_field(0, "T-BLUE", Some(0));
    r.place_on_field(0, "T-GREEN", Some(0));
    let (a, _b, c) = opp_board(&mut r);
    play_zudo(&mut r);

    pick(&mut r, a);
    drive_to_bounce(&mut r, c);
    assert_eq!(
        source_ids(&r, a).len(),
        3,
        "floor(3 / 2) = 1 card stripped, not 2"
    );
}

#[test]
fn st21_04_four_tamer_colors_strip_two_cards_from_one_digimon() {
    let mut r = runner();
    r.place_on_field(0, "T-RED", Some(0));
    r.place_on_field(0, "T-BLUE", Some(0));
    r.place_on_field(0, "T-GREEN", Some(0));
    r.place_on_field(0, "T-YELLOW", Some(0));
    let (a, _b, c) = opp_board(&mut r);
    play_zudo(&mut r);

    pick(&mut r, a);
    drive_to_bounce(&mut r, c);
    assert_eq!(
        source_ids(&r, a).len(),
        2,
        "4 colors / 2 = 2 digivolution cards stripped, all from the SAME Digimon"
    );
    assert_eq!(trashed_sources(&r, 1), 2);
}

#[test]
fn st21_04_a_dual_color_tamer_counts_both_of_its_colors() {
    let mut r = runner();
    r.place_on_field(0, "T-DUAL", Some(0)); // Red + Blue = 2 colors
    let (a, _b, c) = opp_board(&mut r);
    play_zudo(&mut r);

    pick(&mut r, a);
    drive_to_bounce(&mut r, c);
    assert_eq!(source_ids(&r, a).len(), 3, "2 colors ⇒ 1 card stripped");
}

#[test]
fn st21_04_duplicate_tamer_colors_count_once() {
    let mut r = runner();
    r.place_on_field(0, "T-RED", Some(0));
    r.place_on_field(0, "T-RED2", Some(0)); // still 1 color
    let (a, b, c) = opp_board(&mut r);
    play_zudo(&mut r);

    let v = r.pending_selection_view().expect("bounce pick");
    assert!(
        v.valid_action_ids.contains(&field_pick(c)),
        "1 distinct color ⇒ 0 cards stripped ⇒ the first prompt is already the bounce"
    );
    assert_eq!(source_ids(&r, a).len(), 4, "nothing stripped");
}

#[test]
fn st21_04_choosing_a_digimon_without_digivolution_cards_strips_nothing() {
    let mut r = runner();
    r.place_on_field(0, "T-RED", Some(0));
    r.place_on_field(0, "T-BLUE", Some(0));
    let c = r.place_on_field(1, "OPP", Some(0)); // the only opponent Digimon, 0 sources
    play_zudo(&mut r);

    pick(&mut r, c); // strip host = C (legal per DCGO, strips nothing)
    let v = r.pending_selection_view().expect("bounce pick follows");
    assert!(v.valid_action_ids.contains(&field_pick(c)));
    pick(&mut r, c);
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(1), 0, "C was bounced");
    assert_eq!(trashed_sources(&r, 1), 0);
}

#[test]
fn st21_04_stripped_digimon_can_become_the_bounce_target() {
    // The bounce candidate set is evaluated AFTER the strip.
    let mut r = runner();
    r.place_on_field(0, "T-RED", Some(0));
    r.place_on_field(0, "T-BLUE", Some(0));
    let a = r.place_stack(1, &["S1", "S2", "OPP"]); // 2 sources
    play_zudo(&mut r);

    pick(&mut r, a); // strip 1 ⇒ A keeps 1 source (<= 1)
    let v = drive_to_bounce(&mut r, a);
    assert!(v.valid_action_ids.contains(&field_pick(a)));
    pick(&mut r, a);
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(1), 0, "A was bounced");
}

#[test]
fn st21_04_no_bounce_target_when_every_digimon_has_two_or_more_cards() {
    let mut r = runner();
    let a = r.place_stack(1, &["S1", "S2", "S3", "OPP"]); // 3 sources, no tamers ⇒ no strip
    play_zudo(&mut r);
    assert!(
        r.pending_selection().is_none(),
        "no opposing Digimon has 1 or fewer digivolution cards ⇒ nothing to return"
    );
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(source_ids(&r, a).len(), 3);
}

#[test]
fn st21_04_nothing_happens_when_opponent_has_no_digimon() {
    let mut r = runner();
    r.place_on_field(0, "T-RED", Some(0));
    r.place_on_field(0, "T-BLUE", Some(0));
    play_zudo(&mut r);
    assert!(r.pending_selection().is_none());
}

#[test]
fn st21_04_when_digivolving_runs_the_same_body() {
    let mut r = runner();
    r.place_on_field(0, "T-RED", Some(0));
    r.place_on_field(0, "T-BLUE", Some(0));
    let (a, _b, c) = opp_board(&mut r);
    let base = r.place_on_field(0, "BLUE4", Some(0));
    let hi = r.add_to_hand(0, CARD_ID);
    assert!(r
        .game
        .digivolve_from_hand(0, hi, base.index as usize, PlaySource::ByHand));

    let v = r
        .pending_selection_view()
        .expect("[When Digivolving] must install the strip host pick");
    assert!(!v.is_optional);
    pick(&mut r, a);
    drive_to_bounce(&mut r, c);
    assert_eq!(source_ids(&r, a).len(), 3);
    pick(&mut r, c);
    r.auto_resolve().unwrap();
    assert_eq!(r.battle_area_size(1), 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [Your Turn] [Once Per Turn] Alliance grant + optional attack
// ═══════════════════════════════════════════════════════════════════════════

/// Zudomon on the field (able to attack), nothing else.
fn ally_runner() -> (DebugRunner, PermanentHandle) {
    let mut r = runner();
    let z = r.place_on_field(0, CARD_ID, Some(0));
    (r, z)
}

#[test]
fn st21_04_adventure_ally_play_grants_alliance_to_a_chosen_digimon_then_offers_an_attack() {
    let (mut r, z) = ally_runner();
    let ally = play_card(&mut r, "ALLY");

    let v = r.pending_selection_view().expect("Alliance grant pick");
    assert_eq!(v.selecting_player, 0);
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(!v.is_optional, "the grant is mandatory — you must give it");
    assert!(
        v.valid_action_ids.contains(&field_pick(z)),
        "Zudomon itself"
    );
    assert!(
        v.valid_action_ids.contains(&field_pick(ally)),
        "the new ally"
    );

    pick(&mut r, ally);
    assert!(
        r.game.has_keyword(ally, Keyword::Alliance),
        "the chosen Digimon gains <Alliance>"
    );
    assert!(
        !r.game.has_keyword(z, Keyword::Alliance),
        "only the chosen Digimon gains it"
    );

    let v = r.pending_selection_view().expect("optional attack pick");
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(v.is_optional, "'may attack' — PASS must be legal");
    assert!(
        v.valid_action_ids.contains(&field_pick(z)),
        "Zudomon can attack"
    );
    assert!(
        !v.valid_action_ids.contains(&field_pick(ally)),
        "the just-played ally is summoning sick"
    );

    let sec_before = r.security_count(1);
    r.execute_action(0, PASS).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.security_count(1), sec_before, "declined: nobody attacked");
}

#[test]
fn st21_04_alliance_can_be_given_to_zudomon_itself() {
    let (mut r, z) = ally_runner();
    let ally = play_card(&mut r, "ALLY");
    pick(&mut r, z);
    assert!(r.game.has_keyword(z, Keyword::Alliance));
    assert!(!r.game.has_keyword(ally, Keyword::Alliance));
    let _ = r.execute_action(0, PASS);
}

#[test]
fn st21_04_granted_alliance_expires_at_end_of_turn() {
    let (mut r, z) = ally_runner();
    let ally = play_card(&mut r, "ALLY");
    pick(&mut r, ally);
    r.execute_action(0, PASS).unwrap();
    assert!(r.game.has_keyword(ally, Keyword::Alliance));

    r.end_turn();
    let _ = r.auto_resolve();
    assert!(
        !r.game.has_keyword(ally, Keyword::Alliance),
        "'for the turn' — the grant is gone after the turn ends"
    );
}

#[test]
fn st21_04_accepting_the_optional_attack_makes_zudomon_attack() {
    let (mut r, z) = ally_runner();
    let ally = play_card(&mut r, "ALLY");
    pick(&mut r, ally);
    let sec_before = r.security_count(1);

    // Accept the optional attacker pick (Zudomon), then keep accepting
    // whatever the attack flow asks until it is over.
    pick(&mut r, z);
    for _ in 0..30 {
        let Some(v) = r.pending_selection_view() else {
            break;
        };
        let a = v
            .valid_action_ids
            .iter()
            .copied()
            .find(|a| *a != PASS)
            .unwrap_or(PASS);
        r.execute_action(v.selecting_player, a).unwrap();
    }
    assert!(
        r.security_count(1) < sec_before || r.game_over(),
        "Zudomon attacked the opponent's security"
    );
}

#[test]
fn st21_04_non_adventure_ally_does_not_trigger_and_does_not_spend_the_opt() {
    let (mut r, z) = ally_runner();
    play_card(&mut r, "PLAIN-ALLY");
    assert!(
        r.pending_selection().is_none(),
        "no [ADVENTURE] trait ⇒ the clause is not triggered"
    );

    // The once-per-turn use must be unspent: a later [ADVENTURE] ally triggers.
    let ally = play_card(&mut r, "ALLY");
    assert!(
        r.pending_selection().is_some(),
        "the OPT was not consumed by the non-ADVENTURE ally"
    );
}

#[test]
fn st21_04_adventure_tamer_does_not_trigger() {
    let (mut r, z) = ally_runner();
    play_card(&mut r, "ADV-TAMER");
    assert!(
        r.pending_selection().is_none(),
        "'your other Digimon' — a Tamer with the trait is not a Digimon"
    );
}

#[test]
fn st21_04_zudomons_own_play_is_not_an_other_digimon() {
    let mut r = runner();
    r.place_on_field(0, "ALLY", Some(0)); // an ADVENTURE Digimon already in play
    play_zudo(&mut r);
    assert!(
        r.pending_selection().is_none(),
        "Zudomon entering must not trigger its own 'other Digimon' clause \
         (and with no opposing Digimon its [On Play] has no targets)"
    );
}

#[test]
fn st21_04_does_not_trigger_when_zudomon_is_not_on_the_battle_area() {
    let mut r = runner();
    r.add_to_hand(0, CARD_ID); // in hand only
    play_card(&mut r, "ALLY");
    assert!(r.pending_selection().is_none());
}

#[test]
fn st21_04_is_gated_to_your_turn() {
    let (mut r, z) = ally_runner();
    let ally = r.place_on_field(0, "ALLY", Some(0));
    r.game.turn_player_idx = 1; // the opponent's turn
    r.fire_play_event_triggers(0, ally.index as usize, true, false);
    assert!(
        r.pending_selection().is_none(),
        "[Your Turn] — an ally entering on the opponent's turn does not trigger it"
    );
}

#[test]
fn st21_04_once_per_turn_lockout_and_rearm_next_turn() {
    let (mut r, z) = ally_runner();
    play_card(&mut r, "ALLY");
    // Resolve: grant to Zudomon, decline the attack.
    pick(&mut r, z);
    r.execute_action(0, PASS).unwrap();
    assert!(r.pending_selection().is_none());

    play_card(&mut r, "ALLY2");
    assert!(
        r.pending_selection().is_none(),
        "[Once Per Turn]: a second [ADVENTURE] ally in the same turn does not trigger it"
    );

    // Two end_turns later it is my turn again and the use is back.
    r.end_turn();
    let _ = r.auto_resolve();
    r.end_turn();
    let _ = r.auto_resolve();
    assert_eq!(r.turn_player(), 0);
    play_card(&mut r, "ALLY3");
    assert!(
        r.pending_selection().is_some(),
        "the once-per-turn use re-arms on my next turn"
    );
}

#[test]
fn st21_04_adventure_digimon_digivolving_triggers_it_too() {
    let (mut r, z) = ally_runner();
    let base = r.place_on_field(0, "BASE3", Some(0));
    let hi = r.add_to_hand(0, "ADV4");
    assert!(r
        .game
        .digivolve_from_hand(0, hi, base.index as usize, PlaySource::ByHand));

    let v = r
        .pending_selection_view()
        .expect("[ADVENTURE] Digimon digivolving must trigger the Alliance grant");
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(!v.is_optional);
    assert!(v.valid_action_ids.contains(&field_pick(z)));
    assert!(v.valid_action_ids.contains(&field_pick(base)));
}

#[test]
fn st21_04_non_adventure_digivolution_does_not_trigger() {
    // A plain Lv.4 (no ADVENTURE trait) digivolving from BASE3 must not trigger.
    let mut plain4 = dig("PLAIN4", CardColor::Red, 4, 4000, false);
    plain4.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 0,
    }];
    // The card must exist before the runner starts, so build a dedicated runner.
    let mut r = base_builder()
        .add_card(plain4)
        .deck(0, &["FILLER"; 8])
        .deck(1, &["FILLER"; 8])
        .security(1, &["SEC", "SEC"])
        .memory(10)
        .start();
    r.set_first_player(0);
    r.place_on_field(0, CARD_ID, Some(0));
    let base = r.place_on_field(0, "BASE3", Some(0));
    let hi = r.add_to_hand(0, "PLAIN4");
    assert!(r
        .game
        .digivolve_from_hand(0, hi, base.index as usize, PlaySource::ByHand));
    assert!(r.pending_selection().is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited <Alliance>
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_04_inherited_alliance_grants_to_the_carrier_via_the_stack() {
    let mut r = runner();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert!(
        r.game.has_keyword(carrier, Keyword::Alliance),
        "a Digimon with Zudomon in its digivolution cards inherits <Alliance>"
    );
}

#[test]
fn st21_04_inherited_alliance_is_absent_without_zudomon_in_the_stack() {
    let mut r = runner();
    let carrier = r.place_on_field(0, "CARRIER", Some(0));
    assert!(!r.game.has_keyword(carrier, Keyword::Alliance));
}
