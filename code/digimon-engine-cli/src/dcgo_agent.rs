//! `dcgo-agent` subcommand: lets one of our trained models play a seat inside the installed DCGO client.
//!
//! The MelonLoader mod (`code/dcgo-melon-recorder`) writes its raw event log while a game runs. When it is the
//! model's seat's turn to decide, the mod drops a request file into a shared directory; this process
//!   1. converts the raw log so far into a DCGO recording (`convert_raw.py --allow-truncated`),
//!   2. replays it through the engine (`ReplaySession::from_dcgo`, the same path `dcgo-replay` verifies),
//!   3. if the engine agrees it is the model seat's decision, builds the observation + mask and runs the policy
//!      (argmax over legal actions; the value is reported, no search),
//!   4. writes a response describing the chosen action in DCGO terms (hand card / target / attacker), which the
//!      mod performs through the same setters a human click reaches.
//! A replay divergence is reported as an error instead of an action: acting on a state the engine does not share
//! with DCGO would be a guess.
//!
//! **Script mode** (`--script <native recording>`): instead of the model, the answer is the action the seat took in a
//! game our engine played (`sim-record`). DCGO's current state is found in that recording by a state fingerprint
//! (turn, phase, memory, every zone), so the answer is the recorded action for exactly this state, and a state the
//! recording never reached is reported as `diverged` rather than guessed. Run one client per seat over PvP with the
//! same plan (`make_playback_plan.py`) so both decks start in the recorded order.
//!
//! Protocol (files in `--dir`): `request-<id>.json` = `{"id": "...", "raw": "<raw jsonl path>", "seat": 0|1}`;
//! the response is written atomically as `response-<id>.json` and the request file is removed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::*;
use digimon_engine::card_data::CardData;
use digimon_engine::inference::BatchedPolicyValueEvaluator;
use digimon_engine::observation::build_observation_tensor;
use digimon_engine::runners::replay::ReplaySession;
use serde_json::{json, Value};

