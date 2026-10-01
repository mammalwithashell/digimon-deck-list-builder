//! EX13-077 Omnimon: Merciful Mode — Digimon, Lv.7, White, DP 16000, Cost 16.
//! Traits: Holy Warrior / ADVENTURE. Form: Mega. Attribute: Vaccine.
//! Digivolve: [Omnimon]: Cost 2.
//! Assembly -8: 6 [ADVENTURE] trait Digimon cards w/different colors.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-077.md`)
//!
//! [On Play] [When Digivolving] 1 of your Digimon may attack without
//! suspending. Then, for every 2 of your Digimon and Tamers' colors, activate
//! 1 effect below:
//! ・This Digimon may battle 1 of your opponent's Digimon.
//! ・By returning 5 cards from your opponent's trash to the bottom of the
//!   deck, ＜Recovery +1＞.
//! [All Turns] This Digimon gains all colors in its digivolution cards, and
//! +1000 DP for each of your Digimon and Tamers' colors.
//!
//! # DCGO C# reference
//! None at the submodule (no EX13_077.cs). Precedents: EX12_037.cs (repeat
//! "for every N … activate 1 effect", same effect may repeat), BT17_070.cs
//! ("By returning N cards from your opponent's trash", optional exact pick),
//! AD1_020.cs ("cards with different colors").
//!
//! # Pattern rows
//! - C-Assembly `distinct_by: color` (G-ASSEMBLY-DISTINCT-BY-COLOR): bipartite
//!   card → color assignment, incl. the mask's hidden −8 availability
//! - optional own attacker + may_attack_now without suspending
//! - repeat_effect_choice over floor(colors / 2)
//! - H-aura AddColor (source colors) + DP per distinct color

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDistinctBy,
    CompiledRepeat, CompiledTiming,
};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    encode_digivolve, ATTACK_START, PASS, PLAY_HAND_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-077";

fn digimon(id: &str, colors: &[CardColor], traits: &[&str], dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.colors = colors.to_vec();
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn adv(id: &str, colors: &[CardColor]) -> CardData {
    digimon(id, colors, &["ADVENTURE"], 5000)
}

fn tamer(id: &str, color: CardColor, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    use CardColor::*;
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-077 YAML loads from the embedded pack")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(adv("ADV-R", &[Red]))
        .add_card(adv("ADV-R2", &[Red]))
        .add_card(adv("ADV-B", &[Blue]))
        .add_card(adv("ADV-Y", &[Yellow]))
        .add_card(adv("ADV-G", &[Green]))
        .add_card(adv("ADV-BK", &[Black]))
        .add_card(adv("ADV-P", &[Purple]))
        .add_card(adv("ADV-RB", &[Red, Blue]))
        .add_card(digimon("PLAIN-P", &[Purple], &[], 5000))
        .add_card(tamer("ADV-TAMER-P", Purple, &["ADVENTURE"]))
        .add_card(tamer("TAMER-Y", Yellow, &[]))
        .add_card({
            let mut c = digimon("OMNIMON", &[White], &["Holy Warrior"], 15000);
            c.card_name = "Omnimon".to_string();
            c.level = Some(7);
            c
        })
        .add_card({
            let mut c = digimon("OMNI-X", &[White], &[], 15000);
            c.card_name = "Omnimon Zwart".to_string();
            c.level = Some(7);
            c
        })
        .add_card(digimon("ALLY-R", &[Red], &[], 6000))
        .add_card(digimon("ALLY-B", &[Blue], &[], 6000))
        .add_card(digimon("OPP", &[Purple], &[], 3000))
        .add_card(digimon("SEC", &[Purple], &[], 1000))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

fn trash_index(runner: &DebugRunner, player: usize, card_id: &str) -> u16 {
    runner.game.players[player]
        .trash
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} in trash")) as u16
}

fn non_pass(runner: &DebugRunner) -> Vec<u16> {
    runner
        .pending_selection_view()
        .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
        .unwrap_or_default()
}

fn has_trash_pick(runner: &DebugRunner) -> bool {
    non_pass(runner)
        .iter()
        .any(|&a| (TRASH_EFFECT_START..TRASH_EFFECT_START + 45).contains(&a))
}

fn pick_field(runner: &mut DebugRunner, h: PermanentHandle) {
    let view = runner.pending_selection_view().expect("field prompt");
    let action = ATTACK_START + h.index as u16;
    assert!(view.valid_action_ids.contains(&action), "{h:?} selectable: {view:?}");
    runner.execute_action(view.selecting_player, action).expect("pick field");
}

fn pass(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("prompt to pass");
    assert!(view.is_optional, "PASS legal: {view:?}");
    runner.execute_action(view.selecting_player, PASS).expect("pass");
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, perm: PermanentHandle) {
    runner.game.enqueue_triggered(timing, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
}

fn colors_of(runner: &DebugRunner, h: PermanentHandle) -> Vec<CardColor> {
    let mut v = runner.game.players[h.player as usize].battle_area[h.index as usize]
        .colors_for_rules(&runner.game.card_data, &runner.game.modifiers, h);
    v.sort_by_key(|c| *c as u8);
    v
}

fn inject_all(runner: &mut DebugRunner, player: u8, ids: &[&str]) {
    for id in ids {
        runner.inject_trash(player, id);
    }
}

const SIX: [&str; 6] = ["ADV-R", "ADV-B", "ADV-Y", "ADV-G", "ADV-BK", "ADV-P"];

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn ex13_077_metadata_alt_paths_and_color_distinct_assembly() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(7), Some(16000), Some(16)));
    assert_eq!(c.color, vec![CompiledColor::White]);
    assert_eq!(c.traits, vec!["Holy Warrior".to_string(), "ADVENTURE".to_string()]);
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 1);
    assert_eq!(digi[0].cost, Some(CompiledCost::Literal(2)));
    let asm = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly -8");
    assert_eq!(asm.cost, Some(CompiledCost::Literal(8)));
    assert_eq!(asm.materials.len(), 1);
    let m = &asm.materials[0];
    assert_eq!(m.distinct_by, Some(CompiledDistinctBy::Color));
    assert_eq!(m.repeat, Some(CompiledRepeat::Range { min: 6, max: 6 }));
    assert!(m.stack_under);
}

