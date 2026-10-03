//! P-240 Arcturusmon — Digimon, Lv.6, Purple/Black, Cost 13, DP 13000.
//! Mega / Virus / Dragonkin, VB.
//!
//! # Card text (card image + fandom wiki + DCGO — bundle / cards.json are empty)
//!
//! Digivolve: Lv.5 (purple / black circles): Cost 5.
//! Digivolve [Lv.5 w/[Gammamon] in text or w/[VB] trait]: Cost 4.
//! <Collision> <Piercing> <Reboot> <Blocker>
//! [On Play] [When Digivolving] <De-Digivolve 3> 1 of your opponent's Digimon.
//! Then, by placing 2 cards with [Gammamon] in their texts or the [VB] trait
//! from your trash as this Digimon's bottom digivolution cards, give 1 of your
//! opponent's Digimon "[Start of Your Main Phase] This Digimon attacks." until
//! their turn ends.
//! [On Deletion] You may play 1 [Proximamon] from your hand or trash without
//! paying the cost.
//! <Assembly -6> [Lv.5 × Lv.4 × Lv.3, all w/[Gammamon] in text or w/[VB] trait]
//! Inherited: [Opponent's Turn] [Once Per Turn] When one of your opponent's
//! Digimon attacks, you may change the attack target to this Digimon.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/P/Purple/P_240.cs`
//!
//! # Patterns
//! - Colour + conditional alt digivolve and 3-material Assembly (EX12-017).
//! - H-group printed keywords (Collision / Piercing / Reboot / Blocker).
//! - [OP]/[WD] De-Digivolve 3 + all-or-nothing "by placing 2 from trash" cost
//!   (`select_zone_cards { min 2, max 2, optional_zero }`) gating a granted
//!   forced attack (EX12-016 `grant_triggered_effect`).
//! - Optional [On Deletion] union-zone free play (P-170).
//! - Inherited [Opp Turn][OPT] attack redirect (EX12-056 / EX8-050).

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledCost, CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{
    encode_attack, PASS, PLAY_HAND_START, SECURITY_TARGET, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword, PlaySource};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "P-240";

fn digimon(
    id: &str,
    name: &str,
    level: u8,
    dp: i32,
    color: CardColor,
    traits: &[&str],
) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = level as u16;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn gammamon_text(id: &str, level: u8) -> CardData {
    let mut c = digimon(
        id,
        "Kausmon",
        level,
        level as i32 * 1000,
        CardColor::Red,
        &[],
    );
    c.effect_text =
        "[When Digivolving] If you have a Digimon with [Gammamon] in its text, draw 1.".into();
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-240 YAML parses and compiles")
        .add_card(digimon("VB5", "Wezenmon", 5, 7000, CardColor::Red, &["VB"]))
        .add_card(digimon("VB4", "Wezen4", 4, 5000, CardColor::Red, &["VB"]))
        .add_card(digimon("VB3", "Wezen3", 3, 3000, CardColor::Red, &["VB"]))
        .add_card(gammamon_text("GTXT", 4))
        .add_card(digimon(
            "PURPLE5",
            "Purple Five",
            5,
            7000,
            CardColor::Purple,
            &[],
        ))
        .add_card(digimon("RED5", "Red Five", 5, 7000, CardColor::Red, &[]))
        .add_card(digimon(
            "PLAIN4",
            "Plain Four",
            4,
            5000,
            CardColor::Red,
            &[],
        ))
        .add_card(digimon("OPP3", "Opp Three", 3, 3000, CardColor::Blue, &[]))
        .add_card(digimon("OPP4", "Opp Four", 4, 5000, CardColor::Blue, &[]))
        .add_card(digimon("OPP5", "Opp Five", 5, 7000, CardColor::Blue, &[]))
        .add_card(digimon("OPP6", "Opp Six", 6, 12000, CardColor::Blue, &[]))
        .add_card(digimon(
            "OPP-B",
            "Opp Bystander",
            4,
            4000,
            CardColor::Blue,
            &[],
        ))
        .add_card(digimon(
            "PROXIMA",
            "Proximamon",
            3,
            3000,
            CardColor::Purple,
            &[],
        ))
        .add_card(digimon("HOST", "Host", 6, 12000, CardColor::Purple, &[]))
        .add_card(digimon(
            "ATK",
            "Opp Attacker",
            5,
            9000,
            CardColor::Blue,
            &[],
        ))
        .add_card(digimon("FILL", "Fill", 3, 1000, CardColor::Red, &[]))
}

fn stack_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn find_perm(r: &DebugRunner, p: u8, id: &str) -> Option<PermanentHandle> {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .position(|perm| {
            !perm.card_sources.is_empty() && perm.top_card().card_id(&r.game.card_data) == id
        })
        .map(|i| PermanentHandle {
            player: p,
            index: i as u8,
        })
}

