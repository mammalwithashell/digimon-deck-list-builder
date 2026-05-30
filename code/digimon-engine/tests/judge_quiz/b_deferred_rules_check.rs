//! Cluster B — rules-check deferred until the ongoing effect fully resolves.
//!
//! Questions (see `card-resolution.md`):
//!   Q6  Pillomon (BT9-033) at 0 DP not deleted until Flame Hellscythe (BT8-109)
//!       resolves — judge: NO (can't play a Digimon yet).
//!   Q7  Eye of the Gorgon (BT9-108) deletes Pillomon (BT9-033) then plays a Lv3
//!       — judge: YES (sequential sub-effects).
//!   Q8  Burst-Digivolve stack (BT13-020/AD1-016/BT21-044/BT21-042/EX4-005/
//!       BT21-004); Comet Hammer (BT23-096) de-digivolves to Agumon — judge:
//!       Agumon trashed → Koromon trashed (DP-less can't remain).
//!   Q13 Nyabootmon (BT22-042)+ShoeShoemon (P-165) vs Rapidmon (X Antibody)
//!       (BT16-101) — judge: −6000 DP.
//!   Q14 Same vs ShineGreymon: Ruin Mode (EX4-074) — judge: −6000 DP.
//!   Q24 Hudiemon (BT23-101)+Tentomon (BT23-037)+Kokomon (EX6-004) vs Rapidmon
//!       (X Antibody) (BT16-101) — judge: 3000 DP (Tentomon deleted by rules
//!       check before Kokomon's trigger).
//!
//! Scenarios authored under tasks §4.
//!
//! All six are BLOCKED-CARD: each needs ≥1 unimplemented card (see
//! card-resolution.md §"Implementation status"). Stubs `#[ignore]`-d on the
//! specific missing card(s); promote once authored (cluster-B authoring, §4).

#![allow(unused_imports)]

use std::sync::Arc;

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, EffectTiming, Expiry};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

fn lv4_digimon(id: &str, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(dp);
    c
}

// ─────────────────────────────────────────────────────────────────────────────
// Cluster-B CORE RULE PROBE — ≤0-DP deletion via state-based rules-check
// ─────────────────────────────────────────────────────────────────────────────
//
// Every cluster-B question (Q6, Q8, Q13, Q14, Q24) turns on the same rule: a
// Digimon driven to 0 DP or below is deleted by a GAME rules-check that runs
// AFTER the ongoing effect resolves — not mid-effect, but it must eventually
// run. This probe tests the rule with a synthetic Digimon (no quiz card needed),
// so it is NOT blocked on card authoring.
//
// Faithful behavior: after an effect reduces a battle-area Digimon to ≤0 DP and
// that effect's resolution completes (the effect queue drains), a rules-check
// deletes the Digimon.

/// Sanity: a non-reduced Digimon is NOT deleted by the post-effect drain.
#[test]
fn zero_dp_probe_healthy_digimon_survives_drain() {
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("VICTIM", 3000))
        .memory(10)
        .start();
    let _ = r.place_on_field(0, "VICTIM", Some(0));
    r.game.drain_effect_queue();
    assert_eq!(r.battle_area_size(0), 1, "healthy Digimon must remain");
}

