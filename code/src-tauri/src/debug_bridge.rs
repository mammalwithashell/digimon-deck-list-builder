//! Dev-only localhost HTTP bridge into the live desktop game.
//!
//! This entire module is compiled ONLY under the `debug-bridge` cargo
//! feature, so release/production bundles carry no network surface. Even
//! when compiled in, the server starts ONLY if the runtime env var
//! `DIGIMON_DEBUG_BRIDGE=1` is set, and it binds `127.0.0.1` exclusively.
//!
//! It dispatches its reads/stages through the same engine worker
//! ([`EngineHandle`]) as the Tauri `rust_*` commands, so staging/inspection
//! here is reflected live in the window — the worker is the single owner of
//! the game state, there is no shared `Arc<Mutex<Game>>`. After any external
//! mutation it emits a `debug:state-changed` window event so the React
//! frontend re-fetches and re-renders the board.
//!
//! The staging/inspection verbs delegate to the same engine primitives the
//! browser `/debug` path uses (`Game::apply_scenario`, `stage_inject_card`,
//! `set_memory`, `to_scenario`) and return the desktop DTO
//! (`game_state_dto`) — so the MCP exercises the desktop `engine_commands`
//! serialization wire that browser-mode cannot reach.
#![cfg(feature = "debug-bridge")]

use std::net::SocketAddr;

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
    action_mask_bytes, current_decision_player, game_state_dto, EngineHandle, EngineWorld,
    GameSession, PlayerKind,
};

