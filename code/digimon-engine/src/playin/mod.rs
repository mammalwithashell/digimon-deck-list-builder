//! Guided Play-In Compiler & Symbolic DCGO YAML Exporter (`digimon_engine::playin`).
//!
//! Compiles a staged `studio-puzzle-v1` document (`setup.kind == "staged"`) into:
//! 1. Deterministic `stacked_decks` (`decks.1`, `decks.2`) inverted against `Game::new`'s
//!    `StdRng::seed_from_u64(seed)` deck/egg shuffles,
//! 2. A legal Turn-1 action prefix (`prefix_actions`, starting with `[0, 0]` Mulligan keeps)
//!    reaching a functionally equivalent (or `strict_zones` multiset-exact) board state,
//! 3. Remapped `target` and `avoid` lines (plus `expectations`) verified against `verify_puzzle`
//!    goal semantics, and
//! 4. Symbolic `dcgo-harness` YAML (`decks: { p0, p1 }` + `steps: [{ actor, do: ... }]`).

pub mod dcgo_yaml;
pub mod deck_stacker;
pub mod remap;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::action::build_action_mask;
use crate::action::space::{
    decode_attack, decode_digivolve, ATTACK_END, ATTACK_START, BREEDING_TARGET, CONCEDE_GAME,
    DIGIVOLVE_END, DIGIVOLVE_START, DNA_DIGIVOLVE_END, DNA_DIGIVOLVE_START, FIELD_EFFECT_END,
    FIELD_EFFECT_START, HAND_EFFECT_END, HAND_EFFECT_START, PASS, PLAY_HAND_END, PLAY_HAND_START,
    SECURITY_TARGET, TRASH_EFFECT_END, TRASH_EFFECT_START,
};

const MULLIGAN_KEEP: u16 = 0;
use crate::card_data::CardData;
use crate::card_registry::CardRegistry;
use crate::enums::{CardKind, GamePhase};
use crate::game::Game;
use crate::inference::RefutationTrigger;
use crate::rules::Rules;
use crate::search::decision_player;

pub use dcgo_yaml::export_dcgo_yaml;
pub use deck_stacker::{
    distance_score, extract_goal, invert_game_new_shuffle, is_equivalent, solve_stacked_decks,
    PlayInGoal, SeatGoal, StackedSeatPlan, TargetPermanent,
};
pub use remap::{
    evaluate_goal_assertion, extract_puzzle_lines, line_satisfies_puzzle_goal,
    remap_and_verify_puzzle, remap_line_actions, remap_single_action, RemappedLine,
};

#[derive(Debug, Clone)]
pub struct PlayInConfig {
    pub beam_width: usize,
    pub max_turns: u16,
    pub strict_zones: bool,
    pub rest_deck_p0: String,
    pub rest_deck_p1: String,
}

