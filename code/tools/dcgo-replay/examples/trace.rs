//! Step a DCGO recording through the engine one recorded step at a time and
//! print, for each step, the recorded action, the engine's pending selection
//! and the board — the cheap stand-in for the engine MCP when hunting a
//! divergence. Usage: `cargo run -p dcgo-replay --example trace -- <rec.jsonl> [from] [to]`.

use std::fs;

use dcgo_replay::{load_card_data, parse_jsonl};
use digimon_engine::live_game::LiveGame;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).expect("usage: trace <recording.jsonl> [from] [to]");
    let from: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let to: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(u32::MAX);
    let text = fs::read_to_string(path).expect("read recording");
    let text = text.trim_start_matches('\u{feff}');
    let recording = parse_jsonl(text).expect("parse recording");
    let card_data = load_card_data(None).expect("cards.json");
    let mut game = LiveGame::from_dcgo_recording(recording, &card_data).expect("build game");
    let stack = std::thread::Builder::new();
    let _ = stack;
    loop {
        let view = match game.replay_step_view() {
            Ok(v) => v,
            Err(e) => {
                println!("view error: {e:?}");
                break;
            }
        };
        if view.is_complete || view.paused {
            println!("STOP cursor={} complete={} paused={} divergences={:#?}", view.cursor, view.is_complete, view.paused, view.divergences);
            {
                let g = &game.game;
                println!(
                    "FINAL game_over={} winner={:?} phase={:?} turn={} memory={} pending={:?}",
                    g.game_over, g.winner, g.current_phase, g.turn_count, g.memory,
                    game.pending_selection()
                );
                for pid in 0..2usize {
                    println!(
                        "FINAL P{} security={} hand={} deck={} battle_area={}",
                        pid,
                        g.players[pid].security.len(),
                        g.players[pid].hand.len(),
                        g.players[pid].deck.len(),
                        g.players[pid].battle_area.len()
                    );
                }
                println!("FINAL attack_in_progress={:?}", g.pending_attack.is_some());
            }
            break;
        }
        if view.cursor >= from && view.cursor <= to {
            println!("--- cursor {} recorded={:?}", view.cursor, view.recorded);
            if let Some(p) = game.pending_selection() {
                println!("    pending: {p:?}");
                if let Some(r) = game.game.pending_selection_resume.as_ref() {
                    let kinds: Vec<String> = r
                        .frames
                        .iter()
                        .map(|f| format!("{f:?}").chars().take(40).collect())
                        .collect();
                    println!("    resume frames: {kinds:?}");
                }
            }
            println!("    in play: {:?}", view.card_ids_in_play);
            for pid in 0..2u8 {
                let g = &game.game;
                let entries: Vec<String> = g.players[pid as usize]
                    .battle_area
                    .iter()
                    .enumerate()
                    .map(|(i, perm)| {
                        let h = digimon_engine::permanent::PermanentHandle { player: pid, index: i as u8 };
                        format!(
                            "{}:{} dp={:?}",
                            i,
                            perm.top_card().card_id(&g.card_data),
                            g.effective_dp(h)
                        )
                    })
                    .collect();
                println!("    P{} field: [{}] memory={}", pid, entries.join(", "), g.memory);
                let deck = &g.players[pid as usize].deck;
                let top: Vec<String> = deck
                    .iter()
                    .rev()
                    .take(8)
                    .map(|c| c.card_id(&g.card_data).to_string())
                    .collect();
                println!("    P{} deck top->: {:?} (len {})  hand: {:?}", pid, top, deck.len(),
                    g.players[pid as usize].hand.iter().map(|c| c.card_id(&g.card_data).to_string()).collect::<Vec<_>>());
            }
        }
        if view.cursor > to {
            break;
        }
        if let Err(e) = game.step_forward() {
            println!("step_forward error at cursor {}: {e:?}", view.cursor);
            break;
        }
    }
}