#[derive(Clone)]
struct BridgeState {
    engine: EngineHandle,
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

/// Map an inner worker-closure error string to an HTTP response. The
/// sentinel `"no_game"` (returned when `world.game` is `None`) becomes the
/// canonical [`no_game`] 400; anything else is a plain [`bad`] request.
fn map_game_err(e: String) -> (StatusCode, String) {
    if e == "no_game" {
        no_game()
    } else {
        bad(e)
    }
}

/// Start the bridge if `DIGIMON_DEBUG_BRIDGE=1`. No-op otherwise.
pub fn maybe_spawn(app: &AppHandle, engine: EngineHandle) {
    if std::env::var("DIGIMON_DEBUG_BRIDGE").ok().as_deref() != Some("1") {
        return;
    }
    let port: u16 = std::env::var("DIGIMON_DEBUG_BRIDGE_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5174);

    let state = BridgeState {
        engine,
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
        .route("/navigate", post(navigate))
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
    let value = s
        .engine
        .run(|world| -> Result<Value, String> {
            let game = world.game.as_ref().ok_or("no_game")?;
            Ok(game.to_scenario())
        })
        .await
        .map_err(bad)?
        .map_err(map_game_err)?;
    Ok(Json(value))
}

async fn ui_state(State(s): State<BridgeState>) -> BridgeResult {
    let value = s
        .engine
        .run(|world| -> Result<Value, String> {
            let game = world.game.as_ref().ok_or("no_game")?;
            Ok(json!({
                "state": game_state_dto(game),
                "action_mask": action_mask_bytes(game),
            }))
        })
        .await
        .map_err(bad)?
        .map_err(map_game_err)?;
    Ok(Json(value))
}

async fn mask(State(s): State<BridgeState>) -> BridgeResult {
    let value = s
        .engine
        .run(|world| -> Result<Value, String> {
            let game = world.game.as_ref().ok_or("no_game")?;
            Ok(json!({ "action_mask": action_mask_bytes(game) }))
        })
        .await
        .map_err(bad)?
        .map_err(map_game_err)?;
    Ok(Json(value))
}

async fn export_scenario(State(s): State<BridgeState>) -> BridgeResult {
    let value = s
        .engine
        .run(|world| -> Result<Value, String> {
            let game = world.game.as_ref().ok_or("no_game")?;
            Ok(game.to_scenario())
        })
        .await
        .map_err(bad)?
        .map_err(map_game_err)?;
    Ok(Json(value))
}

// ─── Mutating endpoints ──────────────────────────────────────────────────

/// Construct a fresh game from a scenario fixture's decks, apply the
/// fixture, and install it as the live desktop game. The whole stage→test
/// loop's "set up a board without playing" entry point.
async fn stage(State(s): State<BridgeState>, Json(fixture): Json<Value>) -> BridgeResult {
    let dto = s
        .engine
        .run(move |world| stage_into(world, &fixture))
        .await
        .map_err(bad)?
        .map_err(bad)?;
    notify(&s);
    Ok(Json(json!({ "state": dto })))
}

/// Pure core of `/stage` (no `AppHandle`), so it is unit-testable without a
/// Tauri runtime. Builds a game from the fixture's decks, stages the
/// fixture onto it, and installs it into the worker's [`EngineWorld`].
/// Returns the desktop `GameStateDto` as a `Value`.
pub(crate) fn stage_into(world: &mut EngineWorld, fixture: &Value) -> Result<Value, String> {
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

    world.game = Some(game);
    world.session = GameSession {
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
    let dto = s
        .engine
        .run(move |world| -> Result<Value, String> {
            let game = world.game.as_mut().ok_or("no_game")?;
            game.apply_scenario(&fixture)?;
            serde_json::to_value(game_state_dto(game)).map_err(|e| e.to_string())
        })
        .await
        .map_err(bad)?
        .map_err(map_game_err)?;
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
    let dto = s
        .engine
        .run(move |world| -> Result<Value, String> {
            let game = world.game.as_mut().ok_or("no_game")?;
            game.stage_inject_card(pid, &b.card_id, &b.zone)?;
            serde_json::to_value(game_state_dto(game)).map_err(|e| e.to_string())
        })
        .await
        .map_err(bad)?
        .map_err(map_game_err)?;
    notify(&s);
    Ok(Json(json!({ "state": dto })))
}

#[derive(Deserialize)]
struct SetMemoryBody {
    memory: i16,
}

async fn set_memory(State(s): State<BridgeState>, Json(b): Json<SetMemoryBody>) -> BridgeResult {
    let dto = s
        .engine
        .run(move |world| -> Result<Value, String> {
            let game = world.game.as_mut().ok_or("no_game")?;
            game.set_memory(b.memory);
            serde_json::to_value(game_state_dto(game)).map_err(|e| e.to_string())
        })
        .await
        .map_err(bad)?
        .map_err(map_game_err)?;
    notify(&s);
    Ok(Json(json!({ "state": dto })))
}

#[derive(Deserialize)]
struct StepBody {
    action: u16,
    player_id: Option<u8>,
}

async fn step(State(s): State<BridgeState>, Json(b): Json<StepBody>) -> BridgeResult {
    let dto = s
        .engine
        .run(move |world| -> Result<Value, String> {
            let game = world.game.as_mut().ok_or("no_game")?;
            let pid = match b.player_id {
                Some(p) => p
                    .checked_sub(1)
                    .ok_or_else(|| "player_id must be 1/2".to_string())?,
                None => current_decision_player(game),
            };
            let mask = action_mask_bytes(game);
            if (b.action as usize) >= mask.len() || mask[b.action as usize] != 1 {
                return Err(format!("action {} is not legal", b.action));
            }
            game.decode_action(b.action, pid);
            serde_json::to_value(game_state_dto(game)).map_err(|e| e.to_string())
        })
        .await
        .map_err(bad)?
        .map_err(map_game_err)?;
    notify(&s);
    Ok(Json(json!({ "state": dto })))
}

#[derive(Deserialize)]
struct NavigateBody {
    route: String,
    theme: Option<String>,
}

/// Dev-only: drive the desktop window's client-side router (+ optional theme)
/// for the screenshot skill. Emits a `debug:navigate` window event the React
/// `DebugBridgeNav` listener consumes; the engine state is untouched.
async fn navigate(State(s): State<BridgeState>, Json(b): Json<NavigateBody>) -> BridgeResult {
    s.app
        .emit("debug:navigate", json!({ "route": b.route, "theme": b.theme }))
        .map_err(bad)?;
    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn navigate_body_parses_route_and_optional_theme() {
        let b: NavigateBody =
            serde_json::from_value(json!({ "route": "/deckbuilder", "theme": "dark" })).unwrap();
        assert_eq!(b.route, "/deckbuilder");
        assert_eq!(b.theme.as_deref(), Some("dark"));

        let b2: NavigateBody = serde_json::from_value(json!({ "route": "/" })).unwrap();
        assert_eq!(b2.route, "/");
        assert!(b2.theme.is_none(), "theme is optional");
    }

    #[test]
    fn hero_fixture_stages_into_a_legal_board() {
        let raw = include_str!(
            "../../../.claude/skills/update-landing-screenshots/fixtures/hero-board.json"
        );
        let fixture: serde_json::Value = serde_json::from_str(raw).expect("fixture is valid JSON");
        let mut world = EngineWorld::default();
        let dto = stage_into(&mut world, &fixture).expect("hero fixture must stage legally");
        assert!(dto.get("players").is_some());
        assert!(world.game.is_some(), "a game must be installed");
    }

    fn starter_deck() -> Vec<String> {
        let mut d: Vec<String> = vec!["ST1-01".to_string(); 5];
        d.extend(std::iter::repeat("ST1-03".to_string()).take(45));
        d
    }

    #[test]
    fn stage_into_installs_a_board_that_round_trips() {
        let mut world = EngineWorld::default();
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

        let dto = stage_into(&mut world, &fixture).expect("staging must succeed");
        assert!(dto.get("players").is_some(), "DTO must carry the desktop players shape");

        let game = world.game.as_ref().expect("a game must be installed");
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
        let mut world = EngineWorld::default();
        let fixture = json!({
            "decks": { "1": starter_deck(), "2": starter_deck() },
            "state": { "memory": 0, "phase": "Main", "turn": 1, "first_player": 1 },
            // An empty field stack is rule-illegal.
            "zones": { "1": { "field": [ { "stack": [], "is_suspended": false, "turn_played": 0 } ] } }
        });
        let err = stage_into(&mut world, &fixture).unwrap_err();
        assert!(
            err.to_lowercase().contains("empty"),
            "expected an empty-stack diagnostic, got: {err}"
        );
        assert!(
            world.game.is_none(),
            "an illegal stage must not install a game"
        );
    }

    #[test]
    fn stage_into_rejects_a_fixture_without_decks() {
        let mut world = EngineWorld::default();
        let err = stage_into(&mut world, &json!({ "zones": {} })).unwrap_err();
        assert!(err.contains("decks"), "expected a decks diagnostic, got: {err}");
    }
}
