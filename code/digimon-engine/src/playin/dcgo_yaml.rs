//! Symbolic DCGO YAML exporter matching `code/tools/dcgo-harness/src/exam/scenario.rs`.
//!
//! Emits ordinary single-key mapping verbs (`do: { hatch: {} }`, `do: { play: { card: ..., from: ... } }`,
//! `do: { digivolve: { from: ..., using: ... } }`, `do: { move: { from: breeding } }`,
//! `do: { main: { on: ... } }`, `do: { attack: { attacker: ..., target: ... } }`,
//! `do: { pass: {} }`, `do: { select: { ... } }`, `do: { dna: { ... } }`, `do: { link: { ... } }`)
//! without `!tag` syntax so `Scenario::from_yaml` parses it cleanly.

use crate::action::build_action_mask;
use crate::action::explain::{explain_action, ActionKind};
use crate::action::space::{
    decode_attack, decode_digivolve, decode_field_effect, decode_source_select, ATTACK_START,
    BREEDING_TARGET, DIGIVOLVE_END, DIGIVOLVE_START, DNA_DIGIVOLVE_START, FIELD_EFFECT_SLOT_FOR_LINK,
    HAND_EFFECT_START, PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT, SECURITY_TARGET, SEL_REVEAL_START,
    SOURCE_SELECT_END, SOURCE_SELECT_START, TARGETS_PER_ATTACKER, TRASH_EFFECT_START,
};
use crate::enums::GamePhase;
use crate::game::Game;
use crate::inference::{match_refutation_trigger, RefutationTrigger};
use crate::policies::greedy_action;
use crate::search::decision_player;
use crate::selection::SelectionKind;

use super::deck_stacker::StackedSeatPlan;