#[allow(clippy::too_many_arguments)]
pub fn run(
    dir: PathBuf,
    model_path: PathBuf,
    converter: PathBuf,
    python: String,
    tensor_profile: Option<String>,
    once: bool,
    script_path: Option<PathBuf>,
    card_data: HashMap<String, CardData>,
) -> ExitCode {
    let mut script = match script_path {
        Some(path) => match Script::load(&path, &card_data) {
            Ok(s) => {
                eprintln!("dcgo-agent: script {} ({} recorded decisions)", path.display(), s.actions.len());
                Some(s)
            }
            Err(e) => {
                eprintln!("error loading script {}: {e}", path.display());
                return ExitCode::from(2);
            }
        },
        None => None,
    };
    let profile = match crate::winprob::resolve_profile(&model_path, tensor_profile.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error resolving tensor profile: {e}");
            return ExitCode::from(2);
        }
    };
    let mut evaluator = match BatchedPolicyValueEvaluator::load_for_profile(&model_path, profile) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error loading model {}: {}", model_path.display(), e);
            return ExitCode::from(2);
        }
    };
    let registry = digimon_engine::card_registry::CardRegistry::from_cards(&card_data);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("cannot create {}: {e}", dir.display());
        return ExitCode::from(2);
    }
    eprintln!(
        "dcgo-agent: model {} (profile {:?}), watching {}",
        model_path.display(),
        profile,
        dir.display()
    );
    loop {
        let mut requests: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.starts_with("request-") && n.ends_with(".json"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        requests.sort();
        for req in requests {
            let response = match handle(&req, &dir, &converter, &python, &card_data, &registry, profile, &mut evaluator, script.as_mut()) {
                Ok(v) => v,
                Err(e) => json!({"status": "error", "message": e}),
            };
            let id = response
                .get("id")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    req.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("request-unknown")
                        .trim_start_matches("request-")
                        .to_string()
                });
            let out = dir.join(format!("response-{id}.json"));
            let tmp = dir.join(format!("response-{id}.json.tmp"));
            let _ = std::fs::write(&tmp, serde_json::to_vec_pretty(&response).unwrap_or_default());
            let _ = std::fs::rename(&tmp, &out);
            let _ = std::fs::remove_file(&req);
            eprintln!("dcgo-agent: {} -> {}", req.display(), response);
        }
        if once {
            return ExitCode::SUCCESS;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

#[allow(clippy::too_many_arguments)]
fn handle(
    req: &Path,
    dir: &Path,
    converter: &Path,
    python: &str,
    card_data: &HashMap<String, CardData>,
    registry: &digimon_engine::card_registry::CardRegistry,
    profile: digimon_engine::observation::ObservationProfileId,
    evaluator: &mut BatchedPolicyValueEvaluator,
    script: Option<&mut Script>,
) -> Result<Value, String> {
    let text = std::fs::read_to_string(req).map_err(|e| format!("reading request: {e}"))?;
    let request: Value = serde_json::from_str(&text).map_err(|e| format!("bad request json: {e}"))?;
    let id = request["id"].as_str().unwrap_or("0").to_string();
    let raw = request["raw"].as_str().ok_or("request has no raw path")?;
    let seat = request["seat"].as_u64().ok_or("request has no seat")? as u8;

    // 1. raw log -> DCGO recording (truncated is fine: the game is still running)
    let converted = dir.join(format!("converted-{id}.jsonl"));
    let status = std::process::Command::new(python)
        .arg(converter)
        .arg(raw)
        .arg(&converted)
        .arg("--allow-truncated")
        .output()
        .map_err(|e| format!("running converter: {e}"))?;
    if !status.status.success() {
        return Err(format!(
            "converter failed: {}",
            String::from_utf8_lossy(&status.stderr).lines().last().unwrap_or("")
        ));
    }
    let rec_text = std::fs::read_to_string(&converted).map_err(|e| format!("reading converted: {e}"))?;
    let _ = std::fs::remove_file(&converted);
    // A live game has no `game_end` yet; the parser requires one. The agent only replays (it never checks a
    // winner), so a placeholder end row is safe here.
    let rec_text = if rec_text.lines().any(|l| l.contains("\"type\":\"game_end\"")) {
        rec_text
    } else {
        format!("{}\n{{\"type\":\"game_end\",\"winner\":-1,\"reason\":\"live\",\"total_steps\":0}}\n", rec_text.trim_end())
    };
    let recording = digimon_engine::dcgo_recording::parse_jsonl(&rec_text).map_err(|e| format!("parse: {e:?}"))?;

    // 2. replay it
    let mut session = ReplaySession::from_dcgo(recording, card_data, false).map_err(|e| format!("session: {e:?}"))?;
    session.run_to_completion();
    // DCGO picks a digivolution source in two steps (the Digimon, then the card); the replay folds the Digimon row
    // into the card row that follows it. Live, the card row does not exist yet: hold back a trailing carrier-only
    // row that does not resolve on its own, and answer from the state before it (the pick is still open).
    if session.divergences().first().is_some() {
        let mut lines: Vec<&str> = rec_text.lines().filter(|l| !l.trim().is_empty()).collect();
        let end = lines.pop();
        let carrier = lines.last().copied().filter(|l| {
            l.contains("\"prompt\":\"SelectPermanentEffect\"") && l.contains("\"targets\"") && !l.contains("\"card_ids\"")
        });
        if let (Some(_), Some(end)) = (carrier, end) {
            lines.pop();
            lines.push(end);
            let trimmed = lines.join("
");
            if let Ok(rec2) = digimon_engine::dcgo_recording::parse_jsonl(&trimmed) {
                if let Ok(mut s2) = ReplaySession::from_dcgo(rec2, card_data, false) {
                    s2.run_to_completion();
                    if s2.divergences().is_empty() {
                        session = s2;
                    }
                }
            }
        }
    }
    if let Some(d) = session.divergences().first() {
        return Ok(json!({"id": id, "status": "diverged", "step": d.step, "message": format!("{:?}", d.kind)}));
    }
    // A one-card ordering prompt has a single legal answer and DCGO never asks it (RevealLibrary places a lone
    // remaining card itself); the replay answers it when the next row arrives (answer_forced_single_ordering_prompt).
    // Apply it here too, so the decision we answer is the one DCGO is actually waiting on.
    let mut advanced = session.game.clone();
    for _ in 0..8 {
        let forced = match advanced.pending_selection.as_ref() {
            Some(p) if matches!(p.kind, digimon_engine::selection::SelectionKind::OrderedPermutation { remaining: 1 })
                && p.valid_action_ids.len() == 1 =>
            {
                Some((p.selecting_player, p.valid_action_ids[0]))
            }
            _ => None,
        };
        let Some((actor, action)) = forced else { break };
        advanced.decode_action(action, actor);
    }
    let game = &advanced;
    // DCGO-vs-engine audit: DCGO's own zones (its latest snapshot) against the engine's replay of DCGO's
    // decisions. A difference that persists is a real rules / card difference (or a recorder gap).
    // Reported once it has held for 3 requests in a row: DCGO's snapshot runs ahead of its own logged rows
    // during animations (a card drawn or played before its row is written).
    {
        use std::sync::Mutex;
        static LAST: Mutex<(String, u32)> = Mutex::new((String::new(), 0));
        let mut last = LAST.lock().unwrap();
        match zone_mismatch(game, Path::new(raw)) {
            Some(note) => {
                if last.0 == note {
                    last.1 += 1;
                } else {
                    *last = (note.clone(), 1);
                }
                if last.1 == 3 {
                    eprintln!("ZONE-MISMATCH step {} turn {}: {}", session.current_step(), game.turn_count, note);
                }
            }
            None => *last = (String::new(), 0),
        }
    }
    if game.game_over {
        return Ok(json!({"id": id, "status": "game_over"}));
    }
    let who = digimon_engine::policies::current_decision_player(game);
    if who != seat {
        return Ok(json!({"id": id, "status": "wait", "decision_player": who, "steps": session.current_step(),
                         "phase": format!("{:?}", game.current_phase), "turn": game.turn_count,
                         "pending": game.pending_selection.as_ref().map(|p| format!("{:?}: {}", p.kind, p.prompt))}));
    }

    // 3a. script: the recorded action for this exact state
    if let Some(script) = script {
        let mut out = script.answer(game, seat)?;
        out["id"] = json!(id);
        out["steps"] = json!(session.current_step());
        return Ok(out);
    }

    // 3. policy
    let mask = build_action_mask(game, seat);
    let obs = build_observation_tensor(game, seat, registry, profile);
    let legal: Vec<usize> = mask.iter().enumerate().filter(|(_, v)| **v > 0.5).map(|(i, _)| i).collect();
    if legal.is_empty() {
        return Ok(json!({"id": id, "status": "no_legal_action"}));
    }
    let results = evaluator
        .evaluate_batch(&[obs.as_slice()], &[mask.as_slice()])
        .map_err(|e| format!("inference: {e}"))?;
    let res = results.first().ok_or("no evaluation result")?;
    let action = legal
        .iter()
        .copied()
        .max_by(|a, b| res.policy[*a].partial_cmp(&res.policy[*b]).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap() as u16;

    // 4. describe it in DCGO terms
    if game.pending_selection.is_some() {
        let mut out = selection_chain(game, seat, registry, profile, evaluator)?;
        out["id"] = json!(id);
        out["status"] = json!("act");
        out["action_id"] = json!(action);
        out["value"] = json!(res.value);
        out["prob"] = json!(res.policy[action as usize]);
        out["legal"] = json!(legal.len());
        out["steps"] = json!(session.current_step());
        return Ok(out);
    }
    let mut out = describe(game, seat, action);
    out["id"] = json!(id);
    out["status"] = json!("act");
    out["action_id"] = json!(action);
    out["value"] = json!(res.value);
    out["prob"] = json!(res.policy[action as usize]);
    out["legal"] = json!(legal.len());
    out["steps"] = json!(session.current_step());
    Ok(out)
}

/// The DCGO prompt family a pending engine selection corresponds to.
fn selection_category(p: &digimon_engine::selection::PendingSelection) -> &'static str {
    use digimon_engine::selection::SelectionKind as K;
    match &p.kind {
        K::Hand => "hand",
        K::Reveal | K::RevealBucket { .. } => "card",
        K::Trash => "card",
        K::Security => "card",
        K::Target | K::OwnField | K::OppField | K::AnyField => "permanent",
        // multi-zone / stack / security picks are DCGO card panels too
        K::UnionZone { .. } | K::Source | K::SourceMulti { .. } | K::Material | K::CountCappedMultiSelect { .. } => "card",
        K::EffectChoice => "effect_choice",
        K::TriggerOrder => "trigger_order",
        // "Place ... in any order": DCGO's card panel, clicked in the chosen order
        K::OrderedPermutation { .. } => "card",
        // "You may activate X's triggered effect": DCGO's OptionalSkill ("Use" / back button)
        K::Replacement => "optional",
        _ => "other",
    }
}

/// What one selection action picks: card id + owner (+ field slot for permanents), or a branch index, or a decline.
fn describe_pick(game: &digimon_engine::game::Game, seat: u8, action: u16) -> Value {
    let p = match game.pending_selection.as_ref() {
        Some(p) => p,
        None => return json!({"pick": "none"}),
    };
    if action == PASS {
        return json!({"pick": "decline"});
    }
    if matches!(p.kind, digimon_engine::selection::SelectionKind::OrderedPermutation { .. }) {
        // the i-th still-unplaced item (the prompt re-installs over the shrinking list after every pick)
        let i = action.wrapping_sub(digimon_engine::action::space::SEL_REVEAL_START) as usize;
        let id = digimon_engine::runners::selection_resolve::permutation_remaining_ids(game).and_then(|ids| ids.get(i).cloned());
        return json!({"pick": "card", "card_id": id, "zone": "Reveal", "order": true});
    }
    if matches!(p.kind, digimon_engine::selection::SelectionKind::Replacement) {
        return json!({"pick": "accept", "card_id": card_id_by_handle(game, p.source_card)});
    }
    if let Some(choice) = p.card_choice.as_ref() {
        let view = choice.view(game, seat);
        if let Some(row) = view.rows.iter().find(|r| r.action_id == action) {
            // a digivolution source names its carrier: DCGO first asks for that Digimon, then for the card
            let carrier = row.carrier_slot.and_then(|slot| {
                let pl = game.player(row.owner.unwrap_or(seat));
                pl.battle_area.get(slot as usize).map(|perm| card_id(game, perm.top_card()))
            });
            return json!({"pick": "card", "card_id": row.card_id, "owner": row.owner, "zone": row.zone.as_ref().map(|z| format!("{:?}", z)),
                          "slot": row.carrier_slot, "carrier_card_id": carrier});
        }
    }
    use digimon_engine::selection::SelectionKind as K;
    let owner = p.zone_owner.unwrap_or(p.selecting_player);
    // count-capped multi-picks (select_zone_cards): hand picks are PLAY_HAND_START + slot, trash picks
    // TRASH_EFFECT_START + index (the replay's count_capped_zone_ids decodes them the same way)
    if matches!(p.kind, K::CountCappedMultiSelect { .. }) {
        if (PLAY_HAND_START..PLAY_HAND_END).contains(&action) {
            let i = (action - PLAY_HAND_START) as usize;
            return json!({"pick": "card", "owner": owner, "zone": "Hand",
                          "card_id": game.player(owner).hand.get(i).map(|c| card_id(game, c))});
        }
        if (TRASH_EFFECT_START..TRASH_EFFECT_END).contains(&action) {
            let i = (action - TRASH_EFFECT_START) as usize;
            return json!({"pick": "card", "owner": owner, "zone": "Trash",
                          "card_id": game.player(owner).trash.get(i).map(|c| card_id(game, c))});
        }
    }
    match &p.kind {
        // a hand pick is HAND_EFFECT_START + slot, or PLAY_HAND_START + slot on prompts that use the play band
        K::Hand if (HAND_EFFECT_START..HAND_EFFECT_END).contains(&action) || (PLAY_HAND_START..PLAY_HAND_END).contains(&action) => {
            let i = if action >= HAND_EFFECT_START { action - HAND_EFFECT_START } else { action - PLAY_HAND_START } as usize;
            json!({"pick": "card", "owner": owner, "zone": "Hand",
                   "card_id": game.player(owner).hand.get(i).map(|c| card_id(game, c))})
        }
        K::Reveal | K::RevealBucket { .. } if (SEL_REVEAL_START..SEL_REVEAL_END).contains(&action) => {
            let i = (action - SEL_REVEAL_START) as usize;
            json!({"pick": "card", "owner": owner, "zone": "Reveal",
                   "card_id": game.revealed_cards.get(i).map(|c| card_id(game, c))})
        }
        K::Trash if (TRASH_EFFECT_START..TRASH_EFFECT_END).contains(&action) => {
            let i = (action - TRASH_EFFECT_START) as usize;
            json!({"pick": "card", "owner": owner, "zone": "Trash",
                   "card_id": game.player(owner).trash.get(i).map(|c| card_id(game, c))})
        }
        K::OwnField | K::OppField | K::Target | K::AnyField if (ATTACK_START..ATTACK_END).contains(&action) => {
            let (a, slot) = decode_attack(action);
            let who = match &p.kind {
                K::AnyField => a as u8,
                K::OwnField => p.selecting_player,
                K::OppField => 1 - p.selecting_player,
                _ => p.zone_owner.unwrap_or(1 - p.selecting_player),
            };
            json!({"pick": "permanent", "owner": who, "slot": slot,
                   "card_id": game.player(who).battle_area.get(slot as usize).map(|perm| card_id(game, perm.top_card()))})
        }
        K::TriggerOrder => {
            // DCGO asks trigger order in its card panel, listing each trigger's SOURCE card: name the pick by it
            let choice = p.effect_choices.as_ref().and_then(|e| e.iter().position(|c| c.action_id == action).map(|i| (i, &e[i])));
            json!({"pick": "trigger", "index": choice.map(|c| c.0),
                   "card_id": choice.and_then(|c| c.1.source_card).and_then(|h| card_id_by_handle(game, h)),
                   "label": choice.map(|c| c.1.label.clone())})
        }
        K::EffectChoice => {
            let idx = p.effect_choices.as_ref().and_then(|e| e.iter().position(|c| c.action_id == action));
            json!({"pick": "branch", "index": idx,
                   "label": p.effect_choices.as_ref().and_then(|e| e.iter().find(|c| c.action_id == action).map(|c| c.label.clone()))})
        }
        _ => json!({"pick": "unknown", "action_id": action}),
    }
}

/// Run the current prompt forward on a clone with the model's own choices until it ends (or another prompt/player
/// takes over), collecting every pick: DCGO answers a multi-pick prompt in ONE call, the engine asks pick by pick.
fn selection_chain(
    game: &digimon_engine::game::Game,
    seat: u8,
    registry: &digimon_engine::card_registry::CardRegistry,
    profile: digimon_engine::observation::ObservationProfileId,
    evaluator: &mut BatchedPolicyValueEvaluator,
) -> Result<Value, String> {
    let mut g = game.clone();
    let first = g.pending_selection.as_ref().ok_or("no selection")?;
    let category = selection_category(first);
    let prompt = first.prompt.clone();
    let kind = format!("{:?}", first.kind);
    let mut picks = Vec::new();
    for _ in 0..24 {
        let Some(p) = g.pending_selection.as_ref() else { break };
        if p.selecting_player != seat || selection_category(p) != category {
            break;
        }
        let mask = build_action_mask(&g, seat);
        let obs = build_observation_tensor(&g, seat, registry, profile);
        let legal: Vec<usize> = mask.iter().enumerate().filter(|(_, v)| **v > 0.5).map(|(i, _)| i).collect();
        if legal.is_empty() {
            break;
        }
        let res = evaluator
            .evaluate_batch(&[obs.as_slice()], &[mask.as_slice()])
            .map_err(|e| format!("inference: {e}"))?;
        let r = res.first().ok_or("no evaluation result")?;
        let a = legal
            .iter()
            .copied()
            .max_by(|x, y| r.policy[*x].partial_cmp(&r.policy[*y]).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap() as u16;
        let pick = describe_pick(&g, seat, a);
        let declined = pick["pick"] == "decline";
        picks.push(pick);
        g.decode_action(a, seat);
        if declined {
            break;
        }
    }
    Ok(json!({"kind": "selection", "category": category, "selection_kind": kind, "prompt": prompt, "picks": picks}))
}

/// Compare DCGO's latest `Snapshot.*` row (hand, trash, battle-area tops, security count per player) with the
/// engine state. `None` when they agree or there is no snapshot.
fn zone_mismatch(game: &digimon_engine::game::Game, raw: &Path) -> Option<String> {
    let text = std::fs::read_to_string(raw).ok()?;
    let snap: Value = text
        .lines()
        .rev()
        .filter(|l| l.contains("\"method\":\"Snapshot."))
        .find_map(|l| serde_json::from_str(l).ok())?;
    let mut diffs = Vec::new();
    for p in snap["players"].as_array()? {
        let pid = p["id"].as_u64()? as u8;
        if pid > 1 {
            continue;
        }
        let pl = game.player(pid);
        let sorted = |v: Vec<String>| {
            let mut v = v;
            v.sort();
            v
        };
        let ids = |key: &str| sorted(p[key].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default());
        let checks: [(&str, Vec<String>, Vec<String>); 3] = [
            ("hand", ids("hand"), sorted(pl.hand.iter().map(|c| card_id(game, c)).collect())),
            ("trash", ids("trash"), sorted(pl.trash.iter().map(|c| card_id(game, c)).collect())),
            ("battle", ids("battle"), sorted(pl.battle_area.iter().map(|b| card_id(game, b.top_card())).collect())),
        ];
        for (zone, dcgo, ours) in checks {
            if dcgo != ours {
                diffs.push(format!("p{pid} {zone}: dcgo {dcgo:?} engine {ours:?}"));
            }
        }
        if let Some(sec) = p["security"].as_array() {
            if sec.len() != pl.security.len() {
                diffs.push(format!("p{pid} security: dcgo {} engine {}", sec.len(), pl.security.len()));
            }
        }
    }
    (!diffs.is_empty()).then(|| diffs.join("; "))
}

fn card_id(game: &digimon_engine::game::Game, cs: &digimon_engine::card_source::CardSource) -> String {
    cs.card_id(&game.card_data).to_string()
}

fn describe(game: &digimon_engine::game::Game, seat: u8, action: u16) -> Value {
    let me = game.player(seat);
    if game.pending_selection.is_some() {
        return json!({"kind": "selection", "prompt": game.pending_selection.as_ref().map(|p| p.prompt.clone())});
    }
    if game.mulligan_current_player().is_some() {
        return json!({"kind": if action == 1 { "mulligan_redraw" } else { "mulligan_keep" }});
    }
    match action {
        a if a == PASS => json!({"kind": "pass"}),
        a if a == HATCH => json!({"kind": "hatch"}),
        a if a == MOVE_FROM_BREEDING => json!({"kind": "move_from_breeding"}),
        a if (PLAY_HAND_START..PLAY_HAND_END).contains(&a) => {
            let slot = (a - PLAY_HAND_START) as usize;
            json!({"kind": "play", "hand_slot": slot, "card_id": me.hand.get(slot).map(|c| card_id(game, c))})
        }
        a if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&a) => {
            let (hand, field) = decode_digivolve(a);
            let target = if field == BREEDING_TARGET {
                me.breeding_area.as_ref().map(|p| card_id(game, p.top_card()))
            } else {
                me.battle_area.get(field as usize).map(|p| card_id(game, p.top_card()))
            };
            json!({"kind": "digivolve", "hand_slot": hand, "card_id": me.hand.get(hand as usize).map(|c| card_id(game, c)),
                   "target_field": field, "target_breeding": field == BREEDING_TARGET, "target_card_id": target})
        }
        a if (ATTACK_START..ATTACK_END).contains(&a) => {
            let (attacker, target) = decode_attack(a);
            let opp = game.player(1 - seat);
            // which copy of that card: DCGO's field order can differ from the engine's slot order, so the mod
            // resolves the attacker / target by card id + occurrence, not by slot
            let occ = |side: &digimon_engine::player::Player, slot: usize| -> usize {
                let id = side.battle_area.get(slot).map(|p| card_id(game, p.top_card()));
                side.battle_area[..slot.min(side.battle_area.len())].iter().filter(|p| Some(card_id(game, p.top_card())) == id).count()
            };
            json!({"kind": "attack", "attacker_field": attacker,
                   "attacker_occurrence": occ(me, attacker as usize),
                   "target_occurrence": if target == 14 { 0 } else { occ(opp, target as usize) },
                   "attacker_card_id": me.battle_area.get(attacker as usize).map(|p| card_id(game, p.top_card())),
                   "target_security": target == 14,
                   "target_field": if target == 14 { Value::Null } else { json!(target) },
                   "target_card_id": if target == 14 { Value::Null } else { json!(opp.battle_area.get(target as usize).map(|p| card_id(game, p.top_card()))) }})
        }
        a if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&a) => {
            // a [Main] effect of a permanent: DCGO's ActivatePermanentAction(field index, skill index); the mod
            // resolves the field index by card id + occurrence, like an attacker
            let (slot, effect) = decode_field_effect(a);
            let id = me.battle_area.get(slot as usize).map(|p| card_id(game, p.top_card()));
            let occurrence = me.battle_area[..(slot as usize).min(me.battle_area.len())]
                .iter()
                .filter(|p| Some(card_id(game, p.top_card())) == id)
                .count();
            json!({"kind": "activate", "field_slot": slot, "card_id": id, "occurrence": occurrence, "skill_index": effect})
        }
        _ => json!({"kind": "other"}),
    }
}

/// Coarse but complete state key: everything a recorded decision depends on that both engines see the same way
/// (card ids, never instance ids, which differ between a DCGO-built and a seeded game).
fn fingerprint(game: &digimon_engine::game::Game) -> String {
    use std::fmt::Write;
    let mut f = format!("t{} {:?} m{} |", game.turn_count, game.current_phase, game.memory);
    for seat in 0..2u8 {
        let p = game.player(seat);
        let ids = |v: &[digimon_engine::card_source::CardSource]| v.iter().map(|c| card_id(game, c)).collect::<Vec<_>>().join(",");
        let mut trash: Vec<String> = p.trash.iter().map(|c| card_id(game, c)).collect();
        trash.sort();
        // deck / security / egg in order: a wrong orientation in the playback plan shows up at the first state
        let _ = write!(f, " p{seat} h[{}] d[{}] s[{}] e[{}] tr[{}] b[", ids(&p.hand), ids(&p.deck), ids(&p.security),
                       ids(&p.digitama_deck), trash.join(","));
        for perm in &p.battle_area {
            let _ = write!(f, "{}/{}{} ", card_id(game, perm.top_card()), perm.card_sources.len(), if perm.is_suspended { "s" } else { "" });
        }
        let _ = write!(f, "] br[{}]", p.breeding_area.as_ref().map(|b| format!("{}/{}", card_id(game, b.top_card()), b.card_sources.len())).unwrap_or_default());
    }
    if let Some(p) = game.pending_selection.as_ref() {
        let _ = write!(f, " sel {:?} by {} '{}'", p.kind, p.selecting_player, p.prompt);
    }
    f
}

/// A native recording (`GameRecorder` JSON, e.g. from `sim-record`) replayed once at load: the state before each
/// recorded decision and the decision itself.
pub struct Script {
    fps: Vec<String>,
    actions: Vec<(u8, u16)>,
    mulligan: [Option<u16>; 2],
    cursor: usize,
}

impl Script {
    pub fn load(path: &Path, card_data: &HashMap<String, CardData>) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{e}"))?;
        let v: Value = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
        let rec = v.get("recording").cloned().unwrap_or(v);
        // recorded player ids are the Python 1/2 convention
        let mut mulligan = [None, None];
        for a in rec["actions"].as_array().ok_or("recording has no actions")? {
            if a["phase"] == "Mulligan" {
                let seat = (a["player_id"].as_u64().unwrap_or(1).saturating_sub(1) as usize).min(1);
                if mulligan[seat].is_none() {
                    mulligan[seat] = a["action_id"].as_u64().map(|x| x as u16);
                }
            }
        }
        let mut session = ReplaySession::new(rec, card_data, false).map_err(|e| format!("{e:?}"))?;
        let (mut fps, mut actions) = (Vec::new(), Vec::new());
        while !session.is_complete() && !session.is_game_over() {
            let fp = fingerprint(&session.game);
            let r = session.step();
            fps.push(fp);
            actions.push((r.player_id, r.action_id));
        }
        Ok(Self { fps, actions, mulligan, cursor: 0 })
    }

    /// The native replay can fold a decision into the step before it (an optional yes/no the recording answers
    /// right after a forced pick): the state recorded with that decision is then already the state AFTER it, and
    /// the live state waiting on it appears nowhere. Near the cursor, find a recorded decision of this seat that is
    /// legal here and whose result is the state recorded with it.
    fn shifted_match(&self, game: &digimon_engine::game::Game, seat: u8) -> Option<usize> {
        let lo = self.cursor.saturating_sub(4).min(self.actions.len());
        (lo..self.actions.len()).chain(0..lo).find(|&k| {
            let (actor, action) = self.actions[k];
            if actor != seat || build_action_mask(game, seat).get(action as usize).copied().unwrap_or(0.0) < 0.5 {
                return false;
            }
            let mut g = game.clone();
            g.decode_action(action, seat);
            fingerprint(&g) == self.fps[k]
        })
    }

    fn answer(&mut self, game: &digimon_engine::game::Game, seat: u8) -> Result<Value, String> {
        if game.mulligan_current_player().is_some() {
            let a = self.mulligan[seat as usize].unwrap_or(0);
            let mut out = describe(game, seat, a);
            out["status"] = json!("act");
            out["action_id"] = json!(a);
            out["script"] = json!("mulligan");
            return Ok(out);
        }
        let fp = fingerprint(game);
        let k = (self.cursor..self.fps.len())
            .chain(0..self.cursor.min(self.fps.len()))
            .find(|&k| self.fps[k] == fp)
            .or_else(|| self.shifted_match(game, seat))
            // a decision folded into the step before it leaves two equal recorded states; the second carries the
            // decision actually made from that state
            .map(|mut k| {
                while k + 1 < self.fps.len() && self.fps[k + 1] == self.fps[k] && {
                    let (actor, action) = self.actions[k];
                    actor == seat && build_action_mask(game, seat).get(action as usize).copied().unwrap_or(0.0) < 0.5
                } {
                    k += 1;
                }
                k
            });
        let Some(k) = k else {
            // the recorded state closest to DCGO's (fewest differing tokens), to show what differs
            let toks: Vec<&str> = fp.split(' ').collect();
            let best = self.fps.iter().enumerate().min_by_key(|(_, f)| {
                let ft: Vec<&str> = f.split(' ').collect();
                ft.iter().zip(toks.iter()).filter(|(a, b)| a != b).count() + ft.len().abs_diff(toks.len())
            });
            let nearest = best.map(|(i, f)| json!({"index": i, "fp": f, "action": self.actions.get(i),
                "following": (i + 1..(i + 5).min(self.fps.len())).map(|j| json!({"fp": self.fps[j], "action": self.actions.get(j)})).collect::<Vec<_>>()}));
            return Ok(json!({"status": "diverged", "message": "DCGO reached a state the sim recording never did",
                             "fingerprint": fp, "script_cursor": self.cursor,
                             "expected": self.fps.get(self.cursor),
                             "nearest": nearest,
                             "prev": self.cursor.checked_sub(1).and_then(|c| self.fps.get(c)),
                             "prev2": self.cursor.checked_sub(2).and_then(|c| self.fps.get(c)),
                             "prev_actions": self.cursor.checked_sub(3).map(|c| self.actions[c..self.cursor].to_vec())}));
        };
        let (actor, action) = self.actions[k];
        if actor != seat {
            return Ok(json!({"status": "wait", "decision_player": actor, "script_step": k}));
        }
        let mask = build_action_mask(game, seat);
        if mask.get(action as usize).copied().unwrap_or(0.0) < 0.5 {
            return Ok(json!({"status": "diverged", "message": format!("recorded action {action} is not legal here"), "script_step": k}));
        }
        // An accepted "you may activate X" (Replacement accept) whose effect then asks this seat a pick: DCGO has no
        // separate accept for most of these (it opens the pick, with "don't" on its back button, or asks "Use?"
        // which the card / hand / permanent answers confirm first), and the replay folds the accept into the pick
        // row. Describe the pick that follows instead.
        let accepted;
        let (game, k) = match game.pending_selection.as_ref() {
            Some(p) if matches!(p.kind, digimon_engine::selection::SelectionKind::Replacement)
                && action == digimon_engine::action::space::REPLACEMENT_ACCEPT
                && k + 1 < self.actions.len()
                && self.actions[k + 1].0 == seat =>
            {
                let mut g = game.clone();
                g.decode_action(action, seat);
                if g.pending_selection.as_ref().is_some_and(|q| q.selecting_player == seat) {
                    accepted = g;
                    (&accepted, k + 1)
                } else {
                    (game, k)
                }
            }
            _ => (game, k),
        };
        let action = self.actions[k].1;
        let mut out = if let Some(first) = game.pending_selection.as_ref() {
            // DCGO answers a multi-pick prompt in one call: take the recorded picks while the same prompt continues
            let category = selection_category(first);
            let mut g = game.clone();
            let mut picks = Vec::new();
            let mut j = k;
            while j < self.actions.len() {
                let Some(p) = g.pending_selection.as_ref() else { break };
                // one DCGO answer covers the picks of ONE prompt (a multi-pick re-parks the same prompt); two
                // different prompts of the same category (e.g. "link to which Digimon", then "which Digimon gains
                // +3000") are two DCGO prompts and two answers
                if p.selecting_player != seat || selection_category(p) != category || self.actions[j].0 != seat
                    || (j > k && p.prompt != first.prompt)
                {
                    break;
                }
                let pick = describe_pick(&g, seat, self.actions[j].1);
                let declined = pick["pick"] == "decline";
                picks.push(pick);
                g.decode_action(self.actions[j].1, seat);
                j += 1;
                if declined {
                    break;
                }
            }
            self.cursor = j;
            // the card whose effect asks: DCGO may first ask which of several triggers to resolve (an order the
            // engine did not need), and the mod answers that panel with this card
            json!({"kind": "selection", "category": category, "selection_kind": format!("{:?}", first.kind),
                   "prompt": first.prompt, "picks": picks,
                   "source_card_id": card_id_by_handle(game, first.source_card)})
        } else {
            self.cursor = k + 1;
            describe(game, seat, action)
        };
        out["status"] = json!("act");
        out["action_id"] = json!(action);
        out["script_step"] = json!(k);
        Ok(out)
    }
}

