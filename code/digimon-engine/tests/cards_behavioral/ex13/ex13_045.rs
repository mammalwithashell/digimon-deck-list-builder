//! EX13-045 Examon — Digimon, Lv.7, Green/Red/Blue, DP 15000, Cost 15.
//! Traits: Holy Warrior / Royal Knight. Attribute: Data.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-045.md`)
//!
//! ＜Raid＞ ＜Piercing＞ ＜Security A. +1＞ ＜Blocker＞ ＜Evade＞
//! [When Digivolving] If DNA digivolving, this Digimon attacks and all of your
//! Digimon get +10000 DP until your opponent's turn ends. Then, this Digimon
//! may battle 1 of your opponent's Digimon.
//! [Your Turn] [Once Per Turn] When this Digimon wins a battle, you may play or
//! use 1 play or use cost 12 or lower [Dracomon] or [Examon] text card from
//! your hand or its digivolution cards without paying the cost.
//! Digivolve: Green/Red/Blue Lv.6 / 5; [DNA] Green Lv.6 + Blue Lv.6: Cost 0.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Green/EX13_045.cs
//!
//! # Patterns this test covers
//! - H9 Raid / H3 Piercing / H4 Security A.+1 / H5 Blocker / H8 Evade.
//! - G2 DNA digivolve route + DNA-origin conditional (mass buff + forced attack).
//! - F1 force attack; effect battle ("may battle").
//! - Win-a-battle trigger (OPT, your turn) → optional union-zone free
//!   play-or-use, Option vs Digimon routed by `binding_card_kind`.

#![allow(dead_code, unused_imports)]

use std::sync::Arc;

use digimon_dsl::compiled::{CompiledAltPathKind, CompiledClause, CompiledCost, CompiledTiming};
use digimon_engine::action::space::{ATTACK_START, PASS, PLAY_HAND_START, SOURCE_SELECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource, UnionZoneSet};
use digimon_engine::{CardEffect, Effect};

const CARD_ID: &str = "EX13-045";

struct OptionMainDraw;

impl CardEffect for OptionMainDraw {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_attacking(card)
            .option_main()
            .name("OptionMain draw 1")
            .process(|ctx: &mut EffectContext| {
                let owner = ctx.player;
                ctx.draw(owner, 1);
            })
            .build()]
    }
}

fn digimon(id: &str, level: u8, dp: i32, color: CardColor, text: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16 + 2;
    c.colors = vec![color];
    c.effect_text = text.to_string();
    c
}

fn option(id: &str, cost: u16, text: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![CardColor::Green];
    c.effect_text = text.to_string();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-045 YAML loads")
        .add_card(make_test_card("PAD", "PAD"))
        .add_card(digimon("GREEN6", 6, 12000, CardColor::Green, ""))
        .add_card(digimon("BLUE6", 6, 12000, CardColor::Blue, ""))
        .add_card(digimon("ALLY", 4, 5000, CardColor::Red, ""))
        .add_card(digimon("OPP-WEAK", 4, 5000, CardColor::Purple, ""))
        .add_card(digimon("OPP-STRONG", 6, 30000, CardColor::Purple, ""))
        .add_card(digimon("DRACO-D", 3, 3000, CardColor::Green, "[On Play] ... [Dracomon] ..."))
        .add_card({
            let mut c = digimon("DRACO-13", 7, 15000, CardColor::Green, "[Dracomon]");
            c.play_cost = 13;
            c
        })
        .add_card(option("EXA-OPT", 6, "[Main] ... [Examon] ..."))
        .add_card(option("PLAIN-OPT", 2, "[Main] Draw 1."))
        .deck(0, &["PAD"; 8])
        .deck(1, &["PAD"; 8])
}

fn fire(runner: &mut DebugRunner, timing: EffectTiming, perm: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(timing, TriggerSource::Permanent(perm));
    runner.game.drain_effect_queue();
}

fn field_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

fn non_pass(runner: &DebugRunner) -> Vec<u16> {
    runner
        .pending_selection_view()
        .map(|v| v.valid_action_ids.into_iter().filter(|&a| a != PASS).collect())
        .unwrap_or_default()
}