#[test]
fn ex13_077_clause_shapes() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    let trig: Vec<_> = c
        .effects
        .iter()
        .filter_map(|cl| match cl {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(trig.len(), 1);
    assert!(trig[0].when.contains(&CompiledTiming::OnPlay));
    assert!(trig[0].when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!trig[0].optional, "the clause runs; its parts are individually optional");
}

// ─── Section 2 — Assembly w/different colors ────────────────────────────────

#[test]
fn ex13_077_assembly_six_colors_for_8() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(8).start();
    runner.skip_mulligan();
    inject_all(&mut runner, 0, &SIX);
    let mem0 = runner.game.memory;
    runner.game.decode_action(PLAY_HAND_START, 0);
    for id in SIX {
        assert!(has_trash_pick(&runner), "Assembly pick {id}");
        let idx = trash_index(&runner, 0, id);
        runner.game.decode_action(TRASH_EFFECT_START + idx, 0);
    }
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 7);
    assert_eq!(mem0 - runner.game.memory, 8, "16 − 8");
    assert_eq!(runner.trash_size(0), 0);
}

#[test]
fn ex13_077_assembly_multicolor_card_stands_for_its_free_color_in_the_mask() {
    // Red + Red/Blue + Y/G/Bk/P: Red/Blue must represent Blue → 6 colors.
    // Memory 0 can afford 16 − 8 = 8 (floor −10), never the full 16.
    let mut runner = builder().hand(0, &[CARD_ID]).memory(0).start();
    runner.skip_mulligan();
    inject_all(&mut runner, 0, &["ADV-R", "ADV-RB", "ADV-Y", "ADV-G", "ADV-BK", "ADV-P"]);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 1.0, "hidden Assembly −8 availability");
}