fn trash_has(r: &DebugRunner, p: u8, id: &str) -> bool {
    r.game.players[p as usize]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == id)
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, source: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(source));
    r.game.drain_effect_queue();
}

fn pick_perm(r: &mut DebugRunner, target: PermanentHandle) {
    let view = r.pending_selection_view().expect("permanent prompt");
    let id = digimon_engine::action::space::ATTACK_START + target.index as u16;
    assert!(
        view.valid_action_ids.contains(&id),
        "{target:?} not selectable: {view:?}"
    );
    r.execute_action(view.selecting_player, id).expect("pick");
}

fn pick_trash(r: &mut DebugRunner, id: &str) {
    let idx = r.game.players[0]
        .trash
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .expect("in trash");
    let view = r.pending_selection_view().expect("trash prompt");
    let action = TRASH_EFFECT_START + idx as u16;
    assert!(
        view.valid_action_ids.contains(&action),
        "{id} not selectable: {view:?}"
    );
    r.execute_action(view.selecting_player, action)
        .expect("pick trash");
}

// ═══ Section 1 — structure ═══════════════════════════════════════════════════

#[test]
fn p_240_metadata_keywords_and_alt_paths() {
    let r = builder().build();
    let data = r
        .game
        .card_data
        .iter()
        .find(|c| c.card_id == CARD_ID)
        .unwrap();
    assert_eq!(data.card_kind, CardKind::Digimon);
    assert_eq!(data.level, Some(6));
    assert_eq!(data.dp, Some(13000));
    assert_eq!(data.play_cost, 13);
    assert_eq!(data.colors, vec![CardColor::Purple, CardColor::Black]);
    assert!(data.traits.iter().any(|t| t == "Dragonkin"));
    assert!(data.traits.iter().any(|t| t == "VB"));
    for kw in [
        Keyword::Collision,
        Keyword::Piercing,
        Keyword::Reboot,
        Keyword::Blocker,
    ] {
        assert!(data.keywords.contains(&kw), "{kw:?} printed keyword");
    }
    let card = r.compiled_card(CARD_ID).unwrap();
    let assembly = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Assembly)
        .expect("Assembly -6");
    assert_eq!(assembly.cost, Some(CompiledCost::Literal(6)));
    assert_eq!(assembly.materials.len(), 3);
    assert_eq!(
        card.alt_paths
            .iter()
            .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
            .count(),
        3,
        "purple Lv5 / black Lv5 (cost 5) + [Gammamon]/[VB] Lv5 (cost 4)"
    );
    let inherited = card.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
        _ => None,
    });
    let inherited = inherited.expect("inherited clause");
    assert!(inherited.once_per_turn && inherited.optional);
    assert!(inherited.when.contains(&CompiledTiming::OnOpponentAttack));
}

// ═══ Digivolution routes ═════════════════════════════════════════════════════

#[test]
fn p_240_digivolves_from_vb_lv5_for_4() {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    let base = r.place_on_field(0, "VB5", Some(0));
    let mem0 = r.game.memory;
    assert!(r
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve));
    assert_eq!(mem0 - r.game.memory, 4, "[VB] Lv.5 route costs 4");
}

#[test]
fn p_240_digivolves_from_purple_lv5_for_5_but_not_from_red_lv5() {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    let red = r.place_on_field(0, "RED5", Some(0));
    assert!(
        !r.game
            .digivolve_from_hand(0, 0, red.index as usize, PlaySource::ByDigivolve),
        "a red Lv.5 without [Gammamon]/[VB] has no route"
    );
    let base = r.place_on_field(0, "PURPLE5", Some(0));
    let mem0 = r.game.memory;
    assert!(r
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve));
    assert_eq!(mem0 - r.game.memory, 5);
}

#[test]
fn p_240_assembly_plays_for_7_stacking_lv5_lv4_lv3() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.skip_mulligan();
    r.inject_trash(0, "VB5");
    r.inject_trash(0, "GTXT");
    r.inject_trash(0, "VB3");
    r.inject_trash(0, "PLAIN4");
    let mem0 = r.game.memory;
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(r.game.pending_selection.is_some(), "Assembly flow surfaces");
    // Drive the material picks (Lv5 VB, Lv4 [Gammamon]-text, Lv3 VB).
    for id in ["VB5", "GTXT", "VB3"] {
        if let Some(view) = r.pending_selection_view() {
            let idx = r.game.players[0]
                .trash
                .iter()
                .position(|c| c.card_id(&r.game.card_data) == id);
            if let Some(idx) = idx {
                let a = TRASH_EFFECT_START + idx as u16;
                if view.valid_action_ids.contains(&a) {
                    r.execute_action(view.selecting_player, a)
                        .expect("material");
                    continue;
                }
            }
            r.execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("material");
        }
    }
    let _ = r.auto_resolve();
    let perm = find_perm(&r, 0, CARD_ID).expect("Arcturusmon in play");
    let stack = stack_ids(&r, perm);
    for id in ["VB5", "GTXT", "VB3"] {
        assert!(
            stack.contains(&id.to_string()),
            "{id} stacked under: {stack:?}"
        );
    }
    assert!(
        !stack.contains(&"PLAIN4".to_string()),
        "non-[Gammamon]/[VB] not a material"
    );
    assert_eq!(mem0 - r.game.memory, 7, "13 − 6 = 7");
}