fn examon_handle(runner: &DebugRunner) -> PermanentHandle {
    let idx = field_ids(runner, 0)
        .iter()
        .position(|c| c == CARD_ID)
        .expect("Examon on the field");
    PermanentHandle { player: 0, index: idx as u8 }
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn ex13_045_metadata_and_alt_paths() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(7), Some(15000), Some(15)));
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 3);
    assert!(digi.iter().all(|p| p.cost == Some(CompiledCost::Literal(5))));
    let dna = c
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::DnaDigivolve)
        .expect("DNA route");
    assert_eq!(dna.cost, Some(CompiledCost::Literal(0)));
    assert_eq!(dna.materials.len(), 2);
}

#[test]
fn ex13_045_carrier_keywords() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    for k in [Keyword::Raid, Keyword::Piercing, Keyword::Blocker, Keyword::Evade] {
        assert!(runner.game.has_keyword(me, k), "{k:?}");
    }
    assert!(runner.game.has_security_attack_change(me), "<Security A. +1>");
}

#[test]
fn ex13_045_clause_shapes() {
    let runner = builder().start();
    let c = runner.compiled_card(CARD_ID).expect("compiled");
    let t: Vec<_> = c
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(t.len(), 2);
    let wd = t.iter().find(|x| x.when == vec![CompiledTiming::WhenDigivolving]).expect("WD");
    assert!(!wd.once_per_turn);
    let win = t.iter().find(|x| x.when == vec![CompiledTiming::OnAllyWonBattle]).expect("win");
    assert!(win.once_per_turn);
}

// ─── Section 2/3 — [When Digivolving] ────────────────────────────────────────

fn dna_runner(opp: &[&str]) -> DebugRunner {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .security(1, &["PAD"; 3])
        .memory(10)
        .start();
    runner.game.current_phase = GamePhase::Main;
    runner.place_on_field(0, "GREEN6", Some(0));
    runner.place_on_field(0, "BLUE6", Some(0));
    runner.place_on_field(0, "ALLY", Some(0));
    for id in opp {
        runner.place_on_field(1, id, Some(0));
    }
    assert!(runner.game.initiate_dna_digivolve(0, 0), "DNA action legal");
    runner.game.resolve_selection(0, 0).expect("first DNA material");
    runner.game.resolve_selection(0, 1).expect("second DNA material");
    runner
}

#[test]
fn ex13_045_dna_digivolve_buffs_all_own_digimon_and_forces_an_attack() {
    let mut runner = dna_runner(&[]);
    let me = examon_handle(&runner);
    let ally = PermanentHandle {
        player: 0,
        index: field_ids(&runner, 0).iter().position(|c| c == "ALLY").unwrap() as u8,
    };
    assert_eq!(runner.effective_dp(me), Some(25000), "+10000 on Examon");
    assert_eq!(runner.effective_dp(ally), Some(15000), "+10000 on every own Digimon");
    let view = runner.pending_selection_view().expect("forced attack target prompt");
    assert!(!view.is_optional, "'this Digimon attacks' is mandatory");
    let sec_before = runner.security_count(1);
    let a = non_pass(&runner)[0];
    runner.execute_action(view.selecting_player, a).expect("attack");
    let _ = runner.auto_resolve();
    assert!(runner.security_count(1) < sec_before, "the attack checked security");
}

#[test]
fn ex13_045_dna_may_battle_an_opponent_digimon() {
    let mut runner = dna_runner(&["OPP-WEAK"]);
    // Drive: decline nothing — attack the player, then choose the battle target.
    let mut battled = false;
    for _ in 0..10 {
        let Some(v) = runner.pending_selection_view() else { break };
        if v.prompt.contains("Raid") {
            runner.execute_action(v.selecting_player, PASS).expect("keep the security target");
            continue;
        }
        if v.prompt.contains("battle") {
            assert!(v.is_optional, "'may battle'");
            runner.execute_action(v.selecting_player, ATTACK_START).expect("battle OPP-WEAK");
            battled = true;
            continue;
        }
        let a = v.valid_action_ids.iter().copied().filter(|&a| a != PASS).max().unwrap_or(PASS);
        runner.execute_action(v.selecting_player, a).unwrap();
    }
    assert!(battled, "the battle prompt was offered");
    assert!(
        !field_ids(&runner, 1).contains(&"OPP-WEAK".to_string()),
        "25000 DP Examon beats a 5000 DP Digimon in battle"
    );
}

#[test]
fn ex13_045_non_dna_digivolve_only_offers_the_battle() {
    let mut runner = builder().start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-WEAK", Some(0));
    fire(&mut runner, EffectTiming::WhenDigivolving, me);
    assert_eq!(runner.effective_dp(me), Some(15000), "no buff without DNA");
    let view = runner.pending_selection_view().expect("battle prompt");
    assert!(view.is_optional);
    assert_eq!(view.kind, SelectionKind::OppField);
    runner.execute_action(0, PASS).expect("decline battle");
    let _ = runner.auto_resolve();
    assert_eq!(runner.battle_area_size(1), 1);
}

