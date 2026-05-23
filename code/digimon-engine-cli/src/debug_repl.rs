//! Interactive REPL for the `debug` subcommand.
//!
//! Reads one command per line from stdin and dispatches to the
//! corresponding `LiveGame` method. Each command's output is printed to
//! stdout; errors go to stderr but the REPL stays alive.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use digimon_engine::card_data::CardData;
use digimon_engine::live_game::LiveGame;
use digimon_engine::view::Perspective;

use crate::{flush_stdout, load_deck, parse_perspective, print_json, read_json_path, read_line};

/// Entry point — drives the REPL loop until EOF or `quit`.
pub fn run(
    deck1: Option<PathBuf>,
    deck2: Option<PathBuf>,
    seed: Option<u64>,
    card_data: HashMap<String, CardData>,
) -> ExitCode {
    let mut state: Option<LiveGame> = match (deck1.as_deref(), deck2.as_deref()) {
        (Some(d1), Some(d2)) => match construct_from_decks(d1, d2, seed, &card_data) {
            Ok(lg) => Some(lg),
            Err(e) => {
                eprintln!("error constructing initial game: {}", e);
                None
            }
        },
        _ => None,
    };

    println!(
        "digimon-engine-cli REPL — type `help` for commands, `quit` to exit. \
         Pool size: {} cards.",
        card_data.len()
    );
    let mut reader = crate::stdin_reader();
    loop {
        print!("> ");
        flush_stdout();
        let Some(line) = read_line(&mut reader) else {
            break;
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cont = dispatch(line, &mut state, &card_data);
        if !cont {
            break;
        }
    }
    ExitCode::SUCCESS
}

/// Returns `false` to terminate the REPL.
fn dispatch(line: &str, state: &mut Option<LiveGame>, card_data: &HashMap<String, CardData>) -> bool {
    let mut parts = line.split_whitespace();
    let Some(cmd) = parts.next() else {
        return true;
    };
    let args: Vec<&str> = parts.collect();
    match cmd {
        "quit" | "exit" => return false,
        "help" => print_help(),
        "new" => handle_new(state, &args, card_data),
        "load" => handle_load(state, &args, card_data),
        "state" => with_game(state, |g| print_json(&serde_json::to_value(g.state(view_arg(&args))).unwrap())),
        "hand" => {
            with_game(state, |g| {
                let player = player_arg(&args, 0);
                print_json(&serde_json::to_value(g.hand(player, view_arg(&args))).unwrap());
            });
        }
        "field" => {
            with_game(state, |g| {
                let player = player_arg(&args, 0);
                print_json(&serde_json::to_value(g.field(player, view_arg(&args))).unwrap());
            });
        }
        "security" => {
            with_game(state, |g| {
                let player = player_arg(&args, 0);
                print_json(&serde_json::to_value(g.security(player, view_arg(&args))).unwrap());
            });
        }
        "pending" => with_game(state, |g| match g.pending_selection() {
            Some(p) => print_json(&serde_json::to_value(p).unwrap()),
            None => println!("(no pending selection)"),
        }),
        "queue" => with_game(state, |g| print_json(&serde_json::to_value(g.effect_queue()).unwrap())),
        "events" => {
            let since = args
                .iter()
                .find_map(|a| a.strip_prefix("--since=").and_then(|s| s.parse::<u64>().ok()));
            with_game(state, |g| {
                print_json(&serde_json::to_value(g.events(since)).unwrap())
            });
        }
        "actions" => with_game(state, |g| {
            let player = player_arg(&args, 0);
            print_json(&serde_json::to_value(g.legal_actions(player)).unwrap());
        }),
        "play" => mutate(state, |g| {
            let player = player_arg(&args, 0);
            let idx: usize = args
                .iter()
                .filter(|a| !a.starts_with("--"))
                .nth(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            let r = g.play(player, idx);
            print_json(&serde_json::to_value(r).unwrap());
        }),
        "step" => mutate(state, |g| {
            let id: u16 = args
                .iter()
                .find_map(|a| a.parse::<u16>().ok())
                .unwrap_or(0);
            let r = g.step(id);
            print_json(&serde_json::to_value(r).unwrap());
        }),
        "resolve" => mutate(state, |g| {
            let player = player_arg(&args, 0);
            let id: u16 = args
                .iter()
                .filter(|a| !a.starts_with("--"))
                .nth(1)
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(0);
            let r = g.resolve_selection(player, id);
            print_json(&serde_json::to_value(r).unwrap());
        }),
        "end-turn" => mutate(state, |g| {
            let r = g.end_turn();
            print_json(&serde_json::to_value(r).unwrap());
        }),
        "pass" => mutate(state, |g| {
            let r = g.pass_turn();
            print_json(&serde_json::to_value(r).unwrap());
        }),
        "inspect" => {
            let Some(card_id) = args.first() else {
                eprintln!("usage: inspect <card_id>");
                return true;
            };
            with_game(state, |g| match g.inspect_card(card_id) {
                Some(ins) => print_json(&serde_json::to_value(ins).unwrap()),
                None => println!("(card {} not in pool)", card_id),
            });
        }
        other => {
            eprintln!("unknown command: {} (type `help`)", other);
        }
    }
    true
}

fn print_help() {
    println!(
        r#"Commands:
  new decks <deck1.json> <deck2.json> [--seed N]   construct fresh game
  load <recording.json> [--step N]                 load a recording (paused)
  state [--view player0|player1|god]               top-level state view
  hand <player> [--view ...]
  field <player> [--view ...]
  security <player> [--view ...]
  pending                                          current PendingSelection
  queue                                            EffectQueue
  events [--since=N]                               GameEvents since seq N
  actions <player>                                 legal decoded actions
  play <player> <hand_idx>                         play_from_hand
  resolve <player> <action_id>                     resolve current selection
  step <action_id>                                 raw decode_action dispatch
  end-turn                                         end_turn
  pass                                             pass_turn
  inspect <card_id>                                inspect card metadata
  help                                             this message
  quit                                             exit
"#
    );
}

fn handle_new(state: &mut Option<LiveGame>, args: &[&str], card_data: &HashMap<String, CardData>) {
    let Some(kind) = args.first() else {
        eprintln!("usage: new decks <d1> <d2> [--seed N]");
        return;
    };
    if *kind != "decks" {
        eprintln!("only `new decks` is supported in v1");
        return;
    }
    let Some(d1_path) = args.get(1) else {
        eprintln!("missing deck1 path");
        return;
    };
    let Some(d2_path) = args.get(2) else {
        eprintln!("missing deck2 path");
        return;
    };
    let seed = args
        .iter()
        .find_map(|a| a.strip_prefix("--seed=").and_then(|s| s.parse::<u64>().ok()));
    match construct_from_decks(std::path::Path::new(d1_path), std::path::Path::new(d2_path), seed, card_data) {
        Ok(lg) => {
            *state = Some(lg);
            println!("(game constructed)");
        }
        Err(e) => eprintln!("error: {}", e),
    }
}

fn handle_load(state: &mut Option<LiveGame>, args: &[&str], card_data: &HashMap<String, CardData>) {
    let Some(path) = args.first() else {
        eprintln!("usage: load <recording.json> [--step N]");
        return;
    };
    let step = args
        .iter()
        .find_map(|a| a.strip_prefix("--step=").and_then(|s| s.parse::<u32>().ok()));
    let recording = match read_json_path(std::path::Path::new(path)) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {}", e);
            return;
        }
    };
    let result = match step {
        Some(n) => LiveGame::from_recording_at_step(recording, n, card_data),
        None => LiveGame::from_recording(recording, card_data),
    };
    match result {
        Ok(lg) => {
            *state = Some(lg);
            println!("(recording loaded)");
        }
        Err(e) => eprintln!("error: {}", e),
    }
}

