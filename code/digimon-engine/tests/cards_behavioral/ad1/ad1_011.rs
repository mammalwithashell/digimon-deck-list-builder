//! AD1-011 Paildramon — Digimon, Lv.5, Blue/Green, DP 8000, Cost 8.
//! Traits: Dragonkin / Hero. Form: Ultimate. Attribute: Free.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/AD1-011.md`)
//!
//! Digivolve: Blue Lv.4 / Cost 4, Green Lv.4 / Cost 4.
//! [Digivolve] Lv.4 w/[Free]/[Hero] trait: Cost 3
//! [DNA Digivolve] Blue Lv.4 + green Lv.4: Cost 0
//!
//! ＜Partition (blue Lv.4 & green Lv.4)＞ [When Digivolving] Until your
//! opponent's turn ends, this Digimon can't be deleted in battle. Then, if DNA
//! digivolving, this Digimon's attack target can't change for the turn.
//! [When Attacking] This Digimon may digivolve into a Digimon card with
//! [Imperialdramon] in its name in the hand with the digivolution cost reduced
//! by 2.
//!
//! Inherited Effect: ＜Partition (blue Lv.4 & green Lv.4)＞
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/AD1/Blue/AD1_011.cs
//!
//! # Patterns
//! - A1: alt digivolve by trait (no color gate); DNA alt-path.
//! - K: <Partition> face-up + inherited.
//! - M1: duration modifier on self (CannotBeDestroyedByBattle, until opp turn end).
//! - C3: DNA-origin conditional tail (CannotSwitchAttackTarget, end of turn).
//! - D2: [When Attacking] optional effect-initiated digivolve from hand, cost −2.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{
    CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType, PlaySource,
};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "AD1-011";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(
    id: &str,
    name: &str,
    color: CardColor,
    level: u8,
    dp: i32,
    traits: &[&str],
) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.colors = vec![color];
    c.dp = Some(dp);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn with_evo(mut c: CardData, from_color: CardColor, from_level: u8, cost: u16) -> CardData {
    c.evo_costs.push(EvoCost {
        card_color: from_color as u8,
        level: from_level,
        memory_cost: cost,
    });
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("AD1-011 YAML loads")
        .add_card(digimon("BLUE4", "Veemon", CardColor::Blue, 4, 5000, &[]))
        .add_card(digimon(
            "GREEN4",
            "Stingmon",
            CardColor::Green,
            4,
            5000,
            &[],
        ))
        .add_card(digimon(
            "HERO4",
            "Hero Lv4",
            CardColor::Yellow,
            4,
            5000,
            &["Hero"],
        ))
        .add_card(digimon(
            "FREE4",
            "Free Lv4",
            CardColor::Red,
            4,
            5000,
            &["Free"],
        ))
        .add_card(digimon(
            "PLAIN4",
            "Plain Lv4",
            CardColor::Yellow,
            4,
            5000,
            &["Beast"],
        ))
        .add_card(with_evo(
            digimon(
                "IMP",
                "Imperialdramon: Test Mode",
                CardColor::Blue,
                6,
                12000,
                &[],
            ),
            CardColor::Blue,
            5,
            5,
        ))
        .add_card(with_evo(
            digimon(
                "IMP-RED",
                "Imperialdramon: Red Mode",
                CardColor::Red,
                6,
                12000,
                &[],
            ),
            CardColor::Red,
            5,
            5,
        ))
        .add_card(with_evo(
            digimon("WARG", "WarGreymon", CardColor::Blue, 6, 12000, &[]),
            CardColor::Blue,
            5,
            5,
        ))
        .add_card(digimon("BIG", "Big Opp", CardColor::Red, 6, 10000, &[]))
        .add_card(digimon("BLK", "Blocker Opp", CardColor::Red, 5, 6000, &[]))
        .add_card(digimon("FILL", "Filler", CardColor::Red, 3, 1000, &[]))
}

fn start(b: DebugRunnerBuilder, hand: &[&str]) -> DebugRunner {
    let mut r = b
        .hand(0, hand)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
        .memory(10)
        .start();
    r.game.current_phase = GamePhase::Main;
    r
}

fn ids(r: &DebugRunner, cards: &[digimon_engine::card_source::CardSource]) -> Vec<String> {
    cards
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    ids(r, &r.game.players[p as usize].hand)
}

fn field_ids(r: &DebugRunner, p: u8) -> Vec<String> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn handle_of(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let i = field_ids(r, p)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} on player {p}'s field: {:?}", field_ids(r, p)));
    PermanentHandle {
        player: p,
        index: i as u8,
    }
}

fn hand_index(r: &DebugRunner, p: u8, id: &str) -> usize {
    hand_ids(r, p)
        .iter()
        .position(|c| c == id)
        .unwrap_or_else(|| panic!("{id} in hand: {:?}", hand_ids(r, p)))
}