// ═══ [On Play] / [When Digivolving] ══════════════════════════════════════════

#[test]
fn p_240_on_play_de_digivolve_3_then_place_2_grants_forced_attack() {
    let mut r = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    r.inject_trash(0, "VB4");
    r.inject_trash(0, "GTXT");
    r.inject_trash(0, "PLAIN4");
    let opp = r.place_stack(1, &["OPP3", "OPP4", "OPP5", "OPP6"]);
    let other = r.place_on_field(1, "OPP-B", Some(0));
    let arc = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, arc);

    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    assert!(!r.pending_is_optional(), "De-Digivolve target is mandatory");
    pick_perm(&mut r, opp);
    assert_eq!(
        stack_ids(&r, opp),
        vec!["OPP3".to_string()],
        "trashed 3 from the top"
    );

    // "By placing 2 ..." — exactly 2 or none.
    assert!(
        r.pending_is_optional(),
        "the placement is optional (zero allowed)"
    );
    pick_trash(&mut r, "VB4");
    pick_trash(&mut r, "GTXT");
    let stack = stack_ids(&r, arc);
    assert_eq!(stack.len(), 3, "2 sources added: {stack:?}");
    assert_eq!(stack.last().map(String::as_str), Some(CARD_ID));
    assert!(trash_has(&r, 0, "PLAIN4"));

    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OppField),
        "choose the forced attacker"
    );
    pick_perm(&mut r, other);
    assert_eq!(
        r.game
            .modifiers
            .granted_triggered_for_timing(other, EffectTiming::StartOfYourMainPhase)
            .len(),
        1
    );
    r.end_turn();
    let prompt = r
        .pending_selection()
        .expect("forced attack at opponent's main phase");
    assert_eq!(prompt.selecting_player, 1);
    assert!(!prompt.is_optional, "the attack is mandatory");
    assert!(prompt
        .valid_action_ids
        .contains(&encode_attack(other.index as u16, SECURITY_TARGET)));
}

#[test]
fn p_240_on_play_decline_placement_grants_nothing() {
    let mut r = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.inject_trash(0, "VB4");
    r.inject_trash(0, "GTXT");
    let opp = r.place_on_field(1, "OPP5", Some(0));
    let arc = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, arc);
    pick_perm(&mut r, opp);
    let view = r.pending_selection_view().expect("placement prompt");
    r.execute_action(view.selecting_player, PASS)
        .expect("decline");
    assert!(
        r.pending_selection().is_none(),
        "no forced-attack grant without placement"
    );
    assert_eq!(stack_ids(&r, arc).len(), 1);
    assert!(trash_has(&r, 0, "VB4") && trash_has(&r, 0, "GTXT"));
    assert!(r
        .game
        .modifiers
        .granted_triggered_for_timing(opp, EffectTiming::StartOfYourMainPhase)
        .is_empty());
}