/// CORE RULE — a Digimon reduced to ≤0 DP by an effect must be deleted once the
/// effect resolves. Pins the general state-based rules-check
/// (`Game::run_state_based_rules_check`), invoked at the outermost
/// `drain_effect_queue` boundary (was Arts-only via `run_rule_check_after_arts`).
///
/// RESOLVED (2026-05-29): the general state-based ≤0-DP rules-check
/// (`Game::run_state_based_rules_check`) now runs at the outermost
/// `drain_effect_queue` boundary, so VICTIM at -1000 DP is deleted once the
/// effect resolves. Was G-NO-GENERAL-ZERO-DP-RULES-CHECK (cluster B root).
#[test]
fn zero_dp_probe_reduced_digimon_deleted_after_effect_resolves() {
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("VICTIM", 3000))
        .add_card(make_test_card("SRC", "Src"))
        .memory(10)
        .start();
    let victim = r.place_on_field(0, "VICTIM", Some(0));
    let src = r.place_on_field(1, "SRC", None);
    let src_card = r.game.player(1).battle_area[0].top_card().handle();

    // Simulate an opponent effect reducing VICTIM to -1000 DP, then resolving.
    {
        let mut ctx = EffectContext::new(&mut r.game, src_card, Some(src), 1);
        ctx.add_dp_modifier(victim, -4000, Expiry::Permanent);
    }
    assert_eq!(
        r.game.effective_dp(victim),
        Some(-1000),
        "precondition: VICTIM is at -1000 effective DP"
    );

    // The effect has resolved — drain the queue (the post-effect boundary).
    r.game.drain_effect_queue();

    // Judge-correct (cluster B): the ≤0-DP Digimon is deleted by the rules check.
    assert_eq!(
        r.battle_area_size(0),
        0,
        "a Digimon at ≤0 DP must be deleted by a state-based rules-check after \
         the effect resolves (see G-NO-GENERAL-ZERO-DP-RULES-CHECK)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Synthetic rule-check TIMING regression tests (tasks §2.6)
// ─────────────────────────────────────────────────────────────────────────────

/// Applies `value` DP to a fixed target when its (synthetic) WhenAttacking
/// timing fires — stages a single queued effect that drives a Digimon's DP.
struct DpModWhenAttacking {
    target: PermanentHandle,
    value: i32,
}
impl CardEffect for DpModWhenAttacking {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let target = self.target;
        let value = self.value;
        vec![Effect::when_attacking(card)
            .name("synthetic dp mod")
            .process(move |ctx| {
                ctx.add_dp_modifier(target, value, Expiry::Permanent);
            })
            .build()]
    }
}

/// Reduces the target to ≤0 DP then restores it ABOVE 0, both within ONE effect
/// body — pins "no mid-effect deletion".
struct ReduceThenRestoreWithinOneEffect {
    target: PermanentHandle,
}
impl CardEffect for ReduceThenRestoreWithinOneEffect {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let target = self.target;
        vec![Effect::when_attacking(card)
            .name("reduce then restore (one effect)")
            .process(move |ctx| {
                ctx.add_dp_modifier(target, -4000, Expiry::Permanent); // 3000 → -1000
                ctx.add_dp_modifier(target, 5000, Expiry::Permanent); //  -1000 → 4000
            })
            .build()]
    }
}

/// Q6-analog (§2.6a) — a Digimon driven to ≤0 DP MID-effect is NOT deleted until
/// the effect fully resolves. One effect reduces VICTIM to -1000 then restores it
/// to +4000; with the old inline mid-effect deletion VICTIM would be gone after
/// the reduce (the restore a no-op). Faithful: the rules-check defers to the
/// resolution boundary, so VICTIM survives at 4000 (Q6/Q13/Q14 root rule).
#[test]
fn q6_analog_no_mid_effect_deletion_within_single_effect() {
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("VICTIM", 3000))
        .add_card(make_test_card("SRC", "Src"))
        .memory(10)
        .start();
    let victim = r.place_on_field(0, "VICTIM", Some(0));
    let src = r.place_on_field(0, "SRC", None);
    r.register_effect(
        "SRC",
        Arc::new(ReduceThenRestoreWithinOneEffect { target: victim }),
    );
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(src));
    r.game.drain_effect_queue();
    assert_eq!(
        r.game.effective_dp(victim),
        Some(4000),
        "VICTIM restored within the same effect must survive (no mid-effect deletion)"
    );
    assert_eq!(r.battle_area_size(0), 2, "VICTIM + SRC both remain");
}

