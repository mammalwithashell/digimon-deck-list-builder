//! Dev-only localhost HTTP bridge into the live desktop game.
//!
//! This entire module is compiled ONLY under the `debug-bridge` cargo
//! feature, so release/production bundles carry no network surface. Even
//! when compiled in, the server starts ONLY if the runtime env var
//! `DIGIMON_DEBUG_BRIDGE=1` is set, and it binds `127.0.0.1` exclusively.
//!
//! It shares the same `Arc<Mutex<Option<Game>>>` as the Tauri `rust_*`
//! commands, so staging/inspection here is reflected live in the window.
//! After any external mutation it emits a `debug:state-changed` window
//! event so the React frontend re-fetches and re-renders the board.
//!
//! The staging/inspection verbs delegate to the same engine primitives the
//! browser `/debug` path uses (`Game::apply_scenario`, `stage_inject_card`,
//! `set_memory`, `to_scenario`) and return the desktop DTO
//! (`game_state_dto`) — so the MCP exercises the desktop `engine_commands`
//! serialization wire that browser-mode cannot reach.
#![cfg(feature = "debug-bridge")]

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use digimon_engine::card_registry::CardRegistry;
use digimon_engine::game::Game;
use digimon_engine::rules::Rules;

use crate::engine_commands::{
    action_mask_bytes, current_decision_player, game_state_dto, GameSession, PlayerKind,
};

#[derive(Clone)]
struct BridgeState {
    game: Arc<Mutex<Option<Game>>>,
    session: Arc<Mutex<GameSession>>,
    app: AppHandle,
}

type BridgeResult = Result<Json<Value>, (StatusCode, String)>;

fn bad(msg: impl ToString) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, msg.to_string())
}

fn no_game() -> (StatusCode, String) {
    (
        StatusCode::BAD_REQUEST,
        "no active game (stage one first)".to_string(),
    )
}

/// Start the bridge if `DIGIMON_DEBUG_BRIDGE=1`. No-op otherwise.
pub fn maybe_spawn(
    app: &AppHandle,
    game: Arc<Mutex<Option<Game>>>,
    session: Arc<Mutex<GameSession>>,
) {
    if std::env::var("DIGIMON_DEBUG_BRIDGE").ok().as_deref() != Some("1") {
        return;
    }
    let port: u16 = std::env::var("DIGIMON_DEBUG_BRIDGE_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5174);

    let state = BridgeState {
        game,
        session,
        app: app.clone(),
    };
    let router = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/internal-state", get(internal_state))
        .route("/state", get(ui_state))
        .route("/mask", get(mask))
        .route("/export-scenario", post(export_scenario))
        .route("/stage", post(stage))
        .route("/apply", post(apply))
        .route("/inject-card", post(inject_card))
        .route("/set-memory", post(set_memory))
        .route("/step", post(step))
        .with_state(state);

    tauri::async_runtime::spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => {
                write_discovery_file(port);
                log::info!("debug bridge listening on http://{addr}");
                if let Err(e) = axum::serve(listener, router).await {
                    log::error!("debug bridge server error: {e}");
                }
            }
            Err(e) => log::error!("debug bridge bind {addr} failed: {e}"),
        }
    });
}

/// Drop a discovery file the MCP can read to find the live bridge.
fn write_discovery_file(port: u16) {
    let Some(dir) = dirs::data_dir().map(|d| d.join("digimon-tcg")) else {
        return;
    };
    let _ = std::fs::create_dir_all(&dir);
    let payload = json!({ "port": port, "base_url": format!("http://127.0.0.1:{port}") });
    let _ = std::fs::write(dir.join("debug_bridge.json"), payload.to_string());
}

fn notify(s: &BridgeState) {
    let _ = s.app.emit("debug:state-changed", ());
}

// ─── Read endpoints ──────────────────────────────────────────────────────

async fn internal_state(State(s): State<BridgeState>) -> BridgeResult {
    let guard = s.game.lock().map_err(bad)?;
    let game = guard.as_ref().ok_or_else(no_game)?;
    Ok(Json(game.to_scenario()))
}