impl Default for PlayInConfig {
    fn default() -> Self {
        Self {
            beam_width: 32,
            max_turns: 5,
            strict_zones: false,
            rest_deck_p0: "custom".to_string(),
            rest_deck_p1: "custom".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayInResult {
    pub ok: bool,
    pub matched: bool,
    pub strict_zones: bool,
    pub seed: u64,
    pub prefix_actions: Vec<u16>,
    pub stacked_decks: BTreeMap<String, Vec<String>>,
    pub remapped_lines: Vec<RemappedLine>,
    pub remapped_expectations: Vec<Value>,
    pub dcgo_yaml: Option<String>,
    pub distance_score: f64,
    pub error: Option<String>,
}

fn parse_string_vec(val: Option<&Value>) -> Vec<String> {
    val.and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn ensure_bootable_deck(raw: &[String], card_data: &HashMap<String, CardData>) -> Vec<String> {
    let mut out: Vec<String> = raw
        .iter()
        .filter(|id| card_data.contains_key(*id))
        .cloned()
        .collect();
    let main_count = out
        .iter()
        .filter(|id| {
            card_data
                .get(*id)
                .is_some_and(|cd| cd.card_kind != CardKind::DigiEgg)
        })
        .count();
    let filler = if card_data.contains_key("ST1-03") {
        "ST1-03"
    } else {
        card_data
            .values()
            .find(|cd| cd.card_kind == CardKind::Digimon)
            .map(|cd| cd.card_id.as_str())
            .unwrap_or("ST1-03")
    };
    for _ in main_count..15 {
        out.push(filler.to_string());
    }
    out
}

/// Reconstruct the staged reference `Game` from a `studio-puzzle-v1` document.
pub fn build_staged_reference_game(
    doc: &Value,
    card_data: &HashMap<String, CardData>,
) -> Result<(Game, u8, [Vec<String>; 2]), String> {
    let schema = doc.get("schema").and_then(Value::as_str).unwrap_or("");
    if schema != "studio-puzzle-v1" {
        return Err(format!(
            "compile_play_in expects schema == 'studio-puzzle-v1', got {schema:?}"
        ));
    }

    let decks_obj = doc.get("decks").and_then(Value::as_object);
    let raw1 = parse_string_vec(decks_obj.and_then(|o| o.get("1")));
    let raw2 = parse_string_vec(decks_obj.and_then(|o| o.get("2")));

    let boot1 = ensure_bootable_deck(&raw1, card_data);
    let boot2 = ensure_bootable_deck(&raw2, card_data);

    let orig_seed = doc.get("seed").and_then(Value::as_u64).unwrap_or(0);
    let seat_1based = doc.get("seat").and_then(Value::as_u64).unwrap_or(1);
    let solver_seat = (seat_1based.max(1) - 1).min(1) as u8;

    let mut game = Game::new(&[boot1, boot2], card_data, Rules::standard(), Some(orig_seed))?;
    let setup = doc.get("setup").unwrap_or(&Value::Null);
    let kind = setup
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("staged");
    let prefix_actions = remap::parse_u16_vec(setup.get("prefix_actions"), "setup.prefix_actions")?;

    if kind == "staged" || setup.get("zones").is_some() || setup.get("state").is_some() {
        while let Some(p) = game.mulligan_current_player() {
            let _ = game.accept_mulligan(p, true);
        }
        let scenario = json!({
            "state": setup.get("state").cloned().unwrap_or_else(|| json!({
                "memory": 0,
                "phase": "Main",
                "turn": 1,
                "first_player": seat_1based.max(1),
            })),
            "zones": setup.get("zones").cloned().unwrap_or_else(|| json!({})),
        });
        game.apply_scenario(&scenario)?;
        game.tick_declarative_effects();
    } else if prefix_actions.is_empty() {
        while let Some(p) = game.mulligan_current_player() {
            let _ = game.accept_mulligan(p, true);
        }
    } else if game.turn_count == 0 {
        game.turn_count = 1;
    }

    for (idx, &aid) in prefix_actions.iter().enumerate() {
        if game.game_over {
            break;
        }
        let pid = decision_player(&game);
        let mask = build_action_mask(&game, pid);
        if (aid as usize) >= mask.len() || mask[aid as usize] <= 0.5 {
            return Err(format!(
                "reference setup.prefix_actions[{idx}] = {aid} is illegal in phase {:?}",
                game.current_phase
            ));
        }
        game.decode_action(aid, pid);
    }

    Ok((game, solver_seat, [raw1, raw2]))
}

#[derive(Clone)]
struct BeamNode {
    game: Game,
    actions: Vec<u16>,
    score: f64,
}

fn state_fingerprint(game: &Game) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    game.turn_count.hash(&mut h);
    game.turn_player().hash(&mut h);
    decision_player(game).hash(&mut h);
    (game.current_phase as u8).hash(&mut h);
    game.memory.hash(&mut h);
    game.pending_selection.is_some().hash(&mut h);
    if let Some(ref sel) = game.pending_selection {
        sel.selecting_player.hash(&mut h);
        sel.valid_action_ids.hash(&mut h);
    }
    for p in 0..2 {
        let pl = &game.players[p];
        pl.security.len().hash(&mut h);
        pl.deck.len().hash(&mut h);
        pl.digitama_deck.len().hash(&mut h);
        if let Some(ref br) = pl.breeding_area {
            true.hash(&mut h);
            deck_stacker::perm_stack_ids(game, br).hash(&mut h);
        } else {
            false.hash(&mut h);
        }
        pl.battle_area.len().hash(&mut h);
        for perm in &pl.battle_area {
            deck_stacker::perm_stack_ids(game, perm).hash(&mut h);
            deck_stacker::perm_linked_ids(game, perm).hash(&mut h);
            perm.is_suspended.hash(&mut h);
            (game.turn_count > perm.turn_played).hash(&mut h);
        }
        let mut hand_ids: Vec<&str> = pl
            .hand
            .iter()
            .map(|c| c.card_id(&game.card_data))
            .collect();
        hand_ids.sort_unstable();
        hand_ids.hash(&mut h);

        let mut trash_ids: Vec<&str> = pl
            .trash
            .iter()
            .map(|c| c.card_id(&game.card_data))
            .collect();
        trash_ids.sort_unstable();
        trash_ids.hash(&mut h);
    }
    h.finish()
}

fn has_delay_keyword_or_text(cd: &CardData) -> bool {
    cd.card_kind == CardKind::Option
        && (cd.effect_text.contains("Delay")
            || cd.effect_text.contains("＜Delay＞")
            || cd.effect_text.contains("<Delay>"))
}

fn is_strict_prefix_of(prefix: &[String], full: &[String]) -> bool {
    prefix.len() <= full.len() && full[..prefix.len()] == *prefix
}

/// Focused action generator for play-in beam expansion.
fn candidate_actions_for_node(
    game: &Game,
    goal: &PlayInGoal,
    strict_zones: bool,
) -> Vec<u16> {
    let mover_u8 = decision_player(game);
    let mask = build_action_mask(game, mover_u8);
    let legal: Vec<u16> = mask
        .iter()
        .enumerate()
        .filter_map(|(i, &m)| {
            let aid = i as u16;
            (m > 0.5 && aid != CONCEDE_GAME).then_some(aid)
        })
        .collect();

    if legal.len() <= 1 {
        return legal;
    }
    if game.current_phase == GamePhase::Mulligan {
        if legal.contains(&MULLIGAN_KEEP) {
            return vec![MULLIGAN_KEEP];
        }
        return vec![legal[0]];
    }

    if game.pending_selection.is_some()
        || matches!(
            game.current_phase,
            GamePhase::BlockTiming | GamePhase::CounterTiming | GamePhase::AllianceTiming
        )
    {
        let mut picks = Vec::new();
        if legal.contains(&PASS) {
            picks.push(PASS);
        }
        for &aid in &legal {
            if aid != PASS && picks.len() < 8 {
                picks.push(aid);
            }
        }
        return picks;
    }

    if game.current_phase == GamePhase::Breeding {
        return legal;
    }

    let me = mover_u8 as usize;
    let opp = 1 - me;
    let sg = &goal.seats[me];
    let opp_sg = &goal.seats[opp];
    let pl = &game.players[me];
    let opp_sec_live = game.players[opp].security.len();

    let mut chosen: Vec<u16> = Vec::new();
    let mut extra_filler_plays: Vec<u16> = Vec::new();
    let mut seen_play_sig: HashSet<(String, u16)> = HashSet::new();

    // Count which target field stacks are already completely matched on `battle_area`.
    let mut target_matched = vec![false; sg.field_stacks.len()];
    for perm in &pl.battle_area {
        let p_ids = deck_stacker::perm_stack_ids(game, perm);
        if let Some(ti) = (0..sg.field_stacks.len())
            .find(|&ti| !target_matched[ti] && sg.field_stacks[ti].stack_ids == p_ids)
        {
            target_matched[ti] = true;
        }
    }

    for &aid in &legal {
        if aid == PASS {
            chosen.push(aid);
            continue;
        }

        if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&aid) {
            let (h_idx, slot) = decode_digivolve(aid);
            let Some(cs) = pl.hand.get(h_idx as usize) else {
                continue;
            };
            let cid = cs.card_id(&game.card_data).to_string();
            if !seen_play_sig.insert((cid.clone(), 1000 + slot)) {
                continue;
            }
            if slot == BREEDING_TARGET {
                let mut b_next = pl
                    .breeding_area
                    .as_ref()
                    .map(|perm| deck_stacker::perm_stack_ids(game, perm))
                    .unwrap_or_default();
                b_next.push(cid);
                let matches_breeding_goal = sg
                    .breeding_stack
                    .as_ref()
                    .is_some_and(|wb| is_strict_prefix_of(&b_next, wb));
                let matches_field_goal = (0..sg.field_stacks.len()).any(|ti| {
                    !target_matched[ti] && is_strict_prefix_of(&b_next, &sg.field_stacks[ti].stack_ids)
                });
                if matches_breeding_goal || matches_field_goal {
                    chosen.push(aid);
                }
            } else if let Some(perm) = pl.battle_area.get(slot as usize) {
                let mut f_next = deck_stacker::perm_stack_ids(game, perm);
                f_next.push(cid);
                let matches_field_goal = (0..sg.field_stacks.len()).any(|ti| {
                    !target_matched[ti] && is_strict_prefix_of(&f_next, &sg.field_stacks[ti].stack_ids)
                });
                if matches_field_goal {
                    chosen.push(aid);
                }
            }
            continue;
        }

        if (PLAY_HAND_START..PLAY_HAND_END).contains(&aid) {
            let h_idx = (aid - PLAY_HAND_START) as usize;
            let Some(cs) = pl.hand.get(h_idx) else {
                continue;
            };
            let cd = &game.card_data[cs.data_index];
            let cid = cd.card_id.clone();
            if !seen_play_sig.insert((cid.clone(), 0)) {
                continue;
            }

            let starts_target_stack = (0..sg.field_stacks.len()).any(|ti| {
                !target_matched[ti]
                    && sg.field_stacks[ti]
                        .stack_ids
                        .first()
                        .is_some_and(|bot| bot == &cid)
            });
            let wanted_in_trash = sg.trash_multiset.contains_key(&cid);
            let is_non_delay_option =
                cd.card_kind == CardKind::Option && !has_delay_keyword_or_text(cd);

            if starts_target_stack {
                chosen.push(aid);
            } else if is_non_delay_option && (wanted_in_trash || !strict_zones) {
                chosen.push(aid);
            } else if opp_sec_live > opp_sg.security_count
                && pl.battle_area.is_empty()
                && cd.card_kind == CardKind::Digimon
                && cd.play_cost <= 5
            {
                extra_filler_plays.push(aid);
            }
            continue;
        }

        if (ATTACK_START..ATTACK_END).contains(&aid) {
            let (atk, tgt) = decode_attack(aid);
            if tgt == SECURITY_TARGET {
                let needs_chip = opp_sec_live > opp_sg.security_count;
                let needs_suspend = pl.battle_area.get(atk as usize).is_some_and(|perm| {
                    let p_ids = deck_stacker::perm_stack_ids(game, perm);
                    !perm.is_suspended
                        && sg
                            .field_stacks
                            .iter()
                            .any(|tf| tf.stack_ids == p_ids && tf.is_suspended)
                });
                if needs_chip || needs_suspend {
                    chosen.push(aid);
                }
            } else {
                // Attacking an opponent Digimon can clear a sacrificial Digimon or suspend attacker.
                chosen.push(aid);
            }
            continue;
        }

        if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&aid)
            || (HAND_EFFECT_START..HAND_EFFECT_END).contains(&aid)
            || (TRASH_EFFECT_START..TRASH_EFFECT_END).contains(&aid)
            || (DNA_DIGIVOLVE_START..DNA_DIGIVOLVE_END).contains(&aid)
        {
            chosen.push(aid);
        }
    }

    chosen.extend(extra_filler_plays.into_iter().take(2));
    if chosen.is_empty() {
        chosen.extend(legal.into_iter().take(4));
    }
    chosen
}

fn run_single_seed_beam_search(
    doc: &Value,
    puzzle_id: &str,
    target_game: &Game,
    goal: &PlayInGoal,
    raw_decks: &[Vec<String>; 2],
    card_data: &HashMap<String, CardData>,
    registry: &CardRegistry,
    triggers: &[RefutationTrigger],
    config: &PlayInConfig,
    seed: u64,
) -> Result<PlayInResult, String> {
    let (plans, pre_decks) = solve_stacked_decks(raw_decks, target_game, goal, card_data, seed)?;
    let mut stacked_map = BTreeMap::new();
    stacked_map.insert("1".to_string(), pre_decks[0].clone());
    stacked_map.insert("2".to_string(), pre_decks[1].clone());

    let initial_game = Game::new(&pre_decks, card_data, Rules::standard(), Some(seed))?;
    let mut root_game = initial_game.clone();
    let mut root_actions = Vec::new();
    for _ in 0..4 {
        if root_game.current_phase != GamePhase::Mulligan || root_game.game_over {
            break;
        }
        let pid = decision_player(&root_game);
        root_game.decode_action(MULLIGAN_KEEP, pid);
        root_actions.push(MULLIGAN_KEEP);
    }

    let root_score = distance_score(&root_game, goal, config.strict_zones);
    let root_node = BeamNode {
        game: root_game,
        actions: root_actions,
        score: root_score,
    };

    let mut last_verify_err: Option<String> = None;

    if is_equivalent(&root_node.game, goal, config.strict_zones) {
        match remap_and_verify_puzzle(
            target_game,
            &root_node.game,
            goal.solver_seat,
            doc,
            registry,
            triggers,
        ) {
            Ok((remapped_lines, remapped_expectations)) => {
                let target_line = remapped_lines
                    .iter()
                    .find(|l| l.role == "target")
                    .map(|l| l.actions.as_slice())
                    .unwrap_or(&[]);
                let yaml = export_dcgo_yaml(
                    puzzle_id,
                    seed,
                    &plans,
                    &initial_game,
                    &root_node.actions,
                    target_line,
                    goal.solver_seat,
                    triggers,
                    &config.rest_deck_p0,
                    &config.rest_deck_p1,
                )?;
                return Ok(PlayInResult {
                    ok: true,
                    matched: true,
                    strict_zones: config.strict_zones,
                    seed,
                    prefix_actions: root_node.actions,
                    stacked_decks: stacked_map,
                    remapped_lines,
                    remapped_expectations,
                    dcgo_yaml: Some(yaml),
                    distance_score: 0.0,
                    error: None,
                });
            }
            Err(e) => {
                last_verify_err = Some(e);
            }
        }
    }

    let mut best_node = root_node.clone();
    let mut beam = vec![root_node];
    let beam_cap = config.beam_width.max(1);
    let max_turns = config.max_turns.max(1);
    let max_depth = (max_turns as usize) * 8 + 4;
    let mut global_visited: HashSet<u64> = HashSet::new();
    let mut verify_attempts = 0usize;

    for _depth in 0..max_depth {
        if beam.is_empty() {
            break;
        }
        let mut next_candidates: Vec<BeamNode> = Vec::new();
        let mut step_seen: HashSet<u64> = HashSet::new();

        for node in &beam {
            if node.game.game_over || node.game.turn_count > max_turns {
                continue;
            }
            let cands = candidate_actions_for_node(&node.game, goal, config.strict_zones);
            let mover = decision_player(&node.game);
            for aid in cands {
                let mut child_game = node.game.clone();
                child_game.decode_action(aid, mover);
                if child_game.game_over || child_game.turn_count > max_turns {
                    continue;
                }
                let mut child_actions = Vec::with_capacity(node.actions.len() + 1);
                child_actions.extend_from_slice(&node.actions);
                child_actions.push(aid);

                if verify_attempts < 8 && is_equivalent(&child_game, goal, config.strict_zones) {
                    verify_attempts += 1;
                    match remap_and_verify_puzzle(
                        target_game,
                        &child_game,
                        goal.solver_seat,
                        doc,
                        registry,
                        triggers,
                    ) {
                        Ok((remapped_lines, remapped_expectations)) => {
                            let target_line = remapped_lines
                                .iter()
                                .find(|l| l.role == "target")
                                .map(|l| l.actions.as_slice())
                                .unwrap_or(&[]);
                            let yaml = export_dcgo_yaml(
                                puzzle_id,
                                seed,
                                &plans,
                                &initial_game,
                                &child_actions,
                                target_line,
                                goal.solver_seat,
                                triggers,
                                &config.rest_deck_p0,
                                &config.rest_deck_p1,
                            )?;
                            return Ok(PlayInResult {
                                ok: true,
                                matched: true,
                                strict_zones: config.strict_zones,
                                seed,
                                prefix_actions: child_actions,
                                stacked_decks: stacked_map,
                                remapped_lines,
                                remapped_expectations,
                                dcgo_yaml: Some(yaml),
                                distance_score: 0.0,
                                error: None,
                            });
                        }
                        Err(e) => {
                            last_verify_err = Some(e);
                        }
                    }
                }

                let fp = state_fingerprint(&child_game);
                if !step_seen.insert(fp) || !global_visited.insert(fp) {
                    continue;
                }
                let sc = distance_score(&child_game, goal, config.strict_zones);
                let child_node = BeamNode {
                    game: child_game,
                    actions: child_actions,
                    score: sc,
                };
                if sc < best_node.score {
                    best_node = child_node.clone();
                }
                next_candidates.push(child_node);
            }
        }

        next_candidates.sort_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.actions.len().cmp(&b.actions.len()))
        });
        next_candidates.truncate(beam_cap);
        beam = next_candidates;
    }

    let orig_lines = extract_puzzle_lines(doc).unwrap_or_default();
    let orig_exps = doc
        .get("expectations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let err_msg = last_verify_err.unwrap_or_else(|| {
        format!(
            "beam search (width={}, max_turns={}, strict_zones={}) did not reach equivalent board; closest distance={:.3}",
            config.beam_width, config.max_turns, config.strict_zones, best_node.score
        )
    });

    Ok(PlayInResult {
        ok: true,
        matched: false,
        strict_zones: config.strict_zones,
        seed,
        prefix_actions: best_node.actions,
        stacked_decks: stacked_map,
        remapped_lines: orig_lines,
        remapped_expectations: orig_exps,
        dcgo_yaml: None,
        distance_score: best_node.score,
        error: Some(err_msg),
    })
}