/// Play one game in our engine with the model on both seats (argmax, concede masked) and write it as a native
/// recording: the input of DCGO playback.
#[allow(clippy::too_many_arguments)]
pub fn sim_record(
    model_path: PathBuf,
    deck0: PathBuf,
    deck1: PathBuf,
    seed: u64,
    out: PathBuf,
    tensor_profile: Option<String>,
    max_steps: u32,
    card_data: HashMap<String, CardData>,
) -> ExitCode {
    let read_deck = |p: &Path| -> Result<Vec<String>, String> {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(p).map_err(|e| format!("{e}"))?).map_err(|e| format!("{e}"))?;
        let list = v.get("cards").unwrap_or(&v);
        Ok(list.as_array().ok_or("deck file must be a list of card ids (or {\"cards\": [...]})")?
            .iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
    };
    let (d0, d1) = match (read_deck(&deck0), read_deck(&deck1)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("error reading decks: {e}");
            return ExitCode::from(2);
        }
    };
    let profile = match crate::winprob::resolve_profile(&model_path, tensor_profile.as_deref()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error resolving tensor profile: {e}");
            return ExitCode::from(2);
        }
    };
    let mut evaluator = match BatchedPolicyValueEvaluator::load_for_profile(&model_path, profile) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("error loading model {}: {}", model_path.display(), e);
            return ExitCode::from(2);
        }
    };
    let registry = digimon_engine::card_registry::CardRegistry::from_cards(&card_data);
    let mut runner = match digimon_engine::runners::HeadlessRunner::new(d0.clone(), d1.clone(), &card_data, false, true, false, Some(seed)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error building the game: {e}");
            return ExitCode::from(2);
        }
    };
    let mut steps = 0u32;
    while !runner.is_game_over() && steps < max_steps {
        let game = runner.game();
        let seat = runner.current_decision_player();
        let mut mask = build_action_mask(game, seat);
        if let Some(m) = mask.get_mut(CONCEDE_GAME as usize) {
            *m = 0.0;
        }
        let obs = build_observation_tensor(game, seat, &registry, profile);
        let legal: Vec<usize> = mask.iter().enumerate().filter(|(_, v)| **v > 0.5).map(|(i, _)| i).collect();
        if legal.is_empty() {
            eprintln!("no legal action at step {steps}");
            break;
        }
        let action = match evaluator.evaluate_batch(&[obs.as_slice()], &[mask.as_slice()]) {
            Ok(r) => {
                let r = &r[0];
                legal.iter().copied().max_by(|a, b| r.policy[*a].partial_cmp(&r.policy[*b]).unwrap_or(std::cmp::Ordering::Equal)).unwrap() as u16
            }
            Err(e) => {
                eprintln!("inference: {e}");
                return ExitCode::from(2);
            }
        };
        runner.step(action);
        steps += 1;
    }
    let doc = json!({
        "source": "digimon-engine-cli sim-record",
        "model": model_path.display().to_string(),
        "seed": seed,
        "decks": [d0, d1],
        "outcome": {"game_over": runner.is_game_over(), "winner_id": runner.winner_id(), "steps": steps},
        "recording": runner.get_recording(),
    });
    if let Err(e) = std::fs::write(&out, serde_json::to_vec_pretty(&doc).unwrap_or_default()) {
        eprintln!("writing {}: {e}", out.display());
        return ExitCode::from(2);
    }
    eprintln!("sim-record: {} steps, game_over={}, winner={} -> {}", steps, runner.is_game_over(), runner.winner_id(), out.display());
    ExitCode::SUCCESS
}

/// The card id behind a card handle, wherever the card is (hand, battle area stacks, breeding, trash, security, deck).
fn card_id_by_handle(game: &digimon_engine::game::Game, h: digimon_engine::card_source::CardHandle) -> Option<String> {
    for seat in 0..2u8 {
        let p = game.player(seat);
        let stacks = p.battle_area.iter().chain(p.breeding_area.iter()).flat_map(|perm| perm.card_sources.iter());
        let zones = p.hand.iter().chain(p.trash.iter()).chain(p.security.iter()).chain(p.deck.iter()).chain(stacks);
        for cs in zones {
            if cs.handle() == h {
                return Some(card_id(game, cs));
            }
        }
    }
    None
}