/// Q24-analog (§2.6b) — the rules-check fires BETWEEN top-level queued effects.
/// A turn-player effect drives VICTIM to ≤0; the rules-check deletes it BEFORE a
/// separate opponent "+DP save" trigger resolves. If the check only ran at
/// queue-empty, both would resolve (net -2000 → VICTIM at 1000) and VICTIM would
/// survive — so this distinguishes the between-effects timing (D3).
#[test]
fn q24_analog_rules_check_deletes_between_queued_effects() {
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("VICTIM", 3000))
        .add_card(make_test_card("DAMAGER", "Damager"))
        .add_card(make_test_card("HEALER", "Healer"))
        .memory(10)
        .start();
    r.set_first_player(0); // player 0 is turn player → DAMAGER resolves first
    let victim = r.place_on_field(1, "VICTIM", Some(0));
    let damager = r.place_on_field(0, "DAMAGER", None);
    let healer = r.place_on_field(1, "HEALER", None);
    r.register_effect(
        "DAMAGER",
        Arc::new(DpModWhenAttacking {
            target: victim,
            value: -4000,
        }),
    );
    r.register_effect(
        "HEALER",
        Arc::new(DpModWhenAttacking {
            target: victim,
            value: 2000,
        }),
    );
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(damager));
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(healer));
    r.game.drain_effect_queue();
    assert_eq!(
        r.battle_area_size(1),
        1,
        "VICTIM deleted by the rules-check after the damager resolves, before the heal could save it"
    );
}

/// Q6 — Pillomon (BT9-033) at 0 DP not deleted until Flame Hellscythe (BT8-109)
/// resolves. Judge: NO (can't play a Digimon yet).
#[test]
#[ignore = "BLOCKED-CARD: needs BT8-109 (Flame Hellscythe). BT9-033 implemented."]
fn q6_pillomon_zero_dp_not_deleted_until_flame_hellscythe_resolves() {}