#[test]
fn p_240_on_play_fewer_than_2_eligible_skips_placement() {
    let mut r = builder()
        .deck(0, &["FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.inject_trash(0, "VB4");
    r.inject_trash(0, "PLAIN4");
    let opp = r.place_on_field(1, "OPP5", Some(0));
    let arc = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, arc);
    pick_perm(&mut r, opp);
    assert!(
        r.pending_selection().is_none(),
        "only 1 eligible card → nothing to place"
    );
    assert_eq!(stack_ids(&r, arc).len(), 1);
}

#[test]
fn p_240_when_digivolving_runs_the_same_body() {
    let mut r = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL"])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    let opp = r.place_stack(1, &["OPP3", "OPP4"]);
    let base = r.place_on_field(0, "VB5", Some(0));
    assert!(r
        .game
        .digivolve_from_hand(0, 0, base.index as usize, PlaySource::ByDigivolve));
    // Possibly a trigger-order prompt first; then the De-Digivolve target.
    if matches!(r.pending_kind(), Some(SelectionKind::TriggerOrder)) {
        r.accept_optional_trigger().expect("order");
    }
    assert_eq!(r.pending_kind(), Some(SelectionKind::OppField));
    pick_perm(&mut r, opp);
    assert_eq!(
        stack_ids(&r, opp),
        vec!["OPP3".to_string()],
        "De-Digivolve stops at Lv.3"
    );
}

// ═══ [On Deletion] ═══════════════════════════════════════════════════════════

#[test]
fn p_240_on_deletion_plays_proximamon_free_from_trash() {
    let mut r = builder().memory(3).start();
    r.inject_trash(0, "PROXIMA");
    let arc = r.place_on_field(0, CARD_ID, Some(0));
    let mem0 = r.memory();
    r.game.delete_permanent_with_effects(arc);
    assert!(r.pending_is_optional(), "'You may play' — PASS declines");
    let view = r.pending_selection_view().expect("union pick");
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    r.execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick Proximamon");
    let _ = r.auto_resolve();
    assert!(find_perm(&r, 0, "PROXIMA").is_some());
    assert_eq!(r.memory(), mem0, "without paying the cost");
}

#[test]
fn p_240_on_deletion_decline_and_no_proximamon_cases() {
    let mut r = builder().hand(0, &["PROXIMA"]).memory(3).start();
    let arc = r.place_on_field(0, CARD_ID, Some(0));
    r.game.delete_permanent_with_effects(arc);
    assert!(matches!(
        r.pending_kind(),
        Some(SelectionKind::UnionZone { .. })
    ));
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(find_perm(&r, 0, "PROXIMA").is_none());

    let mut r = builder().hand(0, &["FILL"]).memory(3).start();
    let arc = r.place_on_field(0, CARD_ID, Some(0));
    r.game.delete_permanent_with_effects(arc);
    assert!(
        r.pending_selection().is_none(),
        "no [Proximamon] → no prompt"
    );
}

// ═══ Inherited [Opponent's Turn][OPT] redirect ═══════════════════════════════

fn redirect_setup() -> (
    DebugRunner,
    PermanentHandle,
    PermanentHandle,
    PermanentHandle,
) {
    let mut r = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(0, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1);
    r.game.enter_main_phase();
    r.game.memory = 10; // keep the opponent's turn alive across attacks
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let atk1 = r.place_on_field(1, "ATK", Some(0));
    let atk2 = r.place_on_field(1, "ATK", Some(0));
    (r, host, atk1, atk2)
}

#[test]
fn p_240_inherited_redirects_opponent_attack_to_this_digimon() {
    let (mut r, host, atk1, _atk2) = redirect_setup();
    let sec0 = r.security_count(0);
    r.attack_player(atk1, 0, false);
    assert!(r.pending_is_optional(), "'you may change'");
    r.accept_optional_trigger().expect("accept redirect");
    let _ = r.auto_resolve();
    assert_eq!(
        r.security_count(0),
        sec0,
        "attack redirected — no security check"
    );
    // 9000 attacker vs 12000 host: the attacker loses the redirected battle.
    assert!(
        find_perm(&r, 0, "HOST").is_some(),
        "the host battled and survived"
    );
    assert!(
        r.game.players[1].battle_area.len() == 1,
        "the redirected attacker was deleted in battle"
    );
}

#[test]
fn p_240_inherited_decline_leaves_attack_on_player() {
    let (mut r, host, atk1, _atk2) = redirect_setup();
    let sec0 = r.security_count(0);
    r.attack_player(atk1, 0, false);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), sec0 - 1, "security checked");
    assert!(find_perm(&r, 0, "HOST").is_some());
}

#[test]
fn p_240_inherited_is_once_per_turn_and_resets_next_opponent_turn() {
    let (mut r, host, atk1, atk2) = redirect_setup();
    r.attack_player(atk1, 0, false);
    r.accept_optional_trigger().expect("accept first redirect");
    let _ = r.auto_resolve();
    assert_eq!(r.game.turn_player(), 1, "still the opponent's turn");
    let atk2 = find_perm(&r, 1, "ATK").expect("second attacker");
    let sec = r.security_count(0);
    r.attack_player(atk2, 0, false);
    assert!(
        !r.pending_is_optional(),
        "OPT used this turn → no second redirect prompt"
    );
    let _ = r.auto_resolve();
    assert_eq!(r.security_count(0), sec - 1, "second attack hits security");

    // Next opponent turn: the lockout has cleared.
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 1);
    r.game.enter_main_phase();
    r.game.memory = 10;
    let atk3 = r.place_on_field(1, "ATK", Some(0));
    r.attack_player(atk3, 0, false);
    assert!(
        r.pending_is_optional(),
        "OPT reset on the next opponent turn"
    );
}

#[test]
fn p_240_inherited_does_not_fire_on_your_own_turn() {
    let mut r = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    r.game.enter_main_phase();
    let host = r.place_stack(0, &[CARD_ID, "HOST"]);
    let mine = r.place_on_field(0, "ATK", Some(0));
    r.attack_player(mine, 1, false);
    assert!(
        !r.pending_is_optional(),
        "[Opponent's Turn] only — my own attack never offers the redirect"
    );
}