#[test]
fn ex13_077_assembly_rejects_a_set_whose_colors_cannot_all_differ() {
    // Red, Red/Blue, Blue cover only {Red, Blue} — three cards, two colors —
    // so with Y/G/P the trash holds at most 5 cards w/different colors, even
    // though every PAIR of the three "differs somewhere".
    let mut runner = builder().hand(0, &[CARD_ID]).memory(0).start();
    runner.skip_mulligan();
    inject_all(&mut runner, 0, &["ADV-R", "ADV-RB", "ADV-B", "ADV-Y", "ADV-G", "ADV-P"]);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 0.0, "no −8; full 16 overdraws");
}

#[test]
fn ex13_077_assembly_rejects_duplicate_colors() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(0).start();
    runner.skip_mulligan();
    inject_all(&mut runner, 0, &["ADV-R", "ADV-R2", "ADV-B", "ADV-Y", "ADV-G", "ADV-BK"]);
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 0.0);
}

#[test]
fn ex13_077_assembly_needs_adventure_trait_digimon_cards() {
    // Purple comes only from a non-[ADVENTURE] Digimon and an [ADVENTURE]
    // Tamer — neither is an "[ADVENTURE] trait Digimon card".
    let mut runner = builder().hand(0, &[CARD_ID]).memory(0).start();
    runner.skip_mulligan();
    inject_all(
        &mut runner,
        0,
        &["ADV-R", "ADV-B", "ADV-Y", "ADV-G", "ADV-BK", "PLAIN-P", "ADV-TAMER-P"],
    );
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[PLAY_HAND_START as usize], 0.0);
}

#[test]
fn ex13_077_assembly_pick_masks_cards_without_a_free_color() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(8).start();
    runner.skip_mulligan();
    inject_all(
        &mut runner,
        0,
        &["ADV-R", "ADV-R2", "ADV-RB", "ADV-B", "ADV-Y", "ADV-G", "ADV-BK", "ADV-P"],
    );
    runner.game.decode_action(PLAY_HAND_START, 0);
    let r = TRASH_EFFECT_START + trash_index(&runner, 0, "ADV-R");
    let r2 = TRASH_EFFECT_START + trash_index(&runner, 0, "ADV-R2");
    let rb = TRASH_EFFECT_START + trash_index(&runner, 0, "ADV-RB");
    let b = TRASH_EFFECT_START + trash_index(&runner, 0, "ADV-B");
    assert!(non_pass(&runner).contains(&r2));
    runner.game.decode_action(r, 0);
    let offered = non_pass(&runner);
    assert!(!offered.contains(&r2), "a second mono-Red card has no free color");
    assert!(offered.contains(&rb), "Red/Blue can still stand for Blue");
    assert!(
        !runner.pending_selection_view().unwrap().valid_action_ids.contains(&PASS),
        "Assembly −8 needs all 6 cards once started"
    );
    runner.game.decode_action(rb, 0);
    assert!(
        !non_pass(&runner).contains(&b),
        "Red/Blue now holds Blue → mono-Blue has no free color"
    );
    for id in ["ADV-Y", "ADV-G", "ADV-BK", "ADV-P"] {
        let idx = trash_index(&runner, 0, id);
        runner.game.decode_action(TRASH_EFFECT_START + idx, 0);
    }
    let _ = runner.auto_resolve();
    let omni = &runner.game.players[0].battle_area[0];
    assert_eq!(omni.card_sources.len(), 7);
    assert_eq!(runner.game.memory, 0, "8 − (16 − 8)");
}

#[test]
fn ex13_077_assembly_declined_at_the_gate_plays_at_full_cost() {
    // Memory caps at 10; the full 16 lands at −6 (the −8 would land at +2).
    let mut runner = builder().hand(0, &[CARD_ID]).memory(10).start();
    runner.skip_mulligan();
    inject_all(&mut runner, 0, &SIX);
    runner.game.decode_action(PLAY_HAND_START, 0);
    assert!(has_trash_pick(&runner));
    pass(&mut runner);
    // Full cost 16 → −6. The fresh Omnimon can't attack (summoning sickness)
    // and is alone (1 color), so its [On Play] opens no prompt
    // (G-DSL-CAN-ATTACK-PREDICATE) and the turn passes at −6: the memory
    // gauge now reads +6 from the opponent's side.
    assert_eq!(runner.game.turn_player(), 1, "−6 memory passed the turn");
    assert_eq!(runner.game.memory, 6, "full cost 16 paid (−6 for P0)");
    assert_eq!(runner.game.players[0].battle_area[0].card_sources.len(), 1);
    assert_eq!(runner.trash_size(0), 6);
}

