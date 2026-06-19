// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Re-use the modules defined in lib.rs so integration tests can also
// reach them without repeating the `mod` declarations here.
use digimon_tcg::deck_commands;
use digimon_tcg::deck_storage;
use digimon_tcg::engine_commands;
use digimon_tcg::format_commands;
use digimon_tcg::models;
use digimon_tcg::updater;

use std::sync::Arc;

use engine_commands::EngineHandle;
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
    // Spawn the single engine worker thread (64 MB stack) up front; its
    // handle is the only way commands reach the game state. See
    // `engine_commands::EngineHandle`.
    let engine = EngineHandle::spawn();

    tauri::Builder::default()
        // File + stdout logging. Without this, `log::warn!`/`log::info!`
        // (e.g. from the updater) route to a null logger and updater
        // failures are completely silent. Logs land in the OS log dir
        // (e.g. %APPDATA%\com.digimon-tcg.desktop\logs on Windows).
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("digimon-tcg".into()),
                    },
                ))
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        // Restores window position (NOT size — preset selection is the
        // source of truth for size) across launches so multi-monitor
        // users find the window where they left it.
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup({
            let engine = engine.clone();
            move |app| {
                let handle = app.handle().clone();
                updater::spawn_min_version_check(handle);
                // Dev-only staging bridge (feature `debug-bridge`, env-gated).
                // Dispatches its reads/stages through the same engine worker as
                // the `rust_*` commands, so external staging is reflected live
                // in the window.
                #[cfg(feature = "debug-bridge")]
                {
                    digimon_tcg::debug_bridge::maybe_spawn(&app.handle().clone(), engine.clone());
                }
                // Engine handle is unused in `setup` outside the debug bridge,
                // but the move-capture keeps the signature uniform.
                let _ = &engine;
                Ok(())
            }
        })
        .manage(engine.clone())
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
            engine_commands::rust_get_decoded_actions,
            engine_commands::rust_get_board_tensor_summary,
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
            models::models_resolve_starter,
            deck_commands::rust_parse_deck,
            deck_commands::rust_validate_deck_raw,
            deck_commands::rust_list_tested_cards,
            deck_commands::rust_card_database,
            deck_commands::rust_list_formats,
            deck_commands::rust_card_legality,
            deck_commands::rust_card_legality_bulk,
            deck_storage::decks_list,
            deck_storage::decks_get,
            deck_storage::decks_put,
            deck_storage::decks_delete,
            deck_storage::decks_update_library,
            deck_storage::rust_list_starter_decks,
            deck_storage::deck_folders_list,
            deck_storage::deck_folders_create,
            deck_storage::deck_folders_update,
            deck_storage::deck_folders_delete,
            format_commands::formats_list,
            updater::updater_check_info,
            updater::updater_open_download_page,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