/// Q7 — Eye of the Gorgon (BT9-108) deletes Pillomon (BT9-033) with sub-effect 1,
/// then plays a Lv3 with sub-effect 2. Judge: YES.
///
/// Board (card-resolution.md Q7): Player 1 controls Pillomon (BT9-033),
/// `[All Turns] Players can't play Digimon by effects` — a `target_player: any`
/// floodgate that installs `CannotPlayDigimonByEffect` on BOTH players. Player 0
/// has a purple Lv.3 Digimon in trash and plays Eye of the Gorgon (BT9-108):
///   sub-effect 1 — Delete 1 opponent unsuspended Digimon (Pillomon);
///   sub-effect 2 — "If you do, you may play 1 purple Lv.3 Digimon from your
///                   trash without paying its cost."
/// The two sub-effects resolve SEQUENTIALLY within the one [Main] effect. By the
/// time sub-effect 2 runs, sub-effect 1 has already deleted Pillomon, so its
/// `CannotPlayDigimonByEffect` floodgate is gone and the trash-play is no longer
/// blocked. Judge: YES — the Lv.3 play succeeds.
///
/// What this pins (so it cannot pass for the wrong reason):
///   1. CONTROL: while Pillomon is alive, `CannotPlayDigimonByEffect` is really
///      installed on Player 0 — a direct `play_from_trash_free` IS blocked. (If
///      the floodgate were never active, the test would false-pass.)
///   2. The real BT9-108 [Main] effect runs through the engine's action path
///      (`decode_action`, which ticks declarative state between selections), so
///      after sub-effect 1 deletes Pillomon the floodgate clears before
///      sub-effect 2's play resolves.
///   3. The purple Lv.3 leaves trash and enters Player 0's battle area — the
///      sequential play actually happened.
#[test]
fn q7_eye_of_the_gorgon_sequential_delete_then_play() {
    use digimon_engine::action::space::encode_attack;
    use digimon_engine::card_source::CardSource;
    use digimon_engine::effect_context::EffectContext;
    use digimon_engine::enums::{CardColor, ModifierType};
    use digimon_engine::selection::SelectionKind;

    // A purple Lv.3 Digimon (no [On Play]) — the sub-effect-2 trash-play target.
    let mut purple_l3 = make_test_card("PURPLE-L3", "Purple L3");
    purple_l3.card_kind = CardKind::Digimon;
    purple_l3.colors = vec![CardColor::Purple];
    purple_l3.level = Some(3);
    purple_l3.dp = Some(2000);

    let mut r = DebugRunner::builder()
        .dsl_card("BT9-108")
        .expect("BT9-108 Eye of the Gorgon loads")
        .dsl_card("BT9-033")
        .expect("BT9-033 Pillomon loads")
        .add_card(purple_l3)
        .hand(0, &["BT9-108"])
        .memory(10)
        .start();
    r.skip_mulligan();

    // Player 1's Pillomon — `[All Turns] Players can't play Digimon by effects`.
    let pillomon = r.place_on_field(1, "BT9-033", Some(0));
    // Player 0's purple Lv.3 seeded into trash (the sub-effect-2 candidate).
    {
        let data_idx = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "PURPLE-L3")
            .expect("PURPLE-L3 in card_data");
        let next_idx = r.game.next_card_index();
        r.game.players[0]
            .trash
            .push(CardSource::new(data_idx, 0, next_idx));
    }

    // Materialize Pillomon's floodgate.
    r.game.tick_declarative_effects();
    assert!(
        r.game
            .modifiers
            .player_has(0, ModifierType::CannotPlayDigimonByEffect),
        "precondition: Pillomon's [All Turns] floodgate installs \
         CannotPlayDigimonByEffect on Player 0"
    );

    // CONTROL — while Pillomon is alive, a direct effect-play of the purple Lv.3
    // from trash is BLOCKED by the floodgate (returns None). This proves the gate
    // is genuinely active; without it the Q7 outcome would be vacuous. Uses the
    // SAME engine entry point BT9-108's sub-effect 2 uses
    // (`play_from_trash_free_unsuspended`, the `play_from_trash_free` step body).
    {
        let src_card = r.game.player(1).battle_area[pillomon.index as usize]
            .top_card()
            .handle();
        let trash_card = r.game.player(0).trash[0].handle();
        let mut ctx = EffectContext::new(&mut r.game, src_card, Some(pillomon), 0);
        let blocked = ctx.play_from_trash_free_unsuspended(trash_card);
        assert!(
            blocked.is_none(),
            "CONTROL: with Pillomon alive, playing a Digimon from trash by effect \
             must be blocked by CannotPlayDigimonByEffect"
        );
    }
    assert_eq!(
        r.battle_area_size(0),
        0,
        "CONTROL must not have actually played anything"
    );
    assert_eq!(
        r.trash_size(0),
        1,
        "the purple Lv.3 is still in Player 0's trash after the blocked control play"
    );

    // ── Now run the REAL Eye of the Gorgon [Main] effect ──────────────────────
    // Play BT9-108 from hand; this installs the mandatory delete prompt.
    r.play(0, 0).expect("play BT9-108 from hand");
    let view = r
        .pending_selection_view()
        .expect("[Main] sub-effect 1: delete-target prompt installs");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(
        view.valid_action_ids,
        vec![encode_attack(0, pillomon.index as u16)],
        "Pillomon (unsuspended) is the sole legal delete target"
    );

    // Resolve the delete via `decode_action` so declarative state is re-ticked
    // AFTER Pillomon is removed — the engine's real between-selection refresh
    // that clears the floodgate before sub-effect 2 resolves.
    r.game
        .decode_action(view.valid_action_ids[0], 0);

    // Sub-effect 1 happened: Pillomon deleted, floodgate cleared.
    assert_eq!(
        r.battle_area_size(1),
        0,
        "sub-effect 1 deleted Pillomon"
    );
    assert!(
        !r.game
            .modifiers
            .player_has(0, ModifierType::CannotPlayDigimonByEffect),
        "deleting Pillomon clears its CannotPlayDigimonByEffect floodgate"
    );

    // Sub-effect 2: the "If you do, you may play 1 purple Lv.3 from trash" prompt.
    let play_view = r
        .pending_selection_view()
        .expect("[Main] sub-effect 2: trash-play prompt installs after the delete");
    assert_eq!(
        play_view.kind,
        SelectionKind::Trash,
        "sub-effect 2 selects a Digimon from your own trash"
    );
    assert_eq!(
        play_view.selecting_player, 0,
        "the controller chooses the card to play"
    );
    assert!(
        play_view.is_optional,
        "the printed 'you may play' makes the trash play optional"
    );
    let pick = play_view.valid_action_ids[0];

    let trash_before = r.trash_size(0);
    let battle_before = r.battle_area_size(0);
    // Resolve the play via `decode_action` (the engine's action path).
    r.game.decode_action(pick, 0);
    r.auto_resolve().ok();

    // Judge: YES — the sequential play succeeds.
    assert_eq!(
        r.battle_area_size(0),
        battle_before + 1,
        "sub-effect 2 played the purple Lv.3 — sequential resolution let it land \
         after Pillomon's floodgate was removed by sub-effect 1 (judge-quiz Q7: YES)"
    );
    assert!(
        r.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&r.game.card_data) == "PURPLE-L3"),
        "the purple Lv.3 from trash is now on Player 0's battle area"
    );
    assert_eq!(
        r.trash_size(0),
        trash_before - 1,
        "the played purple Lv.3 left the trash"
    );
}