// ─── Section 3 — Digivolve ───────────────────────────────────────────────────

#[test]
fn ex13_077_digivolves_from_omnimon_for_2() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(2).start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "OMNIMON", Some(0));
    runner.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    let _ = runner.auto_resolve();
    let top = runner.game.players[0].battle_area[0].top_card();
    assert_eq!(top.card_id(&runner.game.card_data), CARD_ID);
    assert_eq!(runner.memory(), 0);
}

#[test]
fn ex13_077_cannot_digivolve_from_a_differently_named_lv7() {
    let mut runner = builder().hand(0, &[CARD_ID]).memory(2).start();
    runner.skip_mulligan();
    runner.place_on_field(0, "OMNI-X", Some(0));
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(mask[encode_digivolve(0, 0) as usize], 0.0, "[Omnimon] is a name requirement");
}

// ─── Section 4 — [All Turns] ─────────────────────────────────────────────────

#[test]
fn ex13_077_gains_source_colors_and_1000_dp_per_own_color() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    let omni = runner.place_stack(0, &["ADV-R", "ADV-B", CARD_ID]);
    runner.place_on_field(0, "TAMER-Y", Some(0));
    runner.place_on_field(1, "ADV-G", Some(0)); // opponent's colors don't count
    runner.game.tick_declarative_effects();
    use CardColor::*;
    assert_eq!(colors_of(&runner, omni), vec![Red, Blue, White]);
    // White/Red/Blue (this Digimon, incl. gained) + Yellow (Tamer) = 4.
    assert_eq!(runner.effective_dp(omni), Some(20000));
}

#[test]
fn ex13_077_alone_with_no_sources_is_one_color() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    let omni = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.tick_declarative_effects();
    assert_eq!(colors_of(&runner, omni), vec![CardColor::White]);
    assert_eq!(runner.effective_dp(omni), Some(17000));
}

// ─── Section 5 — [On Play] / [When Digivolving] ─────────────────────────────

#[test]
fn ex13_077_on_play_ally_attacks_without_suspending_then_one_effect_per_two_colors() {
    let mut runner = builder().security(1, &["SEC"; 3]).start();
    runner.skip_mulligan();
    runner.game.turn_count = 1;
    let omni = runner.place_on_field(0, CARD_ID, None);
    let ally = runner.place_on_field(0, "ALLY-R", Some(0));
    inject_all(&mut runner, 1, &["PAD"; 5]);
    let opp_deck0 = runner.deck_size(1);
    let own_sec0 = runner.security_count(0);
    let opp_sec0 = runner.security_count(1);
    fire(&mut runner, EffectTiming::OnPlay, omni);

    let v = runner.pending_selection_view().expect("attacker prompt");
    assert_eq!(v.kind, SelectionKind::OwnField);
    assert!(v.is_optional, "'may attack'");
    pick_field(&mut runner, ally);
    // Attack target prompt → the player (security check).
    let v = runner.pending_selection_view().expect("attack target prompt");
    assert_eq!(v.kind, SelectionKind::Target, "{v:?}");
    let target = non_pass(&runner)[0];
    runner.execute_action(v.selecting_player, target).expect("attack");
    while runner.pending_kind().is_some_and(|k| k != SelectionKind::EffectChoice) {
        let v = runner.pending_selection_view().unwrap();
        runner.execute_action(v.selecting_player, v.valid_action_ids[0]).unwrap();
    }
    assert_eq!(runner.security_count(1), opp_sec0 - 1, "the attack checked security");
    assert!(!runner.game.players[0].battle_area[ally.index as usize].is_suspended, "no suspend");

    // White + Red = 2 colors → exactly 1 effect.
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));
    let opp_trash0 = runner.trash_size(1); // 5 PAD + the checked security card
    runner.execute_branch(1).expect("return-5 mode");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice), "'by returning' yes/no");
    runner.execute_branch(0).expect("return");
    for _ in 0..5 {
        let v = runner.pending_selection_view().expect("return pick");
        let a = non_pass(&runner)[0];
        runner.execute_action(v.selecting_player, a).unwrap();
    }
    assert!(runner.pending_selection_view().is_none(), "only one effect for 2 colors");
    assert_eq!(runner.trash_size(1), opp_trash0 - 5);
    assert_eq!(runner.deck_size(1), opp_deck0 + 5);
    assert_eq!(runner.security_count(0), own_sec0 + 1, "<Recovery +1>");
}