fn yaml_scalar(s: &str) -> String {
    if s.is_empty()
        || s.contains(':')
        || s.contains('#')
        || s.contains('{')
        || s.contains('}')
        || s.contains('[')
        || s.contains(']')
        || s.contains('"')
        || s.contains('\'')
        || s.starts_with(' ')
        || s.ends_with(' ')
    {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

fn pick_primary_card_id(
    plans: &[StackedSeatPlan; 2],
    initial_game: &Game,
    full_actions: &[u16],
) -> String {
    let mut g = initial_game.clone();
    let mut non_filler: Option<String> = None;
    for &aid in full_actions {
        if g.game_over {
            break;
        }
        let pid = decision_player(&g);
        if g.current_phase != GamePhase::Mulligan {
            let exp = explain_action(&g, pid, aid);
            if let Some(cid) = exp.card_id {
                if !cid.starts_with("ST1-") {
                    return cid;
                }
                if non_filler.is_none() {
                    non_filler = Some(cid);
                }
            }
        }
        g.decode_action(aid, pid);
    }
    for plan in plans {
        for cid in &plan.draw_stack {
            if !cid.starts_with("ST1-") {
                return cid.clone();
            }
        }
    }
    non_filler
        .or_else(|| plans[0].draw_stack.first().cloned())
        .unwrap_or_else(|| "ST1-03".to_string())
}

/// Expand `prefix_actions + target_line` (auto-stepping opponent interrupts along `target_line`)
/// into concrete `(actor, action_id)` pairs on a cloned `initial_game`, tracking how many
/// main-deck cards each seat actually drew so `decks.p0.stack` / `decks.p1.stack` stays compact.
pub fn export_dcgo_yaml(
    puzzle_id: &str,
    seed: u64,
    plans: &[StackedSeatPlan; 2],
    initial_game: &Game,
    prefix_actions: &[u16],
    target_actions: &[u16],
    solver_seat: u8,
    refutation_triggers: &[RefutationTrigger],
    rest_deck_p0: &str,
    rest_deck_p1: &str,
) -> Result<String, String> {
    let mut g = initial_game.clone();
    let initial_deck_lens = [g.players[0].deck.len(), g.players[1].deck.len()];
    // Note: `initial_game` already dealt 5 starting hand cards per player; `finalize_mulligan`
    // pops 5 security cards, and play pops additional draws.
    let mut full_executed: Vec<u16> = Vec::new();
    let mut step_lines: Vec<(u8, String)> = Vec::new();

    // Combine prefix_actions and target_actions into a single walk, resolving auto-adversary
    // interrupts during the target_actions portion.
    let mut all_steps_game = Vec::new();
    for &aid in prefix_actions {
        if g.game_over {
            break;
        }
        let pid = decision_player(&g);
        let mask = build_action_mask(&g, pid);
        if (aid as usize) >= mask.len() || mask[aid as usize] <= 0.5 {
            return Err(format!(
                "DCGO export: prefix action {aid} illegal in phase {:?}",
                g.current_phase
            ));
        }
        all_steps_game.push((g.clone(), pid, aid));
        full_executed.push(aid);
        g.decode_action(aid, pid);
    }

    for &aid in target_actions {
        for _ in 0..64 {
            if g.game_over || decision_player(&g) == solver_seat {
                break;
            }
            let opp = decision_player(&g);
            let mask = build_action_mask(&g, opp);
            let legal: Vec<u16> = mask
                .iter()
                .enumerate()
                .filter_map(|(i, &m)| (m > 0.5).then_some(i as u16))
                .collect();
            if legal.is_empty() {
                break;
            }
            let opp_aid = if legal.len() == 1 {
                legal[0]
            } else if let Some(t) = match_refutation_trigger(&g, opp, refutation_triggers) {
                t
            } else {
                let gr = greedy_action(&g, &mask);
                if mask.get(gr as usize).copied().unwrap_or(0.0) > 0.5 {
                    gr
                } else {
                    legal[0]
                }
            };
            all_steps_game.push((g.clone(), opp, opp_aid));
            full_executed.push(opp_aid);
            g.decode_action(opp_aid, opp);
        }
        if g.game_over {
            break;
        }
        let pid = decision_player(&g);
        let mask = build_action_mask(&g, pid);
        if (aid as usize) < mask.len() && mask[aid as usize] > 0.5 {
            all_steps_game.push((g.clone(), pid, aid));
            full_executed.push(aid);
            g.decode_action(aid, pid);
        } else {
            break;
        }
    }

    let final_deck_lens = [g.players[0].deck.len(), g.players[1].deck.len()];

    // Convert `all_steps_game` (skipping Mulligan steps) into symbolic `ScenarioStep` strings.
    let mut idx = 0usize;
    while idx < all_steps_game.len() {
        let (ref pre_g, actor, aid) = all_steps_game[idx];
        if pre_g.current_phase == GamePhase::Mulligan {
            idx += 1;
            continue;
        }

        // Check for 3-step DNA digivolve (`ActionKind::DnaDigivolve` + 2 `Material` picks).
        if pre_g.pending_selection.is_none() {
            let exp = explain_action(pre_g, actor, aid);
            if exp.kind == ActionKind::DnaDigivolve
                && idx + 2 < all_steps_game.len()
                && all_steps_game[idx + 1].1 == actor
                && all_steps_game[idx + 2].1 == actor
            {
                let h_idx = (aid - DNA_DIGIVOLVE_START) as usize;
                let cid = exp
                    .card_id
                    .unwrap_or_else(|| "UNKNOWN".to_string());
                let dup_count = pre_g
                    .player(actor)
                    .hand
                    .iter()
                    .filter(|c| c.card_id(&pre_g.card_data) == cid)
                    .count();
                let from_ref = if dup_count > 1 {
                    format!("hand.{h_idx}")
                } else {
                    "hand".to_string()
                };
                let m1 = all_steps_game[idx + 1].2;
                let m2 = all_steps_game[idx + 2].2;
                let s1 = if m1 >= ATTACK_START { m1 - ATTACK_START } else { m1 };
                let s2 = if m2 >= ATTACK_START { m2 - ATTACK_START } else { m2 };
                step_lines.push((
                    actor,
                    format!(
                        "{{ dna: {{ card: {}, from: {}, materials: [field.{s1}, field.{s2}] }} }}",
                        yaml_scalar(&cid),
                        yaml_scalar(&from_ref)
                    ),
                ));
                idx += 3;
                continue;
            }
        }

        let do_str = symbolic_do_mapping(pre_g, actor, aid);
        step_lines.push((actor, do_str));
        idx += 1;
    }

    if step_lines.is_empty() {
        step_lines.push((0, "{ pass: {} }".to_string()));
    }

    let primary_card = pick_primary_card_id(plans, initial_game, &full_executed);
    let clause = format!("{primary_card}#effect#0");
    let clean_slug: String = puzzle_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = if clean_slug.trim_matches('-').is_empty() {
        "playin".to_string()
    } else {
        clean_slug.trim_matches('-').to_string()
    };

    // Each seat drew `5` starting hand + `(initial_deck_len - final_deck_len)` cards (5 security + turn/evo draws).
    let stack_len_for = |p: usize| -> usize {
        let popped = initial_deck_lens[p].saturating_sub(final_deck_lens[p]);
        (5 + popped + 2).min(plans[p].draw_stack.len())
    };
    let s0 = &plans[0].draw_stack[..stack_len_for(0)];
    let s1 = &plans[1].draw_stack[..stack_len_for(1)];

    let mut yaml = String::new();
    yaml.push_str(&format!("# Generated by digimon_engine::playin for puzzle {puzzle_id}\n"));
    yaml.push_str(&format!("card: {}\n", yaml_scalar(&primary_card)));
    yaml.push_str(&format!("clause: {}\n", yaml_scalar(&clause)));
    yaml.push_str(&format!(
        "interaction: {{ id: {}, source: combo, kind: positive }}\n",
        yaml_scalar(&format!("combo:{slug}"))
    ));
    yaml.push_str(&format!("seed: {seed}\n"));
    yaml.push_str("decks:\n");
    yaml.push_str("  p0:\n");
    yaml.push_str("    stack:\n");
    for c in s0 {
        yaml.push_str(&format!("      - {}\n", yaml_scalar(c)));
    }
    yaml.push_str(&format!("    rest: {}\n", yaml_scalar(rest_deck_p0)));
    yaml.push_str("  p1:\n");
    yaml.push_str("    stack:\n");
    for c in s1 {
        yaml.push_str(&format!("      - {}\n", yaml_scalar(c)));
    }
    yaml.push_str(&format!("    rest: {}\n", yaml_scalar(rest_deck_p1)));
    yaml.push_str("steps:\n");
    for (actor, do_expr) in &step_lines {
        yaml.push_str(&format!("  - actor: {actor}\n    do: {do_expr}\n"));
    }

    Ok(yaml)
}

fn symbolic_do_mapping(game: &Game, actor: u8, action_id: u16) -> String {
    if let Some(sel) = game.pending_selection.as_ref() {
        return symbolic_selection_mapping(game, sel, actor, action_id);
    }

    if game.current_phase == GamePhase::BlockTiming || game.current_phase == GamePhase::AllianceTiming
    {
        if action_id == PASS {
            return "{ select: { decline: true } }".to_string();
        }
        if (100..114).contains(&action_id) {
            let slot = action_id - 100;
            return format!("{{ select: {{ targets: [own.field.{slot}] }} }}");
        }
    }
    if game.current_phase == GamePhase::CounterTiming {
        if action_id == PASS {
            return "{ pass: {} }".to_string();
        }
        if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&action_id) {
            let (h_idx, f_slot) = decode_digivolve(action_id);
            let cid = game
                .player(actor)
                .hand
                .get(h_idx as usize)
                .map(|c| c.card_id(&game.card_data).to_string())
                .unwrap_or_else(|| "UNKNOWN".to_string());
            let from_slot = if f_slot == BREEDING_TARGET {
                "breeding".to_string()
            } else {
                format!("field.{f_slot}")
            };
            return format!(
                "{{ digivolve: {{ from: {from_slot}, using: {} }} }}",
                yaml_scalar(&cid)
            );
        }
    }

    let exp = explain_action(game, actor, action_id);
    match exp.kind {
        ActionKind::Hatch => "{ hatch: {} }".to_string(),
        ActionKind::Move => "{ move: { from: breeding } }".to_string(),
        ActionKind::Pass => "{ pass: {} }".to_string(),
        ActionKind::Play => {
            let h_idx = action_id.saturating_sub(PLAY_HAND_START) as usize;
            let cid = exp.card_id.unwrap_or_else(|| "UNKNOWN".to_string());
            let dup = game
                .player(actor)
                .hand
                .iter()
                .filter(|c| c.card_id(&game.card_data) == cid)
                .count();
            let from_str = if dup > 1 {
                format!("hand.{h_idx}")
            } else {
                "hand".to_string()
            };
            format!(
                "{{ play: {{ card: {}, from: {} }} }}",
                yaml_scalar(&cid),
                yaml_scalar(&from_str)
            )
        }
        ActionKind::Digivolve => {
            let (_h, f_slot) = decode_digivolve(action_id);
            let cid = exp.card_id.unwrap_or_else(|| "UNKNOWN".to_string());
            let from_str = if f_slot == BREEDING_TARGET {
                "breeding".to_string()
            } else {
                format!("field.{f_slot}")
            };
            format!(
                "{{ digivolve: {{ from: {}, using: {} }} }}",
                yaml_scalar(&from_str),
                yaml_scalar(&cid)
            )
        }
        ActionKind::Attack => {
            let (atk, tgt) = decode_attack(action_id);
            let target_str = if tgt == SECURITY_TARGET {
                "security".to_string()
            } else {
                format!("field.{tgt}")
            };
            format!("{{ attack: {{ attacker: field.{atk}, target: {target_str} }} }}")
        }
        ActionKind::FieldEffect => {
            let (perm, sub) = decode_field_effect(action_id);
            if sub == FIELD_EFFECT_SLOT_FOR_LINK {
                let cid = exp.card_id.unwrap_or_else(|| "UNKNOWN".to_string());
                format!(
                    "{{ link: {{ card: {}, from: field.{perm} }} }}",
                    yaml_scalar(&cid)
                )
            } else {
                let on_str = if perm == BREEDING_TARGET {
                    "breeding".to_string()
                } else {
                    format!("field.{perm}")
                };
                format!("{{ main: {{ on: {on_str} }} }}")
            }
        }
        ActionKind::HandEffect => {
            let h_idx = action_id.saturating_sub(HAND_EFFECT_START) as usize;
            let cid = exp.card_id.unwrap_or_else(|| "UNKNOWN".to_string());
            let dup = game
                .player(actor)
                .hand
                .iter()
                .filter(|c| c.card_id(&game.card_data) == cid)
                .count();
            let from_str = if dup > 1 {
                format!("hand.{h_idx}")
            } else {
                "hand".to_string()
            };
            if game.hand_effect_slot_is_link(actor, h_idx) {
                format!(
                    "{{ link: {{ card: {}, from: {} }} }}",
                    yaml_scalar(&cid),
                    yaml_scalar(&from_str)
                )
            } else {
                format!(
                    "{{ play: {{ card: {}, from: {} }} }}",
                    yaml_scalar(&cid),
                    yaml_scalar(&from_str)
                )
            }
        }
        _ => "{ pass: {} }".to_string(),
    }
}

fn symbolic_selection_mapping(
    game: &Game,
    sel: &crate::selection::PendingSelection,
    actor: u8,
    action_id: u16,
) -> String {
    if action_id == PASS {
        return "{ select: { decline: true } }".to_string();
    }
    if matches!(sel.kind, SelectionKind::Replacement) && action_id == REPLACEMENT_ACCEPT {
        return "{ select: { yes: true } }".to_string();
    }

    if let Some(ref cc) = sel.card_choice {
        let view = cc.view(game, actor);
        if let Some(row) = view.rows.iter().find(|r| r.action_id == action_id) {
            if let Some(ref cid) = row.card_id {
                return format!("{{ select: {{ cards: [{}] }} }}", yaml_scalar(cid));
            }
        }
    }

    let owner = game.player(sel.zone_owner.unwrap_or(sel.selecting_player));
    match &sel.kind {
        SelectionKind::OwnField if (100..114).contains(&action_id) => {
            format!("{{ select: {{ targets: [own.field.{}] }} }}", action_id - 100)
        }
        SelectionKind::OppField if (100..114).contains(&action_id) => {
            format!("{{ select: {{ targets: [opp.field.{}] }} }}", action_id - 100)
        }
        SelectionKind::AnyField if (100..130).contains(&action_id) => {
            let rel = action_id - 100;
            let p = (rel / TARGETS_PER_ATTACKER) as u8;
            let slot = rel % TARGETS_PER_ATTACKER;
            let side = if p == actor { "own" } else { "opp" };
            format!("{{ select: {{ targets: [{side}.field.{slot}] }} }}")
        }
        SelectionKind::Hand | SelectionKind::UnionZone { .. } if action_id < SEL_REVEAL_START => {
            if let Some(c) = owner.hand.get(action_id as usize) {
                return format!(
                    "{{ select: {{ cards: [{}] }} }}",
                    yaml_scalar(c.card_id(&game.card_data))
                );
            }
            format!("{{ select: {{ value: {action_id} }} }}")
        }
        SelectionKind::Trash | SelectionKind::UnionZone { .. }
            if action_id >= TRASH_EFFECT_START =>
        {
            let idx = (action_id - TRASH_EFFECT_START) as usize;
            if let Some(c) = owner.trash.get(idx) {
                return format!(
                    "{{ select: {{ cards: [{}] }} }}",
                    yaml_scalar(c.card_id(&game.card_data))
                );
            }
            format!("{{ select: {{ value: {idx} }} }}")
        }
        SelectionKind::Reveal | SelectionKind::RevealBucket { .. }
            if action_id >= SEL_REVEAL_START =>
        {
            let idx = (action_id - SEL_REVEAL_START) as usize;
            if let Some(c) = game.revealed_cards.get(idx) {
                return format!(
                    "{{ select: {{ cards: [{}] }} }}",
                    yaml_scalar(c.card_id(&game.card_data))
                );
            }
            format!("{{ select: {{ value: {idx} }} }}")
        }
        SelectionKind::Source if (SOURCE_SELECT_START..SOURCE_SELECT_END).contains(&action_id) => {
            let (f, s) = decode_source_select(action_id);
            if let Some(perm) = owner.battle_area.get(f as usize) {
                if let Some(c) = perm.card_sources.get(s as usize) {
                    return format!(
                        "{{ select: {{ cards: [{}] }} }}",
                        yaml_scalar(c.card_id(&game.card_data))
                    );
                }
            }
            format!("{{ select: {{ value: {action_id} }} }}")
        }
        SelectionKind::EffectChoice => {
            if let Some(ref entries) = sel.effect_choices {
                if entries.len() == 2 {
                    let l1 = entries[1].label.trim().to_ascii_lowercase();
                    let is_dec = l1 == "decline"
                        || l1 == "skip"
                        || l1 == "no"
                        || l1 == "pass"
                        || l1.starts_with("don't")
                        || l1.starts_with("do not");
                    if is_dec {
                        if entries[0].action_id == action_id {
                            return "{ select: { yes: true } }".to_string();
                        }
                        if entries[1].action_id == action_id {
                            return "{ select: { decline: true } }".to_string();
                        }
                    }
                }
                if let Some(entry) = entries.iter().find(|e| e.action_id == action_id) {
                    return format!(
                        "{{ select: {{ choice: {}, sim_only: true }} }}",
                        yaml_scalar(&entry.label)
                    );
                }
            }
            format!("{{ select: {{ value: {action_id} }} }}")
        }
        _ => {
            if let Some(cid) = explain_action(game, actor, action_id).card_id {
                format!("{{ select: {{ cards: [{}] }} }}", yaml_scalar(&cid))
            } else {
                format!("{{ select: {{ value: {action_id} }} }}")
            }
        }
    }
}