async fn ui_state(State(s): State<BridgeState>) -> BridgeResult {
    let guard = s.game.lock().map_err(bad)?;
    let game = guard.as_ref().ok_or_else(no_game)?;
    Ok(Json(json!({
        "state": game_state_dto(game),
        "action_mask": action_mask_bytes(game),
    })))
}

async fn mask(State(s): State<BridgeState>) -> BridgeResult {
    let guard = s.game.lock().map_err(bad)?;
    let game = guard.as_ref().ok_or_else(no_game)?;
    Ok(Json(json!({ "action_mask": action_mask_bytes(game) })))
}

async fn export_scenario(State(s): State<BridgeState>) -> BridgeResult {
    let guard = s.game.lock().map_err(bad)?;
    let game = guard.as_ref().ok_or_else(no_game)?;
    Ok(Json(game.to_scenario()))
}

// ─── Mutating endpoints ──────────────────────────────────────────────────

/// Construct a fresh game from a scenario fixture's decks, apply the
/// fixture, and install it as the live desktop game. The whole stage→test
/// loop's "set up a board without playing" entry point.
async fn stage(State(s): State<BridgeState>, Json(fixture): Json<Value>) -> BridgeResult {
    let dto = stage_into(&s.game, &s.session, &fixture).map_err(bad)?;
    notify(&s);
    Ok(Json(json!({ "state": dto })))
}

/// Pure core of `/stage` (no `AppHandle`), so it is unit-testable without a
/// Tauri runtime. Builds a game from the fixture's decks, stages the
/// fixture onto it, and installs it into the shared state. Returns the
/// desktop `GameStateDto` as a `Value`.
pub(crate) fn stage_into(
    game_arc: &Arc<Mutex<Option<Game>>>,
    session_arc: &Arc<Mutex<GameSession>>,
    fixture: &Value,
) -> Result<Value, String> {
    let decks = fixture
        .get("decks")
        .and_then(Value::as_object)
        .ok_or("fixture missing 'decks' object")?;
    let deck = |key: &str| -> Result<Vec<String>, String> {
        decks
            .get(key)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .ok_or_else(|| format!("fixture missing decks['{key}']"))
    };
    let deck1 = deck("1")?;
    let deck2 = deck("2")?;
    let seed = fixture.get("seed").and_then(Value::as_u64);

    let db = digimon_engine::deck_tools::full_card_data();
    let mut game = Game::new(&[deck1, deck2], &db, Rules::standard(), seed)?;
    game.start_game();
    while let Some(p) = game.mulligan_current_player() {
        let _ = game.accept_mulligan(p, true);
    }
    game.apply_scenario(fixture)?;
    let registry = CardRegistry::from_cards(&db);
    let dto = serde_json::to_value(game_state_dto(&game)).map_err(|e| e.to_string())?;

    let mut g = game_arc.lock().map_err(|e| e.to_string())?;
    let mut sess = session_arc.lock().map_err(|e| e.to_string())?;
    *g = Some(game);
    *sess = GameSession {
        registry: Some(registry),
        // Both seats human so the desktop agent loop doesn't auto-step a
        // staged board out from under inspection.
        player_kinds: vec![PlayerKind::Human, PlayerKind::Human],
        player_model_ids: vec![None, None],
    };
    Ok(dto)
}

/// Re-stage the fixture onto the EXISTING game (its card pool must already
/// contain the referenced ids).
async fn apply(State(s): State<BridgeState>, Json(fixture): Json<Value>) -> BridgeResult {
    let mut guard = s.game.lock().map_err(bad)?;
    let game = guard.as_mut().ok_or_else(no_game)?;
    game.apply_scenario(&fixture).map_err(bad)?;
    let dto = game_state_dto(game);
    drop(guard);
    notify(&s);
    Ok(Json(json!({ "state": dto })))
}

#[derive(Deserialize)]
struct InjectCardBody {
    player_id: u8,
    card_id: String,
    zone: String,
}

async fn inject_card(State(s): State<BridgeState>, Json(b): Json<InjectCardBody>) -> BridgeResult {
    let pid = b.player_id.checked_sub(1).ok_or_else(|| bad("player_id must be 1/2"))?;
    let mut guard = s.game.lock().map_err(bad)?;
    let game = guard.as_mut().ok_or_else(no_game)?;
    game.stage_inject_card(pid, &b.card_id, &b.zone).map_err(bad)?;
    let dto = game_state_dto(game);
    drop(guard);
    notify(&s);
    Ok(Json(json!({ "state": dto })))
}

