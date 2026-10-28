//! AD1-020 Tommy, Takuya, & Zoe — Tamer, Blue+Red+Green, Cost 5, no traits.
//!
//! # Card text (official bundle `data/card_bundles/AD1-020.md`; image checked)
//!
//! ```text
//! [Security] Play this card without paying the cost.
//! [Start of Your Main Phase] [On Play] You may place up to 2 [Hybrid] trait
//! cards with different colors from your hand or trash under this Tamer. If
//! this effect placed, <Draw 1>. Then, if there are 4 or more [Hybrid] trait
//! cards under this Tamer, gain 2 memory.
//! (Rule) Name: Also treated as [Tommy Himi]/[Takuya Kanbara]/[Zoe Orimoto].
//! Inherited Effect: [End of Your Turn] [Once Per Turn] By attacking with this
//! Digimon with the [Hybrid] or [Ten Warriors] trait, it gains
//! <Security A. +1> for the attack.
//! ```
//!
//! cards.json labels the inherited box "Security Effect"; the official DB and
//! the card image print it as the Inherited Effect (DCGO `SetIsInheritedEffect`).
//! Q&A: a multi-colour card differs in colour if any part differs (same ruling
//! as sister card AD1-023, Q6113).
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/AD1/Blue/AD1_020.cs` — NameIsAlso; shared
//! OP/SOMP coroutine (non-optional trigger; the per-card "Do not place" choice
//! is the opt-out; draw iff any placed; independent ≥4 Hybrid sources check);
//! OnEndTurn inherited (`isOptional`, max 1/turn, `CanActivate` = carrier
//! Digimon with Hybrid/Ten Warriors top AND `CanAttack`; `SelectAttackEffect`
//! with `SetCanNotSelectNotAttack`, then S-Attack +1 `UntilEndAttack`);
//! `PlaySelfTamerSecurityEffect`.

use digimon_engine::{
    action::{
        build_action_mask,
        space::{PASS, TRASH_EFFECT_START},
    },
    debug_runner::{make_test_card, DebugRunner},
    enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType},
    permanent::PermanentHandle,
    TriggerSource,
};

const ID: &str = "AD1-020";

fn setup(hand: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder().dsl_card(ID).expect("AD1-020 YAML");
    for (id, colors, tr) in [
        ("R", vec![CardColor::Red], "Hybrid"),
        ("B", vec![CardColor::Blue], "Hybrid"),
        ("RB", vec![CardColor::Red, CardColor::Blue], "Hybrid"),
        ("W", vec![CardColor::Yellow], "Ten Warriors"),
        ("N", vec![CardColor::Red], "Other"),
    ] {
        let mut c = make_test_card(id, id);
        c.colors = colors;
        c.traits = vec![tr.into()];
        c.dp = Some(9000);
        b = b.add_card(c);
    }
    let mut t = make_test_card("T", "Tamer");
    t.card_kind = CardKind::Tamer;
    t.level = None;
    t.dp = None;
    b = b.add_card(t);
    b.hand(0, hand)
        .deck(0, &["N"; 20])
        .deck(1, &["N"; 20])
        .security(0, &["R", "B"])
        .security(1, &["T"; 5])
        .memory(5)
        .start()
}

fn trigger(r: &mut DebugRunner, h: PermanentHandle, t: EffectTiming) {
    r.game.enqueue_triggered(t, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn pick(r: &mut DebugRunner, a: u16) {
    let p = r.pending_selection().unwrap().selecting_player;
    assert!(build_action_mask(&r.game, p)[a as usize] > 0.5, "action {a} masked");
    r.execute_action(p, a).unwrap();
}

fn accept(r: &mut DebugRunner) {
    let a = *r
        .pending_selection()
        .unwrap()
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    pick(r, a);
}

fn non_pass(r: &DebugRunner) -> usize {
    r.pending_selection()
        .unwrap()
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .count()
}

// ─── [On Play] / [Start of Your Main Phase] placement ───────────────────────

#[test]
fn ad1_020_mixed_colors_hand_and_trash_draw_once_both_timings() {
    for t in [EffectTiming::OnPlay, EffectTiming::StartOfYourMainPhase] {
        let mut r = setup(&["R", "N", "W"]);
        r.inject_trash(0, "R");
        r.inject_trash(0, "B");
        r.inject_trash(0, "N");
        let h = r.place_on_field(0, ID, None);
        trigger(&mut r, h, t);
        // Only Hybrid cards: hand R + trash R + trash B (not N, not Ten Warriors W).
        assert_eq!(non_pass(&r), 3, "{t:?}");
        assert!(r.pending_selection().unwrap().valid_action_ids.contains(&PASS), "'you may'");
        pick(&mut r, 0); // hand R
        // Trash R is now the same colour → not offered; trash B is.
        assert!(!r
            .pending_selection()
            .unwrap()
            .valid_action_ids
            .contains(&TRASH_EFFECT_START));
        pick(&mut r, TRASH_EFFECT_START + 1);
        assert!(r.pending_selection().is_none());
        assert_eq!(r.game.players[0].battle_area[0].digivolution_cards().len(), 3, "top + 2 placed");
        assert_eq!(r.hand_size(0), 3, "3 - 1 placed + 1 drawn");
        assert_eq!(r.memory(), 5, "only 2 Hybrid under → no memory");
    }
}

#[test]
fn ad1_020_dual_color_cards_count_as_different_colors() {
    let mut r = setup(&["RB", "RB"]);
    let h = r.place_on_field(0, ID, None);
    trigger(&mut r, h, EffectTiming::OnPlay);
    pick(&mut r, 0);
    pick(&mut r, 1);
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.game.players[0].battle_area[0].digivolution_cards().len(), 3, "top + 2");
}

#[test]
fn ad1_020_decline_places_nothing_and_does_not_draw() {
    let mut r = setup(&["R"]);
    let h = r.place_on_field(0, ID, None);
    trigger(&mut r, h, EffectTiming::OnPlay);
    pick(&mut r, PASS);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.game.players[0].battle_area[0].digivolution_cards().len(), 1, "only the Tamer itself");
}

#[test]
fn ad1_020_four_hybrid_threshold_independent_of_placement() {
    for n in [3, 4, 6] {
        let mut r = setup(&["B"]);
        let mut stack = vec!["R"; n];
        stack.push(ID);
        let h = r.place_stack(0, &stack);
        trigger(&mut r, h, EffectTiming::StartOfYourMainPhase);
        pick(&mut r, PASS);
        assert_eq!(r.hand_size(0), 1, "declined → no draw");
        assert_eq!(r.memory(), if n >= 4 { 7 } else { 5 }, "n={n}");
    }
}

#[test]
fn ad1_020_partial_placement_reaches_threshold() {
    let mut r = setup(&["B"]);
    let h = r.place_stack(0, &["R", "R", "R", ID]);
    trigger(&mut r, h, EffectTiming::OnPlay);
    pick(&mut r, 0);
    assert!(r.pending_selection().is_none(), "no other colour left");
    assert_eq!(r.hand_size(0), 1, "placed B, drew 1");
    assert_eq!(r.memory(), 7);
}

#[test]
fn ad1_020_no_candidates_no_draw_and_non_hybrid_sources_dont_count() {
    let mut r = setup(&["N", "W"]);
    let h = r.place_stack(0, &["R", "R", "R", "W", "N", ID]);
    trigger(&mut r, h, EffectTiming::OnPlay);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 2);
    assert_eq!(r.memory(), 5, "3 Hybrid + Ten Warriors + Other → below 4");
}

// ─── Rule: name aliases ──────────────────────────────────────────────────────

#[test]
fn ad1_020_rule_names_in_hand_and_sources() {
    let mut r = setup(&[ID]);
    for name in ["Tommy Himi", "Takuya Kanbara", "Zoe Orimoto"] {
        assert!(r.game.players[0].hand[0].contains_card_name(name, &r.game.card_data));
    }
    r.place_stack(0, &[ID, "R"]);
    for name in ["Tommy Himi", "Takuya Kanbara", "Zoe Orimoto"] {
        assert!(r.game.players[0].battle_area[0].card_sources[0]
            .contains_card_name(name, &r.game.card_data));
    }
}

// ─── Inherited: [End of Your Turn][OPT] attack with Security A. +1 ──────────

fn sa_change(r: &DebugRunner, h: PermanentHandle) -> i32 {
    r.modifiers()
        .get(h, ModifierType::SecurityAttackChange)
        .iter()
        .map(|e| e.value)
        .sum()
}

#[test]
fn ad1_020_inherited_attack_checks_one_extra_security_for_the_attack() {
    for top in ["R", "W"] {
        let mut r = setup(&[]);
        r.game.turn_count = 3;
        let h = r.place_stack(0, &[ID, top]);
        trigger(&mut r, h, EffectTiming::EndOfYourTurn);
        let view = r.pending_selection_view().expect("activation prompt");
        assert!(view.is_optional, "{top}: optional activation");
        accept(&mut r);
        // "By attacking": once activated, the attack is not declinable.
        if let Some(p) = r.pending_selection() {
            assert!(!p.valid_action_ids.contains(&PASS), "{top}: attack is the cost");
        }
        r.auto_resolve().unwrap();
        assert_eq!(r.game.players[1].security.len(), 3, "{top}: 1 + 1 checks");
        assert!(r.game.players[0].battle_area[0].is_suspended, "{top}: attacked");
        assert_eq!(sa_change(&r, h), 0, "{top}: +1 lasts only for the attack");
    }
}

#[test]
fn ad1_020_inherited_once_per_turn_and_decline_keeps_use() {
    let mut r = setup(&[]);
    r.game.turn_count = 3;
    let h = r.place_stack(0, &[ID, "R"]);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    pick(&mut r, PASS);
    assert_eq!(r.game.players[1].security.len(), 5);
    assert!(!r.game.players[0].battle_area[0].is_suspended);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(r.pending_selection().is_some(), "declined activation keeps OPT");
    accept(&mut r);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[1].security.len(), 3);
    r.game.players[0].battle_area[0].is_suspended = false;
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(r.pending_selection().is_none(), "once per turn");
}

#[test]
fn ad1_020_inherited_requires_hybrid_or_ten_warriors_and_an_able_attacker() {
    // Non-Hybrid / non-Ten Warriors carrier.
    let mut r = setup(&[]);
    r.game.turn_count = 3;
    let h = r.place_stack(0, &[ID, "N"]);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(r.pending_selection().is_none(), "trait gate");
    // Suspended carrier cannot pay "by attacking" → not offered.
    let mut r = setup(&[]);
    r.game.turn_count = 3;
    let h = r.place_stack(0, &[ID, "R"]);
    r.game.players[0].battle_area[0].is_suspended = true;
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(r.pending_selection().is_none(), "cannot attack → cannot activate");
    assert_eq!(r.game.players[1].security.len(), 5);
}

#[test]
fn ad1_020_tamer_itself_has_no_end_of_turn_effect() {
    let mut r = setup(&[]);
    r.game.turn_count = 3;
    let h = r.place_on_field(0, ID, None);
    trigger(&mut r, h, EffectTiming::EndOfYourTurn);
    assert!(r.pending_selection().is_none(), "inherited only");
}

// ─── [Security] ──────────────────────────────────────────────────────────────

#[test]
fn ad1_020_security_dispatch_plays_self_free() {
    let mut r = setup(&[]);
    r.game.players[0].security.clear();
    r.add_to_hand(0, ID);
    let c = r.game.players[0].hand.pop().unwrap();
    r.game.players[0].security.push(c);
    let attacker = r.place_on_field(1, "N", Some(0));
    r.game.turn_player_idx = 1;
    r.game.turn_count = 3;
    r.game.current_phase = GamePhase::Main;
    let memory = r.memory();
    let _ = r.attack_player(attacker, 0, false);
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),
        ID
    );
    assert_eq!(r.memory(), memory);
}
