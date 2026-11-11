//! ST21-06 MagnaAngemon — Lv.5, Yellow, Archangel / ADVENTURE, Ultimate,
//! Vaccine, DP 7000, Cost 7.
//!
//! # Card text (official Bandai DB, data/card_bundles/ST21-06.md)
//!
//! ```text
//! [On Play] [When Digivolving] Place 1 of your opponent's 6000 DP or lower
//! Digimon as the top security card. For every 2 colors your Tamers have, add
//! 2000 to this effect's DP maximum.
//! [Your Turn] [Once Per Turn] When your other Digimon are played or digivolve,
//! if any of them have the [ADVENTURE] trait, 1 of your Digimon gains
//! <Alliance> for the turn. Then, 1 of your Digimon may attack.
//!
//! Inherited: <Alliance>
//!
//! [Digivolve box] Lv.4 w/[ADVENTURE] trait: Cost 3
//! [Standard circle] Yellow Lv.4: Cost 3
//! Official Q&A: the <Alliance> grant cannot be declined ("You must give to
//! 1 of your Digimon").
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/Yellow/ST21_06.cs`
//!
//! # Patterns this test file covers
//!
//! | Clause                                          | Pattern                                    |
//! |-------------------------------------------------|--------------------------------------------|
//! | alt-path (Lv.4 ADVENTURE, cost 3) + yellow Lv.4 | digivolve alt_paths (structural + behavioral) |
//! | [OP][WD] place opp Digimon as top security      | selection DP cap (formula: Tamer colors / 2) + place_on_security |
//! | [Your Turn][OPT] Alliance grant then may attack | on_ally_played + on_digivolve observer, OPT lockout, mandatory grant, optional attack |
//! | Inherited <Alliance>                            | inherited grant_keyword via digivolution stack |
//!
//! # Assertions the author was least able to confirm without running them
//!
//! - After the attacker pick, the attack-target prompt is driven with the
//!   first non-PASS legal action; the opponent has no Digimon so the only
//!   legal target is their security stack.
//! - `Game::end_turn()` twice returns to player 0 with the once-per-turn
//!   lockout cleared (BT16-017 idiom).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledCost, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-06";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn digimon(id: &str, color: CardColor, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A Tamer printing the given colors.
fn tamer(id: &str, colors: &[CardColor], traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = colors.to_vec();
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// A Lv.4 that digivolves from a red Lv.3 for 2 (used as the digivolving
/// Digimon that triggers the [Your Turn] observer).
fn lv4_from_red_lv3(id: &str, traits: &[&str]) -> CardData {
    let mut c = digimon(id, CardColor::Red, 4, 5000, traits);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 2,
    }];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-06 in embedded DSL pack")
        // Allies that can be played / digivolved.
        .add_card(digimon("ADV-A", CardColor::Red, 3, 3000, &["ADVENTURE"]))
        .add_card(digimon("ADV-B", CardColor::Red, 3, 3000, &["ADVENTURE"]))
        .add_card(digimon("ADV-C", CardColor::Red, 3, 3000, &["ADVENTURE"]))
        .add_card(digimon("PLAIN-A", CardColor::Red, 3, 3000, &[]))
        .add_card(lv4_from_red_lv3("ADV-LV4", &["ADVENTURE"]))
        .add_card(lv4_from_red_lv3("PLAIN-LV4", &[]))
        .add_card(digimon("RED-LV3-BASE", CardColor::Red, 3, 3000, &[]))
        .add_card(tamer("ADV-TAMER", &[CardColor::Red], &["ADVENTURE"]))
        // Digivolution bases for MagnaAngemon itself (Lv.4).
        .add_card(digimon("YELLOW-LV4", CardColor::Yellow, 4, 5000, &[]))
        .add_card(digimon("BLUE-LV4-ADV", CardColor::Blue, 4, 5000, &["ADVENTURE"]))
        .add_card(digimon("BLUE-LV4-PLAIN", CardColor::Blue, 4, 5000, &[]))
        .add_card(digimon("YELLOW-LV3-ADV", CardColor::Yellow, 3, 3000, &["ADVENTURE"]))
        // Opponent Digimon at assorted DP.
        .add_card(digimon("OPP-6000-A", CardColor::Blue, 4, 6000, &[]))
        .add_card(digimon("OPP-6000-B", CardColor::Blue, 4, 6000, &[]))
        .add_card(digimon("OPP-8000", CardColor::Blue, 4, 8000, &[]))
        .add_card(digimon("OPP-10000", CardColor::Blue, 4, 10000, &[]))
        .add_card(digimon("OPP-12000", CardColor::Blue, 4, 12000, &[]))
        // Tamers for color counting.
        .add_card(tamer("TAMER-RED", &[CardColor::Red], &[]))
        .add_card(tamer("TAMER-RED-2", &[CardColor::Red], &[]))
        .add_card(tamer("TAMER-BLUE", &[CardColor::Blue], &[]))
        .add_card(tamer("TAMER-GREEN", &[CardColor::Green], &[]))
        .add_card(tamer("TAMER-WHITE", &[CardColor::White], &[]))
        .add_card(tamer("TAMER-DUAL", &[CardColor::Red, CardColor::Blue], &[]))
        .add_card(digimon("SEC", CardColor::Red, 3, 3000, &[]))
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
        .memory(10)
}

fn hand_index(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in player 0's hand"))
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

/// Fire MagnaAngemon's own [On Play] bundle for a permanent already on field.
fn fire_on_play(runner: &mut DebugRunner, h: PermanentHandle) {
    runner.fire_on_play(h.player, h.index as usize);
}

fn fire_when_digivolving(runner: &mut DebugRunner, h: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(h));
    runner.game.drain_effect_queue();
}

/// Play `id` from player 0's hand and return the new permanent's handle.
fn play_from_hand(runner: &mut DebugRunner, id: &str) -> PermanentHandle {
    let idx = hand_index(runner, id);
    let field = runner.play(0, idx).expect("card plays from hand");
    runner.perm_handle(0, field)
}

/// Candidate action ids of the pending selection, PASS excluded.
fn candidates(runner: &DebugRunner) -> Vec<u16> {
    let view = runner.pending_selection_view().expect("pending selection");
    view.valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .collect()
}

fn pick(runner: &mut DebugRunner, action: u16) {
    let view = runner.pending_selection_view().expect("pending selection");
    assert!(
        view.valid_action_ids.contains(&action),
        "action {action} must be legal, got {view:?}"
    );
    runner
        .execute_action(view.selecting_player, action)
        .expect("selection action resolves");
}

/// Answer the [Your Turn] pair of prompts: give <Alliance> to `target`, then
/// decline the optional attack.
fn grant_alliance_and_decline_attack(runner: &mut DebugRunner, target: PermanentHandle) {
    pick(runner, encode_attack(0, target.index as u16));
    let view = runner
        .pending_selection_view()
        .expect("the optional attacker prompt follows the grant");
    assert!(view.is_optional, "'may attack' prompt must be optional");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the optional attack");
    assert!(runner.pending_selection().is_none());
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_06_identity_matches_printed_card() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "MagnaAngemon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(7000));
    assert_eq!(card.cost, Some(7));
    let traits: Vec<String> = card.traits.iter().map(|t| t.to_lowercase()).collect();
    assert!(traits.iter().any(|t| t == "archangel"), "{:?}", card.traits);
    assert!(traits.iter().any(|t| t == "adventure"), "{:?}", card.traits);
}

#[test]
fn st21_06_alt_paths_are_yellow_lv4_and_adventure_lv4_both_cost_3() {
    let card = compiled(CARD_ID);
    assert_eq!(
        card.alt_paths.len(),
        2,
        "printed yellow Lv.4 circle + the [Digivolve] Lv.4 w/[ADVENTURE] box"
    );
    for path in &card.alt_paths {
        assert_eq!(path.cost, Some(CompiledCost::Literal(3)));
        assert!(
            !path.ignore_requirements,
            "DCGO ignoreDigivolutionRequirement is false"
        );
    }
    let adventure_path = format!("{:?}", card.alt_paths[1].from);
    assert!(
        adventure_path.contains("ADVENTURE"),
        "second alt path gates on the ADVENTURE trait, got {adventure_path}"
    );
}

#[test]
fn st21_06_clause_shape() {
    let card = compiled(CARD_ID);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2, "[OP][WD] clause + [Your Turn] observer");

    let place = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play] clause");
    assert!(place.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!place.optional, "placing into security is mandatory");
    assert!(!place.once_per_turn);

    let observer = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnAllyPlayed))
        .expect("[Your Turn] observer");
    assert!(observer.when.contains(&CompiledTiming::OnDigivolve));
    assert!(observer.once_per_turn, "[Once Per Turn]");
    assert!(!observer.optional, "the Alliance grant cannot be declined");
    assert_eq!(observer.scope, CompiledScope::FaceUp);

    let inherited_alliance = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope: CompiledScope::Inherited,
                ..
            }) if keyword.eq_ignore_ascii_case("Alliance")
        )
    });
    assert!(inherited_alliance, "inherited <Alliance> grant");
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 2 — Digivolution requirements
// ═══════════════════════════════════════════════════════════════════════════

fn digivolve_into_magnaangemon(base_id: &str) -> (DebugRunner, PermanentHandle, bool, i16) {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let base = runner.place_on_field(0, base_id, Some(0));
    let idx = hand_index(&runner, CARD_ID);
    let before = runner.memory();
    let ok = runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand);
    let paid = before - runner.memory();
    let _ = runner.auto_resolve();
    (runner, base, ok, paid)
}

#[test]
fn st21_06_digivolves_from_yellow_lv4_for_3() {
    let (runner, base, ok, paid) = digivolve_into_magnaangemon("YELLOW-LV4");
    assert!(ok, "printed yellow Lv.4 circle");
    assert_eq!(paid, 3);
    assert_eq!(top_id(&runner, base), CARD_ID);
}

#[test]
fn st21_06_digivolves_from_any_color_lv4_adventure_for_3() {
    let (runner, base, ok, paid) = digivolve_into_magnaangemon("BLUE-LV4-ADV");
    assert!(ok, "[Digivolve] Lv.4 w/[ADVENTURE] trait accepts a blue base");
    assert_eq!(paid, 3);
    assert_eq!(top_id(&runner, base), CARD_ID);
}

#[test]
fn st21_06_cannot_digivolve_from_non_yellow_non_adventure_lv4() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let base = runner.place_on_field(0, "BLUE-LV4-PLAIN", Some(0));
    let card = runner.game.players[0].hand[hand_index(&runner, CARD_ID)].clone();
    let perm = runner.game.players[0].battle_area[base.index as usize].clone();
    assert!(
        !runner.game.can_digivolve(&card, &perm),
        "blue Lv.4 without [ADVENTURE] matches neither route"
    );
}

#[test]
fn st21_06_cannot_digivolve_from_adventure_lv3() {
    let mut runner = builder().hand(0, &[CARD_ID]).start();
    let base = runner.place_on_field(0, "YELLOW-LV3-ADV", Some(0));
    let card = runner.game.players[0].hand[hand_index(&runner, CARD_ID)].clone();
    let perm = runner.game.players[0].battle_area[base.index as usize].clone();
    assert!(
        !runner.game.can_digivolve(&card, &perm),
        "the [ADVENTURE] box needs a LEVEL 4 base"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 3 — [On Play] / [When Digivolving] place into security
// ═══════════════════════════════════════════════════════════════════════════

/// Runner with MagnaAngemon on field, the given opponent Digimon, and the
/// given own Tamers. Returns (runner, magna handle, opponent handles).
fn place_setup(
    opponents: &[&str],
    tamers: &[&str],
) -> (DebugRunner, PermanentHandle, Vec<PermanentHandle>) {
    let mut runner = builder().start();
    let magna = runner.place_on_field(0, CARD_ID, Some(0));
    let opps = opponents
        .iter()
        .map(|id| runner.place_on_field(1, id, Some(0)))
        .collect();
    for id in tamers {
        runner.place_on_field(0, id, None);
    }
    (runner, magna, opps)
}

fn legal_opponent_slots(runner: &DebugRunner) -> Vec<u16> {
    let mut v = candidates(runner);
    v.sort_unstable();
    v
}

#[test]
fn st21_06_on_play_cap_is_6000_without_tamers() {
    let (mut runner, magna, opps) = place_setup(
        &["OPP-6000-A", "OPP-6000-B", "OPP-8000", "OPP-12000"],
        &[],
    );
    fire_on_play(&mut runner, magna);

    let view = runner.pending_selection_view().expect("security-placement pick");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!view.is_optional, "placing into security is mandatory");
    let mut want = vec![
        encode_attack(0, opps[0].index as u16),
        encode_attack(0, opps[1].index as u16),
    ];
    want.sort_unstable();
    assert_eq!(
        legal_opponent_slots(&runner),
        want,
        "only the two 6000-DP Digimon (6000 is inclusive) are legal"
    );
}

#[test]
fn st21_06_on_play_places_chosen_digimon_as_opponents_top_security() {
    let (mut runner, magna, opps) = place_setup(&["OPP-6000-A", "OPP-6000-B"], &[]);
    let opp_security_before = runner.security_count(1);
    let own_security_before = runner.security_count(0);

    fire_on_play(&mut runner, magna);
    pick(&mut runner, encode_attack(0, opps[1].index as u16));
    let _ = runner.auto_resolve();

    assert_eq!(runner.battle_area_size(1), 1, "chosen Digimon left the field");
    assert_eq!(runner.security_count(1), opp_security_before + 1);
    assert_eq!(
        runner.game.players[1]
            .security
            .last()
            .expect("opponent security has a top card")
            .card_id(&runner.game.card_data),
        "OPP-6000-B",
        "the card is the TOP security card of its owner"
    );
    assert_eq!(
        runner.security_count(0),
        own_security_before,
        "my own security is untouched"
    );
}

#[test]
fn st21_06_on_play_two_tamer_colors_raise_cap_to_8000() {
    let (mut runner, magna, opps) = place_setup(
        &["OPP-6000-A", "OPP-8000", "OPP-10000"],
        &["TAMER-RED", "TAMER-BLUE"],
    );
    fire_on_play(&mut runner, magna);
    let mut want = vec![
        encode_attack(0, opps[0].index as u16),
        encode_attack(0, opps[1].index as u16),
    ];
    want.sort_unstable();
    assert_eq!(
        legal_opponent_slots(&runner),
        want,
        "2 Tamer colors -> +2000: 6000 and 8000 legal, 10000 not"
    );
}

#[test]
fn st21_06_on_play_one_tamer_color_adds_nothing() {
    let (mut runner, magna, opps) = place_setup(
        &["OPP-6000-A", "OPP-6000-B", "OPP-8000"],
        &["TAMER-RED"],
    );
    fire_on_play(&mut runner, magna);
    assert!(
        !candidates(&runner).contains(&encode_attack(0, opps[2].index as u16)),
        "'for every 2 colors' needs a full pair: 1 color adds nothing"
    );
    assert_eq!(candidates(&runner).len(), 2);
}

#[test]
fn st21_06_on_play_same_color_tamers_do_not_stack() {
    let (mut runner, magna, opps) = place_setup(
        &["OPP-6000-A", "OPP-6000-B", "OPP-8000"],
        &["TAMER-RED", "TAMER-RED-2"],
    );
    fire_on_play(&mut runner, magna);
    assert!(
        !candidates(&runner).contains(&encode_attack(0, opps[2].index as u16)),
        "two red Tamers are ONE color"
    );
}

#[test]
fn st21_06_on_play_multicolor_tamer_counts_each_color() {
    let (mut runner, magna, opps) = place_setup(
        &["OPP-6000-A", "OPP-8000", "OPP-10000"],
        &["TAMER-DUAL"],
    );
    fire_on_play(&mut runner, magna);
    assert!(
        candidates(&runner).contains(&encode_attack(0, opps[1].index as u16)),
        "one red/blue Tamer has 2 colors -> 8000 legal"
    );
    assert!(!candidates(&runner).contains(&encode_attack(0, opps[2].index as u16)));
}

#[test]
fn st21_06_on_play_three_colors_still_one_pair() {
    let (mut runner, magna, opps) = place_setup(
        &["OPP-6000-A", "OPP-8000", "OPP-10000"],
        &["TAMER-RED", "TAMER-BLUE", "TAMER-GREEN"],
    );
    fire_on_play(&mut runner, magna);
    assert!(candidates(&runner).contains(&encode_attack(0, opps[1].index as u16)));
    assert!(
        !candidates(&runner).contains(&encode_attack(0, opps[2].index as u16)),
        "3 colors = 1 pair (floor), so the cap is 8000, not 9000/10000"
    );
}

#[test]
fn st21_06_on_play_four_colors_raise_cap_to_10000() {
    let (mut runner, magna, opps) = place_setup(
        &["OPP-6000-A", "OPP-10000", "OPP-12000"],
        &["TAMER-RED", "TAMER-BLUE", "TAMER-GREEN", "TAMER-WHITE"],
    );
    fire_on_play(&mut runner, magna);
    assert!(candidates(&runner).contains(&encode_attack(0, opps[1].index as u16)));
    assert!(!candidates(&runner).contains(&encode_attack(0, opps[2].index as u16)));
}

#[test]
fn st21_06_on_play_with_no_legal_target_does_nothing() {
    let (mut runner, magna, _opps) = place_setup(&["OPP-8000", "OPP-12000"], &[]);
    let security_before = runner.security_count(1);
    fire_on_play(&mut runner, magna);
    assert!(runner.pending_selection().is_none(), "nothing is 6000 or less");
    assert_eq!(runner.battle_area_size(1), 2);
    assert_eq!(runner.security_count(1), security_before);
}

#[test]
fn st21_06_when_digivolving_places_into_security_too() {
    let (mut runner, magna, opps) = place_setup(&["OPP-6000-A", "OPP-6000-B"], &[]);
    let security_before = runner.security_count(1);
    fire_when_digivolving(&mut runner, magna);
    let view = runner.pending_selection_view().expect("[When Digivolving] pick");
    assert_eq!(view.kind, SelectionKind::OppField);
    pick(&mut runner, encode_attack(0, opps[0].index as u16));
    let _ = runner.auto_resolve();
    assert_eq!(runner.security_count(1), security_before + 1);
    assert_eq!(runner.battle_area_size(1), 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 4 — [Your Turn][OPT] Alliance grant, then may attack
// ═══════════════════════════════════════════════════════════════════════════

fn alliance_runner(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder().hand(0, hand).security(1, &["SEC"]).start();
    runner.game.turn_count = 1;
    let magna = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, magna)
}

#[test]
fn st21_06_adventure_digimon_played_prompts_mandatory_alliance_grant() {
    let (mut runner, magna) = alliance_runner(&["ADV-A"]);
    let ally = play_from_hand(&mut runner, "ADV-A");

    let view = runner
        .pending_selection_view()
        .expect("Alliance grant prompt after an [ADVENTURE] Digimon is played");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        !view.is_optional,
        "Official Q&A: 'You must give to 1 of your Digimon' -- no PASS"
    );
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, magna.index as u16)));
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(0, ally.index as u16)));
}

#[test]
fn st21_06_alliance_is_granted_to_the_chosen_digimon_for_the_turn() {
    let (mut runner, magna) = alliance_runner(&["ADV-A"]);
    let ally = play_from_hand(&mut runner, "ADV-A");

    grant_alliance_and_decline_attack(&mut runner, ally);

    assert!(
        runner.game.has_keyword(ally, Keyword::Alliance),
        "the chosen Digimon gains <Alliance>"
    );

    // "for the turn": the grant is gone once the turn ends.
    runner.end_turn();
    let _ = runner.auto_resolve_declining_end_of_turn_attacks();
    assert!(
        !runner.game.has_keyword(ally, Keyword::Alliance),
        "<Alliance> lasts only for the turn"
    );
}

#[test]
fn st21_06_alliance_can_go_to_magnaangemon_itself() {
    let (mut runner, magna) = alliance_runner(&["ADV-A"]);
    play_from_hand(&mut runner, "ADV-A");
    grant_alliance_and_decline_attack(&mut runner, magna);
    assert!(runner.game.has_keyword(magna, Keyword::Alliance));
}

#[test]
fn st21_06_attack_prompt_offers_only_digimon_that_can_attack() {
    let (mut runner, magna) = alliance_runner(&["ADV-A"]);
    let ally = play_from_hand(&mut runner, "ADV-A");

    pick(&mut runner, encode_attack(0, magna.index as u16));
    let view = runner.pending_selection_view().expect("optional attacker pick");
    assert!(view.is_optional, "'may attack'");
    assert!(
        view.valid_action_ids
            .contains(&encode_attack(0, magna.index as u16)),
        "MagnaAngemon (in play since a previous turn) can attack"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(0, ally.index as u16)),
        "the just-played ally has no <Rush>, so it cannot attack and is not offered"
    );
}

#[test]
fn st21_06_declining_the_attack_leaves_everything_unsuspended() {
    let (mut runner, magna) = alliance_runner(&["ADV-A"]);
    play_from_hand(&mut runner, "ADV-A");
    let security_before = runner.security_count(1);
    grant_alliance_and_decline_attack(&mut runner, magna);
    assert!(
        !runner.game.players[0].battle_area[magna.index as usize].is_suspended,
        "no attack was declared"
    );
    assert_eq!(runner.security_count(1), security_before);
}

#[test]
fn st21_06_chosen_attacker_attacks_the_opponents_security() {
    let (mut runner, magna) = alliance_runner(&["ADV-A"]);
    play_from_hand(&mut runner, "ADV-A");
    let security_before = runner.security_count(1);

    pick(&mut runner, encode_attack(0, magna.index as u16)); // Alliance recipient
    pick(&mut runner, encode_attack(0, magna.index as u16)); // attacker

    // `may_attack_now` now asks for the attack (confirm and/or target). The
    // opponent has no Digimon, so the only legal target is their security
    // stack; taking the first legal action at each prompt completes the attack.
    assert!(
        runner.pending_selection().is_some(),
        "the attack declaration must be surfaced to the player"
    );
    runner.auto_resolve().expect("attack resolves");

    assert!(
        runner.game.players[0].battle_area[magna.index as usize].is_suspended,
        "attacking suspends the attacker"
    );
    assert!(
        runner.security_count(1) < security_before,
        "the attack checked the opponent's security"
    );
}

#[test]
fn st21_06_non_adventure_digimon_played_does_not_trigger() {
    let (mut runner, _magna) = alliance_runner(&["PLAIN-A"]);
    play_from_hand(&mut runner, "PLAIN-A");
    assert!(runner.pending_selection().is_none());
}

#[test]
fn st21_06_adventure_tamer_played_does_not_trigger() {
    let (mut runner, _magna) = alliance_runner(&[]);
    let tamer_handle = runner.place_on_field(0, "ADV-TAMER", None);
    runner.fire_play_event_triggers(0, tamer_handle.index as usize, false, false);
    assert!(
        runner.pending_selection().is_none(),
        "the observer is about your other DIGIMON"
    );
}

#[test]
fn st21_06_magnaangemon_itself_being_played_does_not_trigger_its_observer() {
    let (mut runner, magna) = alliance_runner(&[]);
    // Its own [On Play] has no legal target (opponent has no Digimon), so any
    // prompt here could only come from the "other Digimon" observer.
    runner.fire_play_event_triggers(0, magna.index as usize, false, false);
    assert!(runner.pending_selection().is_none());
}

#[test]
fn st21_06_does_not_trigger_on_the_opponents_turn() {
    let (mut runner, _magna) = alliance_runner(&[]);
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1);
    let ally = runner.place_on_field(0, "ADV-A", None);
    runner.fire_play_event_triggers(0, ally.index as usize, true, false);
    assert!(
        runner.pending_selection().is_none(),
        "[Your Turn] only: no grant on the opponent's turn"
    );
}

#[test]
fn st21_06_adventure_digivolution_triggers_the_observer() {
    let (mut runner, _magna) = alliance_runner(&["ADV-LV4"]);
    let base = runner.place_on_field(0, "RED-LV3-BASE", Some(0));
    let idx = hand_index(&runner, "ADV-LV4");
    assert!(runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    let view = runner
        .pending_selection_view()
        .expect("digivolving into an [ADVENTURE] Digimon triggers the observer");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(!view.is_optional);
}

#[test]
fn st21_06_non_adventure_digivolution_does_not_trigger() {
    let (mut runner, _magna) = alliance_runner(&["PLAIN-LV4"]);
    let base = runner.place_on_field(0, "RED-LV3-BASE", Some(0));
    let idx = hand_index(&runner, "PLAIN-LV4");
    assert!(runner
        .game
        .digivolve_from_hand(0, idx, base.index as usize, PlaySource::ByHand));
    assert!(runner.pending_selection().is_none());
}

#[test]
fn st21_06_observer_is_once_per_turn_and_resets_next_turn() {
    let (mut runner, magna) = alliance_runner(&["ADV-A", "ADV-B", "ADV-C"]);

    play_from_hand(&mut runner, "ADV-A");
    grant_alliance_and_decline_attack(&mut runner, magna);

    play_from_hand(&mut runner, "ADV-B");
    assert!(
        runner.pending_selection().is_none(),
        "[Once Per Turn]: a second [ADVENTURE] Digimon in the same turn is locked out"
    );

    runner.end_turn();
    let _ = runner.auto_resolve_declining_end_of_turn_attacks();
    runner.end_turn();
    let _ = runner.auto_resolve_declining_end_of_turn_attacks();
    assert_eq!(runner.turn_player(), 0);
    runner.game.set_memory(10);

    play_from_hand(&mut runner, "ADV-C");
    assert!(
        runner.pending_selection().is_some(),
        "the once-per-turn lockout clears on my next turn"
    );
}

#[test]
fn st21_06_non_adventure_play_does_not_spend_the_once_per_turn_use() {
    let (mut runner, magna) = alliance_runner(&["PLAIN-A", "ADV-A"]);
    play_from_hand(&mut runner, "PLAIN-A");
    assert!(runner.pending_selection().is_none());

    play_from_hand(&mut runner, "ADV-A");
    assert!(
        runner.pending_selection().is_some(),
        "the earlier non-[ADVENTURE] play must not have used up the OPT"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Section 5 — Inherited <Alliance>
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn st21_06_inherited_alliance_grants_to_carrier_via_stack() {
    let mut runner = builder().start();
    let carrier = runner.place_stack(0, &[CARD_ID, "YELLOW-LV4"]);
    assert!(
        runner.game.has_keyword(carrier, Keyword::Alliance),
        "a Digimon with MagnaAngemon as a digivolution card has <Alliance>"
    );
}

#[test]
fn st21_06_no_alliance_without_magnaangemon_in_the_stack() {
    let mut runner = builder().start();
    let plain = runner.place_on_field(0, "YELLOW-LV4", Some(0));
    assert!(!runner.game.has_keyword(plain, Keyword::Alliance));
}