fn construct_from_decks(
    d1: &std::path::Path,
    d2: &std::path::Path,
    seed: Option<u64>,
    card_data: &HashMap<String, CardData>,
) -> Result<LiveGame, String> {
    let deck1 = load_deck(d1)?;
    let deck2 = load_deck(d2)?;
    LiveGame::from_decks(deck1, deck2, seed, card_data).map_err(|e| e.to_string())
}

fn with_game(state: &Option<LiveGame>, f: impl FnOnce(&LiveGame)) {
    match state.as_ref() {
        Some(g) => f(g),
        None => eprintln!("no game loaded — use `new decks ...` or `load ...`"),
    }
}

fn mutate(state: &mut Option<LiveGame>, f: impl FnOnce(&mut LiveGame)) {
    match state.as_mut() {
        Some(g) => f(g),
        None => eprintln!("no game loaded — use `new decks ...` or `load ...`"),
    }
}

fn view_arg(args: &[&str]) -> Perspective {
    args.iter()
        .find_map(|a| a.strip_prefix("--view="))
        .map(parse_perspective)
        .unwrap_or(Perspective::God)
}

fn player_arg(args: &[&str], default: u8) -> u8 {
    args.iter()
        .filter(|a| !a.starts_with("--"))
        .next()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(default)
}

// ── tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    fn minimal_db() -> HashMap<String, CardData> {
        let json = r#"{
            "ST1-01": {
                "card_id": "ST1-01", "card_name_eng": "Botamon",
                "card_effect_class_name": "ST1_01", "play_cost": 0, "dp": -1,
                "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
                "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": []
            },
            "ST1-03": {
                "card_id": "ST1-03", "card_name_eng": "Koromon",
                "card_effect_class_name": "ST1_03", "play_cost": 3, "dp": 2000,
                "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
                "type_eng": ["Lesser"], "form_eng": ["Rookie"], "attribute_eng": ["Free"],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": []
            }
        }"#;
        CardData::load_from_str(json).unwrap()
    }

    #[test]
    fn dispatch_help_does_not_panic() {
        let mut state: Option<LiveGame> = None;
        let cd = minimal_db();
        assert!(dispatch("help", &mut state, &cd));
    }

    #[test]
    fn dispatch_state_without_game_emits_error() {
        let mut state: Option<LiveGame> = None;
        let cd = minimal_db();
        // Without a loaded game, `state` should still return true (REPL
        // continues) but print to stderr.
        assert!(dispatch("state", &mut state, &cd));
    }

    #[test]
    fn quit_terminates_repl() {
        let mut state: Option<LiveGame> = None;
        let cd = minimal_db();
        assert!(!dispatch("quit", &mut state, &cd));
        assert!(!dispatch("exit", &mut state, &cd));
    }

    #[test]
    fn step_command_dispatches_when_game_present() {
        let cd = minimal_db();
        let deck: Vec<String> = std::iter::repeat("ST1-01".to_string())
            .take(5)
            .chain(std::iter::repeat("ST1-03".to_string()).take(45))
            .collect();
        let lg = LiveGame::from_decks(deck.clone(), deck, Some(1), &cd).unwrap();
        let mut state = Some(lg);
        // Mulligan keep.
        assert!(dispatch("step 0", &mut state, &cd));
        // Game should still be loaded.
        assert!(state.is_some());
    }

    #[test]
    fn read_line_handles_eof() {
        let input: &[u8] = b"";
        let mut reader = BufReader::new(input);
        assert!(read_line(&mut reader).is_none());
    }

    #[test]
    fn read_line_strips_trailing_newline() {
        let input: &[u8] = b"hello\n";
        let mut reader = BufReader::new(input);
        assert_eq!(read_line(&mut reader).as_deref(), Some("hello"));
    }
}