// ─── Section 2/3/5 — win a battle → play/use ─────────────────────────────────

fn win_runner(hand: &[&str], stack: &[&str]) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = builder().hand(0, hand).memory(3).start();
    runner.register_effect("EXA-OPT", Arc::new(OptionMainDraw));
    runner.register_effect("PLAIN-OPT", Arc::new(OptionMainDraw));
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    for id in stack {
        runner.push_source(me, id);
    }
    let opp = runner.place_on_field(1, "OPP-WEAK", Some(0));
    (runner, me, opp)
}

fn win_battle(runner: &mut DebugRunner, me: PermanentHandle, opp: PermanentHandle) {
    let _ = runner.battle_digimon(me, opp);
}

#[test]
fn ex13_045_winning_a_battle_plays_a_dracomon_text_digimon_from_hand_free() {
    let (mut runner, me, opp) = win_runner(&["DRACO-D", "PLAIN-OPT"], &[]);
    let mem0 = runner.memory();
    win_battle(&mut runner, me, opp);
    assert!(runner.battle_area_size(1) == 0, "battle won");
    assert!(matches!(runner.pending_kind(), Some(SelectionKind::UnionZone { .. })));
    assert!(runner.pending_is_optional());
    assert_eq!(non_pass(&runner).len(), 1, "PLAIN-OPT has no [Dracomon]/[Examon] text");
    let a = non_pass(&runner)[0];
    runner.execute_action(0, a).expect("play");
    let _ = runner.auto_resolve();
    assert!(field_ids(&runner, 0).contains(&"DRACO-D".to_string()));
    assert_eq!(runner.memory(), mem0, "free");
}

#[test]
fn ex13_045_winning_a_battle_uses_an_examon_text_option_from_sources() {
    let (mut runner, me, opp) = win_runner(&[], &["GREEN6", "EXA-OPT"]);
    win_battle(&mut runner, me, opp);
    let a = non_pass(&runner)
        .into_iter()
        .find(|&a| a >= SOURCE_SELECT_START)
        .expect("stacked Option offered");
    runner.execute_action(0, a).expect("use");
    let _ = runner.auto_resolve();
    assert_eq!(runner.hand_size(0), 1, "the Option's body drew 1");
    assert!(runner.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == "EXA-OPT"));
}

#[test]
fn ex13_045_cost_above_12_is_not_offered() {
    let (mut runner, me, opp) = win_runner(&["DRACO-13"], &[]);
    win_battle(&mut runner, me, opp);
    assert!(non_pass(&runner).is_empty(), "play cost 13 exceeds the cap");
}

#[test]
fn ex13_045_losing_or_no_battle_does_not_trigger() {
    let mut runner = builder().hand(0, &["DRACO-D"]).start();
    let me = runner.place_on_field(0, CARD_ID, Some(0));
    let _ = me;
    let strong = runner.place_on_field(1, "OPP-STRONG", Some(0));
    runner.battle_digimon(me, strong);
    let _ = runner.auto_resolve();
    assert!(!field_ids(&runner, 0).contains(&"DRACO-D".to_string()));
}

#[test]
fn ex13_045_win_trigger_decline_keeps_the_opt() {
    let (mut runner, me, opp) = win_runner(&["DRACO-D"], &[]);
    win_battle(&mut runner, me, opp);
    runner.execute_action(0, PASS).expect("decline");
    let _ = runner.auto_resolve();
    let opp2 = runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.battle_digimon(me, opp2);
    assert!(
        matches!(runner.pending_kind(), Some(SelectionKind::UnionZone { .. })),
        "declining did not spend the OPT"
    );
}

#[test]
fn ex13_045_win_trigger_is_once_per_turn() {
    let (mut runner, me, opp) = win_runner(&["DRACO-D", "DRACO-D"], &[]);
    win_battle(&mut runner, me, opp);
    let a = non_pass(&runner)[0];
    runner.execute_action(0, a).expect("play");
    let _ = runner.auto_resolve();
    let opp2 = runner.place_on_field(1, "OPP-WEAK", Some(0));
    runner.battle_digimon(me, opp2);
    assert!(runner.pending_selection().is_none(), "OPT spent");
}