/// Compile a `studio-puzzle-v1` staged puzzle into a Turn-1 legal `replay_prefix`
/// puzzle + symbolic DCGO YAML.
pub fn compile_play_in(
    doc: &Value,
    card_data: &HashMap<String, CardData>,
    config: &PlayInConfig,
) -> Result<PlayInResult, String> {
    let puzzle_id = doc
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("studio-puzzle");
    let (target_game, solver_seat, raw_decks) = build_staged_reference_game(doc, card_data)?;
    let goal = extract_goal(&target_game, solver_seat);
    let registry = CardRegistry::from_cards(card_data);
    let triggers: Vec<RefutationTrigger> = doc
        .get("refutation_triggers")
        .filter(|v| !v.is_null())
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()
        .map_err(|e| format!("invalid refutation_triggers: {e}"))?
        .unwrap_or_default();

    // Try `seed = 0` (`first_player = 0`, matching `dcgo-harness` `p0` first) first.
    let first_try = run_single_seed_beam_search(
        doc,
        puzzle_id,
        &target_game,
        &goal,
        &raw_decks,
        card_data,
        &registry,
        &triggers,
        config,
        0,
    )?;
    if first_try.matched || goal.turn_player == 0 {
        return Ok(first_try);
    }

    // Fallback: try `seed = 1` (`first_player = 1`) when `goal.turn_player == 1`.
    let second_try = run_single_seed_beam_search(
        doc,
        puzzle_id,
        &target_game,
        &goal,
        &raw_decks,
        card_data,
        &registry,
        &triggers,
        config,
        1,
    )?;
    if second_try.matched || second_try.distance_score < first_try.distance_score {
        Ok(second_try)
    } else {
        Ok(first_try)
    }
}