fn offered_hand_ids(r: &DebugRunner) -> Vec<String> {
    let view = r.pending_selection_view().expect("hand prompt pending");
    let hand = hand_ids(r, view.selecting_player);
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| {
            [PLAY_HAND_START, HAND_EFFECT_START]
                .iter()
                .filter_map(|&s| a.checked_sub(s))
                .find(|&i| (i as usize) < hand.len() && i < 30)
        })
        .map(|i| hand[i as usize].clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

fn pick_hand(r: &mut DebugRunner, id: &str) {
    let view = r.pending_selection_view().expect("hand prompt pending");
    let p = view.selecting_player;
    let slot = hand_index(r, p, id) as u16;
    let a = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} not selectable: {view:?}"));
    r.execute_action(p, a).expect("pick hand card");
}

fn decline(r: &mut DebugRunner) {
    let view = r.pending_selection_view().expect("optional prompt pending");
    r.execute_action(view.selecting_player, PASS)
        .expect("decline");
}

/// Standard (player-initiated) digivolve of hand card `id` onto `target`.
fn digivolve(r: &mut DebugRunner, id: &str, target: PermanentHandle) -> bool {
    r.game.current_phase = GamePhase::Main;
    let idx = hand_index(r, 0, id);
    let ok = r
        .game
        .digivolve_from_hand(0, idx, target.index as usize, PlaySource::ByDigivolve);
    r.game.drain_effect_queue();
    ok
}

/// DNA digivolve AD1-011 (hand) from the two given field materials.
fn dna_digivolve(r: &mut DebugRunner, a: PermanentHandle, b: PermanentHandle) {
    r.game.current_phase = GamePhase::Main;
    let idx = hand_index(r, 0, CARD_ID);
    assert!(
        r.game.initiate_dna_digivolve(0, idx),
        "DNA digivolve must initiate"
    );
    r.game
        .resolve_selection(0, a.index as u16)
        .expect("first DNA material");
    r.game
        .resolve_selection(0, b.index as u16)
        .expect("second DNA material");
    r.game.drain_effect_queue();
}

fn has(r: &DebugRunner, h: PermanentHandle, m: ModifierType) -> bool {
    r.game.modifiers.has(h, m)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ad1_011_metadata_matches_print() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Paildramon");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(8000));
    assert_eq!(card.cost, Some(8));
    assert!(card.color.contains(&CompiledColor::Blue));
    assert!(card.color.contains(&CompiledColor::Green));
    assert!(card.traits.iter().any(|t| t == "Dragonkin"));
    assert!(card.traits.iter().any(|t| t == "Hero"));
}

#[test]
fn ad1_011_alt_paths_standard_trait_and_dna() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let digis: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(
        digis.len(),
        3,
        "blue Lv.4/4, green Lv.4/4, Free/Hero Lv.4/3"
    );
    assert_eq!(
        digis
            .iter()
            .filter(|p| p.cost == Some(CompiledCost::Literal(4)))
            .count(),
        2
    );
    assert_eq!(
        digis
            .iter()
            .filter(|p| p.cost == Some(CompiledCost::Literal(3)))
            .count(),
        1
    );
    let dna: Vec<_> = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .collect();
    assert_eq!(dna.len(), 1);
    assert_eq!(dna[0].materials.len(), 2);
    assert_eq!(dna[0].cost, Some(CompiledCost::Literal(0)));
}