#[derive(Deserialize)]
struct SetMemoryBody {
    memory: i16,
}

async fn set_memory(State(s): State<BridgeState>, Json(b): Json<SetMemoryBody>) -> BridgeResult {
    let mut guard = s.game.lock().map_err(bad)?;
    let game = guard.as_mut().ok_or_else(no_game)?;
    game.set_memory(b.memory);
    let dto = game_state_dto(game);
    drop(guard);
    notify(&s);
    Ok(Json(json!({ "state": dto })))
}

#[derive(Deserialize)]
struct StepBody {
    action: u16,
    player_id: Option<u8>,
}

async fn step(State(s): State<BridgeState>, Json(b): Json<StepBody>) -> BridgeResult {
    let mut guard = s.game.lock().map_err(bad)?;
    let game = guard.as_mut().ok_or_else(no_game)?;
    let pid = match b.player_id {
        Some(p) => p.checked_sub(1).ok_or_else(|| bad("player_id must be 1/2"))?,
        None => current_decision_player(game),
    };
    let mask = action_mask_bytes(game);
    if (b.action as usize) >= mask.len() || mask[b.action as usize] != 1 {
        return Err(bad(format!("action {} is not legal", b.action)));
    }
    game.decode_action(b.action, pid);
    let dto = game_state_dto(game);
    drop(guard);
    notify(&s);
    Ok(Json(json!({ "state": dto })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn empty_state() -> (Arc<Mutex<Option<Game>>>, Arc<Mutex<GameSession>>) {
        (
            Arc::new(Mutex::new(None)),
            Arc::new(Mutex::new(GameSession::default())),
        )
    }

    fn starter_deck() -> Vec<String> {
        let mut d: Vec<String> = vec!["ST1-01".to_string(); 5];
        d.extend(std::iter::repeat("ST1-03".to_string()).take(45));
        d
    }

    #[test]
    fn stage_into_installs_a_board_that_round_trips() {
        let (g, s) = empty_state();
        let fixture = json!({
            "schema_version": 1,
            "decks": { "1": starter_deck(), "2": starter_deck() },
            "seed": 7,
            "state": { "memory": 2, "phase": "Main", "turn": 4, "first_player": 1 },
            "zones": { "1": { "field": [
                { "stack": ["BT12-022", "BT12-050", "AD1-011"], "is_suspended": false, "turn_played": 0 }
            ] } },
            "assertions": { "engine": [], "ui": [] }
        });

        let dto = stage_into(&g, &s, &fixture).expect("staging must succeed");
        assert!(dto.get("players").is_some(), "DTO must carry the desktop players shape");

        let guard = g.lock().unwrap();
        let game = guard.as_ref().expect("a game must be installed");
        let snap = game.to_scenario();
        assert_eq!(snap["state"]["memory"], 2);
        assert_eq!(snap["state"]["turn"], 4);
        assert_eq!(
            snap["zones"]["1"]["field"][0]["stack"],
            json!(["BT12-022", "BT12-050", "AD1-011"])
        );
    }

    #[test]
    fn stage_into_rejects_an_illegal_board_without_installing() {
        let (g, s) = empty_state();
        let fixture = json!({
            "decks": { "1": starter_deck(), "2": starter_deck() },
            "state": { "memory": 0, "phase": "Main", "turn": 1, "first_player": 1 },
            // An empty field stack is rule-illegal.
            "zones": { "1": { "field": [ { "stack": [], "is_suspended": false, "turn_played": 0 } ] } }
        });
        let err = stage_into(&g, &s, &fixture).unwrap_err();
        assert!(
            err.to_lowercase().contains("empty"),
            "expected an empty-stack diagnostic, got: {err}"
        );
        assert!(
            g.lock().unwrap().is_none(),
            "an illegal stage must not install a game"
        );
    }

    #[test]
    fn stage_into_rejects_a_fixture_without_decks() {
        let (g, s) = empty_state();
        let err = stage_into(&g, &s, &json!({ "zones": {} })).unwrap_err();
        assert!(err.contains("decks"), "expected a decks diagnostic, got: {err}");
    }
}