#[test]
fn ex13_077_declined_attack_and_one_color_activates_nothing() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    let omni = runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(1, "OPP", Some(0));
    fire(&mut runner, EffectTiming::OnPlay, omni);
    // The just-played Omnimon is the only Digimon and can't attack
    // (summoning sickness) ⇒ no attacker prompt at all.
    assert!(
        runner.pending_selection_view().is_none(),
        "1 color → floor(1/2) = 0 effects"
    );
}

#[test]
fn ex13_077_when_digivolving_counts_gained_colors_and_repeats_the_choice() {
    // Sources Red + Blue are gained: White/Red/Blue + Yellow Tamer = 4 → 2.
    let mut runner = builder().start();
    runner.skip_mulligan();
    let omni = runner.place_stack(0, &["ADV-R", "ADV-B", CARD_ID]);
    runner.place_on_field(0, "TAMER-Y", Some(0));
    let opp_a = runner.place_on_field(1, "OPP", Some(0));
    runner.place_on_field(1, "OPP", Some(0));
    runner.game.tick_declarative_effects();
    fire(&mut runner, EffectTiming::WhenDigivolving, omni);
    pass(&mut runner); // decline the attack
    for round in 0..2 {
        assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice), "choice {round}");
        runner.execute_branch(0).expect("battle mode (repeatable)");
        let v = runner.pending_selection_view().expect("battle target");
        assert_eq!(v.kind, SelectionKind::OppField);
        assert!(v.is_optional, "'may battle'");
        pick_field(&mut runner, opp_a); // the slot re-packs after a deletion
        let _ = runner.game.drain_effect_queue();
    }
    assert!(runner.pending_selection_view().is_none());
    assert!(runner.game.players[1].battle_area.is_empty(), "both lost to 20000 DP");
    assert_eq!(runner.trash_size(1), 2);
    let omni_perm = &runner.game.players[0].battle_area[omni.index as usize];
    assert!(!omni_perm.is_suspended, "a battle is not an attack");
}

#[test]
fn ex13_077_return_mode_needs_five_opponent_trash_cards() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    let omni = runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "ALLY-R", Some(0));
    inject_all(&mut runner, 1, &["PAD"; 4]);
    let own_sec0 = runner.security_count(0);
    fire(&mut runner, EffectTiming::OnPlay, omni);
    pass(&mut runner);
    runner.execute_branch(1).expect("return mode");
    assert!(runner.pending_selection_view().is_none(), "unpayable: no prompt");
    assert_eq!(runner.trash_size(1), 4);
    assert_eq!(runner.security_count(0), own_sec0, "no Recovery without returning");
}

#[test]
fn ex13_077_return_mode_may_be_declined() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    let omni = runner.place_on_field(0, CARD_ID, None);
    runner.place_on_field(0, "ALLY-R", Some(0));
    inject_all(&mut runner, 1, &["PAD"; 6]);
    let own_sec0 = runner.security_count(0);
    fire(&mut runner, EffectTiming::OnPlay, omni);
    pass(&mut runner);
    runner.execute_branch(1).expect("return mode");
    runner.execute_branch(1).expect("don't return");
    assert!(runner.pending_selection_view().is_none());
    assert_eq!(runner.trash_size(1), 6);
    assert_eq!(runner.security_count(0), own_sec0);
}