#[test]
fn ad1_011_has_face_up_and_inherited_partition_excluding_battle_and_own_effects() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let parts: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Partition {
                scope,
                sources,
                exclude_cause,
                ..
            }) => Some((scope.clone(), sources.len(), exclude_cause.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(parts.len(), 2);
    assert!(parts.iter().any(|(s, _, _)| *s == CompiledScope::FaceUp));
    assert!(parts.iter().any(|(s, _, _)| *s == CompiledScope::Inherited));
    for (_, n, ex) in &parts {
        assert_eq!(*n, 2, "blue Lv.4 & green Lv.4");
        assert!(ex.iter().any(|s| s == "battle"));
        assert!(ex.iter().any(|s| s == "own_effect"));
    }
}

#[test]
fn ad1_011_triggered_clauses_are_wd_and_wa_without_opt() {
    let r = builder().start();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let trig: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(trig.len(), 2);
    assert_eq!(trig[0].when, vec![CompiledTiming::WhenDigivolving]);
    assert!(!trig[0].optional, "WD is mandatory");
    assert_eq!(trig[1].when, vec![CompiledTiming::WhenAttacking]);
    assert!(!trig[1].once_per_turn, "WA has no [Once Per Turn]");
    for t in &trig {
        assert_eq!(t.scope, CompiledScope::FaceUp);
    }
}

// ─── Section 2 — Digivolution requirements ───────────────────────────────────

#[test]
fn ad1_011_digivolves_from_hero_trait_lv4_for_3() {
    let mut r = start(builder(), &[CARD_ID]);
    let base = r.place_on_field(0, "HERO4", Some(0));
    let m0 = r.memory();
    assert!(
        digivolve(&mut r, CARD_ID, base),
        "Lv.4 [Hero] → cost 3 alt route"
    );
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
    assert_eq!(m0 - r.memory(), 3);
}

#[test]
fn ad1_011_digivolves_from_free_trait_lv4_for_3() {
    let mut r = start(builder(), &[CARD_ID]);
    let base = r.place_on_field(0, "FREE4", Some(0));
    let m0 = r.memory();
    assert!(
        digivolve(&mut r, CARD_ID, base),
        "Lv.4 [Free] → cost 3 alt route"
    );
    assert_eq!(m0 - r.memory(), 3);
}

#[test]
fn ad1_011_digivolves_from_blue_lv4_for_4() {
    let mut r = start(builder(), &[CARD_ID]);
    let base = r.place_on_field(0, "BLUE4", Some(0));
    let m0 = r.memory();
    assert!(digivolve(&mut r, CARD_ID, base));
    assert_eq!(m0 - r.memory(), 4, "printed blue Lv.4 circle");
}

#[test]
fn ad1_011_cannot_digivolve_from_off_color_lv4_without_trait() {
    let mut r = start(builder(), &[CARD_ID]);
    let base = r.place_on_field(0, "PLAIN4", Some(0));
    let m0 = r.memory();
    assert!(
        !digivolve(&mut r, CARD_ID, base),
        "yellow Lv.4 without Free/Hero has no route"
    );
    assert_eq!(field_ids(&r, 0), vec!["PLAIN4".to_string()]);
    assert_eq!(r.memory(), m0);
}

#[test]
fn ad1_011_dna_digivolves_from_blue_and_green_lv4_for_0() {
    let mut r = start(builder(), &[CARD_ID]);
    let a = r.place_on_field(0, "BLUE4", Some(0));
    let b = r.place_on_field(0, "GREEN4", Some(0));
    let m0 = r.memory();
    dna_digivolve(&mut r, a, b);
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
    assert_eq!(r.memory(), m0, "DNA cost 0");
}

// ─── Section 3 — [When Digivolving] ──────────────────────────────────────────

#[test]
fn ad1_011_standard_digivolve_grants_battle_immunity_but_no_target_lock() {
    let mut r = start(builder(), &[CARD_ID]);
    let base = r.place_on_field(0, "BLUE4", Some(0));
    assert!(digivolve(&mut r, CARD_ID, base));
    let h = handle_of(&r, 0, CARD_ID);
    assert!(has(&r, h, ModifierType::CannotBeDestroyedByBattle));
    assert!(
        !has(&r, h, ModifierType::CannotSwitchAttackTarget),
        "not DNA → no attack-target lock"
    );
}

#[test]
fn ad1_011_dna_digivolve_grants_battle_immunity_and_target_lock() {
    let mut r = start(builder(), &[CARD_ID]);
    let a = r.place_on_field(0, "BLUE4", Some(0));
    let b = r.place_on_field(0, "GREEN4", Some(0));
    dna_digivolve(&mut r, a, b);
    let h = handle_of(&r, 0, CARD_ID);
    assert!(has(&r, h, ModifierType::CannotBeDestroyedByBattle));
    assert!(has(&r, h, ModifierType::CannotSwitchAttackTarget));
}

#[test]
fn ad1_011_battle_immunity_lasts_until_end_of_opponents_turn() {
    let mut r = start(builder(), &[CARD_ID]);
    let base = r.place_on_field(0, "BLUE4", Some(0));
    assert!(digivolve(&mut r, CARD_ID, base));
    let h = handle_of(&r, 0, CARD_ID);
    r.pass_turn();
    assert_eq!(r.turn_player(), 1);
    assert!(
        has(&r, h, ModifierType::CannotBeDestroyedByBattle),
        "active on opp turn"
    );
    r.pass_turn();
    assert!(
        !has(&r, h, ModifierType::CannotBeDestroyedByBattle),
        "expires when the opponent's turn ends"
    );
}

#[test]
fn ad1_011_target_lock_expires_at_end_of_turn() {
    let mut r = start(builder(), &[CARD_ID]);
    let a = r.place_on_field(0, "BLUE4", Some(0));
    let b = r.place_on_field(0, "GREEN4", Some(0));
    dna_digivolve(&mut r, a, b);
    let h = handle_of(&r, 0, CARD_ID);
    r.pass_turn();
    assert!(
        !has(&r, h, ModifierType::CannotSwitchAttackTarget),
        "for the turn only"
    );
    assert!(has(&r, h, ModifierType::CannotBeDestroyedByBattle));
}

#[test]
fn ad1_011_survives_losing_battle_after_digivolving() {
    let mut r = start(builder(), &[CARD_ID]);
    let base = r.place_on_field(0, "BLUE4", Some(0));
    let big = r.place_on_field(1, "BIG", Some(0));
    r.game.players[1].battle_area[big.index as usize].is_suspended = true;
    assert!(digivolve(&mut r, CARD_ID, base));
    let h = handle_of(&r, 0, CARD_ID);
    r.attack_digimon(h, big, false);
    let _ = r.auto_resolve();
    assert!(
        field_ids(&r, 0).contains(&CARD_ID.to_string()),
        "8000 DP loses to 10000 DP but can't be deleted in battle"
    );
    assert!(field_ids(&r, 1).contains(&"BIG".to_string()));
}

#[test]
fn ad1_011_without_when_digivolving_is_deleted_by_losing_battle() {
    let mut r = start(builder(), &[]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let big = r.place_on_field(1, "BIG", Some(0));
    r.game.players[1].battle_area[big.index as usize].is_suspended = true;
    r.attack_digimon(h, big, false);
    let _ = r.auto_resolve();
    assert!(
        !field_ids(&r, 0).contains(&CARD_ID.to_string()),
        "control: no immunity without the [When Digivolving] effect"
    );
}

#[test]
fn ad1_011_dna_target_lock_suppresses_block() {
    let mut r = start(builder(), &[CARD_ID]);
    let a = r.place_on_field(0, "BLUE4", Some(0));
    let b = r.place_on_field(0, "GREEN4", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);
    dna_digivolve(&mut r, a, b);
    let h = handle_of(&r, 0, CARD_ID);
    // DNA stacks the result unsuspended? Make sure it can attack.
    r.game.players[0].battle_area[h.index as usize].is_suspended = false;
    let _ = r.attack_player(h, 1, false);
    assert_ne!(
        r.current_phase(),
        GamePhase::BlockTiming,
        "attack target can't change → no Block window"
    );
}

#[test]
fn ad1_011_standard_digivolve_still_allows_block() {
    let mut r = start(builder(), &[CARD_ID]);
    let base = r.place_on_field(0, "BLUE4", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);
    assert!(digivolve(&mut r, CARD_ID, base));
    let h = handle_of(&r, 0, CARD_ID);
    let res = r.attack_player(h, 1, false);
    assert_eq!(res, AttackResult::InProgress);
    assert_eq!(
        r.current_phase(),
        GamePhase::BlockTiming,
        "non-DNA → Block allowed"
    );
}

// ─── Section 4 — [When Attacking] digivolve into [Imperialdramon] ────────────

#[test]
fn ad1_011_when_attacking_offers_only_routable_imperialdramon() {
    let mut r = start(builder(), &["IMP", "IMP-RED", "WARG"]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(h, 1, false);
    assert!(r.pending_is_optional(), "'may digivolve'");
    assert_eq!(
        offered_hand_ids(&r),
        vec!["IMP".to_string()],
        "WarGreymon lacks the name; the red Imperialdramon has no route"
    );
}

#[test]
fn ad1_011_when_attacking_digivolves_with_cost_reduced_by_2() {
    let mut r = start(builder(), &["IMP"]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let m0 = r.memory();
    r.attack_player(h, 1, false);
    pick_hand(&mut r, "IMP");
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["IMP".to_string()]);
    assert_eq!(m0 - r.memory(), 3, "evo cost 5 reduced by 2");
}

#[test]
fn ad1_011_when_attacking_decline_keeps_paildramon() {
    let mut r = start(builder(), &["IMP"]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let m0 = r.memory();
    r.attack_player(h, 1, false);
    decline(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
    assert_eq!(r.memory(), m0);
    assert!(hand_ids(&r, 0).contains(&"IMP".to_string()));
}

#[test]
fn ad1_011_when_attacking_without_candidate_does_nothing() {
    let mut r = start(builder(), &["WARG"]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(h, 1, false);
    assert!(
        r.pending_selection().is_none(),
        "no [Imperialdramon] → no prompt"
    );
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec![CARD_ID.to_string()]);
}

#[test]
fn ad1_011_when_attacking_is_not_once_per_turn() {
    let mut r = start(builder(), &["IMP"]);
    let h = r.place_on_field(0, CARD_ID, Some(0));
    let h2 = r.place_on_field(0, CARD_ID, Some(0));
    r.attack_player(h, 1, false);
    decline(&mut r);
    let _ = r.auto_resolve();
    r.attack_player(h2, 1, false);
    assert!(
        r.pending_is_optional(),
        "a second Paildramon's [When Attacking] fires in the same turn"
    );
    pick_hand(&mut r, "IMP");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"IMP".to_string()));
}