/// Upgrade a staged `studio-puzzle-v1` JSON document in-place to `setup.kind = "replay_prefix"`
/// using a matched [`PlayInResult`].
pub fn upgrade_puzzle_to_replay_prefix(doc: &Value, res: &PlayInResult) -> Value {
    let mut out = doc.clone();
    out["seed"] = json!(res.seed);
    out["decks"] = json!(res.stacked_decks);
    out["setup"] = json!({
        "kind": "replay_prefix",
        "prefix_actions": res.prefix_actions,
    });
    if !res.remapped_lines.is_empty() {
        out["lines"] = json!(res.remapped_lines);
    }
    if !res.remapped_expectations.is_empty() {
        out["expectations"] = json!(res.remapped_expectations);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn load_test_cards() -> HashMap<String, CardData> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/cards.json");
        let raw = std::fs::read_to_string(path).expect("read data/cards.json");
        CardData::load_from_str(&raw).expect("parse data/cards.json")
    }

    #[test]
    fn invert_game_new_shuffle_preserves_exact_draw_and_egg_order() {
        let cards = load_test_cards();
        let post_main = [
            vec![
                "ST1-02".to_string(),
                "ST1-03".to_string(),
                "ST1-04".to_string(),
                "ST1-05".to_string(),
                "ST1-06".to_string(),
                "ST1-07".to_string(),
                "ST1-08".to_string(),
                "ST1-09".to_string(),
                "ST1-10".to_string(),
                "ST1-11".to_string(),
                "ST1-12".to_string(),
                "ST1-13".to_string(),
            ],
            vec!["ST1-03".to_string(); 12],
        ];
        let post_egg = [
            vec!["ST1-01".to_string(), "BT1-001".to_string()],
            vec!["ST1-01".to_string()],
        ];
        let pre = invert_game_new_shuffle(&post_main, &post_egg, 0);
        let mut g = Game::new(&pre, &cards, Rules::standard(), Some(0)).unwrap();

        // Starting hand popped the last 5 elements of `post_main[0]` in reverse!
        let hand_ids: Vec<String> = g.players[0]
            .hand
            .iter()
            .map(|c| c.card_id(&g.card_data).to_string())
            .collect();
        assert_eq!(
            hand_ids,
            vec!["ST1-13", "ST1-12", "ST1-11", "ST1-10", "ST1-09"]
        );

        while let Some(p) = g.mulligan_current_player() {
            g.accept_mulligan(p, true).unwrap();
        }

        let sec_ids: Vec<String> = g.players[0]
            .security
            .iter()
            .map(|c| c.card_id(&g.card_data).to_string())
            .collect();
        assert_eq!(
            sec_ids,
            vec!["ST1-08", "ST1-07", "ST1-06", "ST1-05", "ST1-04"]
        );
        // First hatch pops the tail of `post_egg[0]` ("BT1-001").
        assert_eq!(
            g.players[0]
                .digitama_deck
                .last()
                .unwrap()
                .card_id(&g.card_data),
            "BT1-001"
        );
    }

    #[test]
    fn compile_play_in_solves_multi_turn_staged_board_remaps_lines_and_exports_dcgo_yaml() {
        std::thread::Builder::new()
            .stack_size(64 * 1024 * 1024)
            .spawn(run_compile_play_in_unit_test)
            .unwrap()
            .join()
            .unwrap();
    }

    fn run_compile_play_in_unit_test() {
        let cards = load_test_cards();
        let mut d1 = vec!["ST1-01".to_string(); 4];
        d1.extend(std::iter::repeat_n("ST1-03".to_string(), 20));
        d1.extend(std::iter::repeat_n("ST1-05".to_string(), 15));
        d1.extend(std::iter::repeat_n("BT2-091".to_string(), 15));

        let mut d2 = vec!["ST1-01".to_string(); 4];
        d2.extend(std::iter::repeat_n("ST1-02".to_string(), 25));
        d2.extend(std::iter::repeat_n("ST1-03".to_string(), 25));

        // Staged puzzle:
        // - Seat 1 has an unsummoning-sick `[ST1-01, ST1-03, ST1-05]` stack on field
        //   (hatched + digivolved in breeding on Turn 1, moved out on Turn 3!),
        //   and holds `["BT2-091"]` (Volcanic Flare, cost 3) in hand at `3` memory.
        // - Seat 2 has `["ST1-02"]` (Biyomon, play cost 2, 3000 DP) on field and 5 security.
        // - Target line: swing security (`114`, 5 -> 4) then Volcanic Flare (`0`, target `100`),
        //   keeping turn at `0` memory with opponent field cleared!
        let staged_doc = json!({
            "schema": "studio-puzzle-v1",
            "id": "playin-unit-volcanic-flare-clear",
            "title": "Play-in unit test: raise Birdramon and Volcanic Flare threat",
            "seat": 1,
            "seed": 0,
            "decks": { "1": d1, "2": d2 },
            "setup": {
                "kind": "staged",
                "state": {
                    "memory": 3,
                    "phase": "Main",
                    "turn": 3,
                    "first_player": 1
                },
                "zones": {
                    "1": {
                        "hand": ["BT2-091"],
                        "security": ["ST1-03", "ST1-03", "ST1-03", "ST1-03", "ST1-03"],
                        "trash": [],
                        "field": [
                            { "stack": ["ST1-01", "ST1-03", "ST1-05"], "is_suspended": false, "turn_played": 1 }
                        ],
                        "breeding": null
                    },
                    "2": {
                        "hand": ["ST1-03"],
                        "security": ["ST1-03", "ST1-03", "ST1-03", "ST1-03", "ST1-03"],
                        "trash": [],
                        "field": [
                            { "stack": ["ST1-02"], "is_suspended": false, "turn_played": 2 }
                        ],
                        "breeding": null
                    }
                }
            },
            "rl_goal": {
                "win_assertions": [
                    { "kind": "zone_count", "player": 2, "zone": "field", "value": 0 },
                    { "kind": "zone_count", "player": 2, "zone": "security", "value": 4 }
                ]
            },
            "lines": [
                {
                    "id": "line-target",
                    "name": "Swing security with Birdramon then Gaia Force rookie",
                    "role": "target",
                    "actions": [114, 0, 100]
                },
                {
                    "id": "line-avoid",
                    "name": "Pass without clearing board",
                    "role": "avoid",
                    "actions": [62]
                }
            ],
            "expectations": [
                { "kind": "choose", "plan": [114] },
                { "kind": "avoid", "plan": [62] }
            ]
        });

        let cfg = PlayInConfig::default();
        let res = compile_play_in(&staged_doc, &cards, &cfg).expect("compile_play_in should run");
        assert!(res.ok);
        assert!(
            res.matched,
            "expected play-in match, got error={:?}, dist={}",
            res.error, res.distance_score
        );
        assert_eq!(res.distance_score, 0.0);
        assert!(res.prefix_actions.len() >= 4);

        // With `strict_zones = true`, natural Turn 1-3 draws leave >1 card in hand,
        // so `strict_zones` must reject the extra filler cards cleanly (`matched == false`).
        let strict_cfg = PlayInConfig {
            beam_width: 16,
            max_turns: 3,
            strict_zones: true,
            ..Default::default()
        };
        let strict_res = compile_play_in(&staged_doc, &cards, &strict_cfg).unwrap();
        assert!(strict_res.ok);
        assert!(!strict_res.matched);
        assert!(strict_res.distance_score > 0.0);

        // Replay from Turn 1 using `stacked_decks` and verify functional equivalence!
        let sd1 = res.stacked_decks.get("1").unwrap().clone();
        let sd2 = res.stacked_decks.get("2").unwrap().clone();
        let mut live = Game::new(&[sd1, sd2], &cards, Rules::standard(), Some(res.seed)).unwrap();
        for &aid in &res.prefix_actions {
            let pid = decision_player(&live);
            let mask = build_action_mask(&live, pid);
            assert!(mask[aid as usize] > 0.5, "illegal prefix action {aid}");
            live.decode_action(aid, pid);
        }
        assert_eq!(live.current_phase, GamePhase::Main);
        assert_eq!(decision_player(&live), 0);
        assert_eq!(live.memory, 3);
        assert_eq!(live.players[0].battle_area.len(), 1);
        assert_eq!(
            deck_stacker::perm_stack_ids(&live, &live.players[0].battle_area[0]),
            vec!["ST1-01", "ST1-03", "ST1-05"]
        );
        // Inert filler draws shifted Gaia Force's hand index from 0 to a higher index —
        // verify `remapped_lines[0]` updated the play-from-hand index and satisfies `rl_goal`!
        let target_remapped = &res.remapped_lines[0];
        assert_eq!(target_remapped.role, "target");
        for &aid in &target_remapped.actions {
            let pid = decision_player(&live);
            let mask = build_action_mask(&live, pid);
            assert!(mask[aid as usize] > 0.5, "remapped action {aid} illegal");
            live.decode_action(aid, pid);
        }
        assert!(line_satisfies_puzzle_goal(&live, 0, &staged_doc));

        // Verify symbolic DCGO YAML output structure.
        let yaml = res.dcgo_yaml.as_deref().expect("dcgo_yaml must be present");
        assert!(yaml.contains("decks:\n  p0:\n    stack:\n"));
        assert!(yaml.contains("steps:\n"));
        assert!(yaml.contains("hatch:"));
        assert!(yaml.contains("digivolve:"));
        assert!(yaml.contains("move:"));
        assert!(yaml.contains("attack:"));
        assert!(yaml.contains("play:"));
    }
}
