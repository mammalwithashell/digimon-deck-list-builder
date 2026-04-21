// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod deck_commands;
mod deck_storage;
mod engine_commands;
mod inference_state;
mod models;
mod updater;

use std::sync::Arc;

use engine_commands::RustEngineState;
use inference_state::InferenceState;
use models::ModelsManager;

/// Where the model cache lives. `dirs::data_dir()` resolves to the standard
/// OS location (e.g. `%APPDATA%\digimon-tcg` on Windows, `~/.local/share/
/// digimon-tcg` on Linux, `~/Library/Application Support/digimon-tcg` on
/// macOS). Falls back to the working dir if the OS can't answer — better to
/// degrade to a local cache than crash on startup.
fn models_cache_root() -> std::path::PathBuf {
    dirs::data_dir()
        .map(|d| d.join("digimon-tcg"))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let handle = app.handle().clone();
            updater::spawn_min_version_check(handle);
            Ok(())
        })
        .manage(RustEngineState::default())
        .manage(InferenceState::default())
        .manage(Arc::new(ModelsManager::new(models_cache_root())))
        .invoke_handler(tauri::generate_handler![
            engine_commands::create_test_game,
            engine_commands::get_rust_game_state,
            engine_commands::rust_play_card,
            engine_commands::rust_attack_digimon,
            engine_commands::rust_attack_player,
            engine_commands::rust_end_turn,
            engine_commands::rust_pass_turn,
            engine_commands::rust_hatch,
            engine_commands::rust_move_from_breeding,
            engine_commands::rust_mulligan_decide,
            engine_commands::rust_create_game,
            engine_commands::rust_submit_action,
            engine_commands::rust_step_game,
            engine_commands::rust_get_mask,
            engine_commands::rust_get_log,
            engine_commands::rust_surrender,
            engine_commands::rust_delete_game,
            engine_commands::rust_load_model,
            engine_commands::rust_unload_model,
            engine_commands::rust_list_loaded_models,
            models::models_engine_contract,
            models::models_fetch_manifest,
            models::models_list_local,
            models::models_download,
            models::models_delete,
            models::models_load_cached,
            deck_commands::rust_parse_deck,
            deck_commands::rust_validate_deck_raw,
            deck_commands::rust_list_tested_cards,
            deck_storage::decks_list,
            deck_storage::decks_get,
            deck_storage::decks_put,
            deck_storage::decks_delete,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