/// Q8 — Burst-Digivolve stack; Comet Hammer (BT23-096) de-digivolves to Agumon
/// (EX4-005); at EoT Burst trashes the top, DP-less Koromon (BT21-004) can't
/// remain. Judge: Agumon trashed → Koromon trashed.
#[test]
#[ignore = "BLOCKED-CARD: needs BT13-020, AD1-016, BT21-044, BT21-042, EX4-005, BT21-004. BT23-096 implemented."]
fn q8_burst_digivolve_dp_less_digimon_trash_chain_at_eot() {}

/// Q13 — Nyabootmon (BT22-042)+ShoeShoemon (P-165); Rapidmon (X Antibody)
/// (BT16-101) measured before ShoeShoemon's On Play. Judge: −6000 DP.
#[test]
#[ignore = "BLOCKED-CARD: needs BT16-101 (Rapidmon X), ST17-07 (Rapidmon). BT22-042, P-165 implemented."]
fn q13_nyabootmon_dp_minus_measured_before_shoeshoemon_on_play() {}

/// Q14 — Same vs ShineGreymon: Ruin Mode (EX4-074); ShoeShoemon enters and gets
/// −5000 but isn't deleted until Nyabootmon resolves. Judge: −6000 DP.
#[test]
#[ignore = "BLOCKED-CARD: needs BT16-101 (Rapidmon X). BT22-042, P-165, EX4-074 implemented."]
fn q14_nyabootmon_dp_minus_vs_shinegreymon_ruin_mode() {}

/// Q24 — Hudiemon (BT23-101) <Alliance> Tentomon (BT23-037); Tentomon suspended →
/// −4000 from Rapidmon X (BT16-101) → deleted by rules check before Kokomon
/// (EX6-004) [Your Turn] contributes. Judge: Hudiemon DP 3000.
#[test]
#[ignore = "BLOCKED-CARD: needs BT23-101 (Hudiemon), BT23-037 (Tentomon), EX6-004 (Kokomon), BT16-101 (Rapidmon X), ST17-07 (Rapidmon)."]
fn q24_hudiemon_alliance_partner_deleted_by_rules_check_before_trigger() {}