#[test]
fn ex13_077_declining_a_battle_still_offers_the_next_effect() {
    // 4 colors → 2 effects; declining the first battle keeps the second.
    let mut runner = builder().start();
    runner.skip_mulligan();
    let omni = runner.place_stack(0, &["ADV-R", "ADV-B", CARD_ID]);
    runner.place_on_field(0, "TAMER-Y", Some(0));
    let opp = runner.place_on_field(1, "OPP", Some(0));
    runner.game.tick_declarative_effects();
    fire(&mut runner, EffectTiming::WhenDigivolving, omni);
    pass(&mut runner); // decline the attacker pick
    runner.execute_branch(0).expect("battle mode");
    pass(&mut runner); // decline the battle
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice), "second effect");
    runner.execute_branch(0).expect("battle mode again");
    pick_field(&mut runner, opp);
    let _ = runner.auto_resolve();
    assert!(runner.game.players[1].battle_area.is_empty());
}

#[test]
fn ex13_077_declining_at_the_attack_target_prompt_still_runs_the_then() {
    let mut runner = builder().security(1, &["SEC"; 3]).start();
    runner.skip_mulligan();
    runner.game.turn_count = 1;
    let omni = runner.place_on_field(0, CARD_ID, None);
    let ally = runner.place_on_field(0, "ALLY-R", Some(0));
    let opp_sec0 = runner.security_count(1);
    fire(&mut runner, EffectTiming::OnPlay, omni);
    pick_field(&mut runner, ally);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Target));
    pass(&mut runner);
    assert_eq!(runner.security_count(1), opp_sec0, "no attack");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice), "Then: 1 effect");
}

// ─── G-DSL-CAN-ATTACK-PREDICATE (2026-10-01) ─────────────────────────────────
//
// "1 of your Digimon may attack without suspending": only a Digimon that can
// attack WITHOUT suspending is offered (DCGO `CanAttack(withoutTap: true)` —
// a suspended Digimon qualifies, a summoning-sick or CannotAttack one does
// not). With no eligible Digimon there is no attacker prompt and the "Then"
// still resolves.

#[test]
fn ex13_077_attacker_pick_offers_only_digimon_that_can_attack_without_suspending() {
    let mut runner = builder().security(1, &["SEC"; 3]).start();
    runner.skip_mulligan();
    runner.game.turn_count = 1;
    let omni = runner.place_on_field(0, CARD_ID, None); // just played
    let ready = runner.place_on_field(0, "ALLY-R", Some(0));
    let suspended = runner.place_on_field(0, "ALLY-R", Some(0));
    runner.game.players[0].battle_area[suspended.index as usize].is_suspended = true;
    let fresh = runner.place_on_field(0, "ALLY-B", None);
    let locked = runner.place_on_field(0, "ALLY-B", Some(0));
    runner.game.modifiers.add(
        locked,
        digimon_engine::modifiers::ModifierEntry::simple(
            digimon_engine::enums::ModifierType::CannotAttack,
            1,
            digimon_engine::enums::Expiry::EndOfTurn,
            1,
        ),
    );
    fire(&mut runner, EffectTiming::OnPlay, omni);

    let v = runner.pending_selection_view().expect("attacker prompt");
    assert_eq!(v.kind, SelectionKind::OwnField);
    let offered = non_pass(&runner);
    let slot = |h: PermanentHandle| ATTACK_START + h.index as u16;
    assert!(offered.contains(&slot(ready)), "{offered:?}");
    assert!(
        offered.contains(&slot(suspended)),
        "a suspended Digimon can still attack WITHOUT suspending"
    );
    assert!(!offered.contains(&slot(omni)), "summoning-sick Omnimon is not offered");
    assert!(!offered.contains(&slot(fresh)), "summoning-sick ally is not offered");
    assert!(!offered.contains(&slot(locked)), "CannotAttack ally is not offered");
}

#[test]
fn ex13_077_no_attacker_prompt_when_none_can_attack_and_then_still_runs() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    runner.game.turn_count = 1;
    let omni = runner.place_on_field(0, CARD_ID, None); // summoning sick
    runner.place_on_field(0, "TAMER-Y", Some(0)); // White + Yellow = 2 colors
    fire(&mut runner, EffectTiming::OnPlay, omni);
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::EffectChoice),
        "no attacker prompt; the 'Then' (1 effect for 2 colors) is offered directly"
    );
}
