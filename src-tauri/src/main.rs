// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine_commands;

#[cfg(not(feature = "no-sidecar"))]
use std::sync::{Arc, Mutex};
#[cfg(not(feature = "no-sidecar"))]
use tauri::Manager;
#[cfg(not(feature = "no-sidecar"))]
use tauri_plugin_shell::ShellExt;

use engine_commands::RustEngineState;

/// State to hold the sidecar process handle for cleanup and the discovered port.
#[cfg(not(feature = "no-sidecar"))]
struct SidecarState {
    child: Arc<Mutex<Option<tauri_plugin_shell::process::CommandChild>>>,
    port: Arc<Mutex<Option<u16>>>,
}

/// Max number of automatic sidecar respawn attempts.
#[cfg(not(feature = "no-sidecar"))]
const MAX_RESPAWN_RETRIES: u32 = 5;

/// Returns the Python sidecar's port.
#[cfg(not(feature = "no-sidecar"))]
#[tauri::command]
fn get_sidecar_port(state: tauri::State<'_, SidecarState>) -> Option<u16> {
    *state.port.lock().unwrap()
}

/// In `no-sidecar` builds the Python sidecar never runs, so the frontend
/// falls back to Tauri `invoke()` commands on `None`.
#[cfg(feature = "no-sidecar")]
#[tauri::command]
fn get_sidecar_port() -> Option<u16> {
    None
}

fn main() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(RustEngineState::default())
        .invoke_handler(tauri::generate_handler![
            get_sidecar_port,
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
        ]);

    #[cfg(not(feature = "no-sidecar"))]
    let builder = spawn_sidecar(builder);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(not(feature = "no-sidecar"))]
fn spawn_sidecar(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder
        .setup(|app| {
            // Resolve models directory from bundled resources
            let resource_dir = app
                .path()
                .resource_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let models_dir = resource_dir.join("resources").join("models");
            let models_dir_str = models_dir.to_string_lossy().to_string();

            let port_state: Arc<Mutex<Option<u16>>> = Arc::new(Mutex::new(None));
            let child_state: Arc<Mutex<Option<tauri_plugin_shell::process::CommandChild>>> =
                Arc::new(Mutex::new(None));

            let app_handle = app.handle().clone();

            // Spawn the Python sidecar (game engine only, no DB/auth)
            let (rx, child) = app_handle
                .shell()
                .sidecar("digimon-server")
                .expect("failed to create sidecar command")
                .args(["--port", "8321", "--models-dir", &models_dir_str])
                .spawn()
                .expect("failed to spawn sidecar");

            *child_state.lock().unwrap() = Some(child);

            // Clone handles for the async task
            let port_clone = Arc::clone(&port_state);
            let child_clone = Arc::clone(&child_state);
            let handle_clone = app_handle.clone();
            let models_dir_clone = models_dir_str.clone();

            // Log sidecar output, parse port, and handle respawn
            tauri::async_runtime::spawn(async move {
                use tauri_plugin_shell::process::CommandEvent;

                let mut rx = rx;
                let mut retries: u32 = 0;

                loop {
                    match rx.recv().await {
                        Some(CommandEvent::Stdout(line)) => {
                            let text = String::from_utf8_lossy(&line);
                            println!("[sidecar stdout] {}", text);
                            // Parse port announcement from sidecar
                            if let Some(port_str) = text.trim().strip_prefix("SIDECAR_PORT=") {
                                if let Ok(port) = port_str.parse::<u16>() {
                                    *port_clone.lock().unwrap() = Some(port);
                                    retries = 0; // Reset retries on successful start
                                }
                            }
                        }
                        Some(CommandEvent::Stderr(line)) => {
                            eprintln!(
                                "[sidecar stderr] {}",
                                String::from_utf8_lossy(&line)
                            );
                        }
                        Some(CommandEvent::Terminated(payload)) => {
                            eprintln!("[sidecar] terminated: {:?}", payload);
                            retries += 1;
                            if retries > MAX_RESPAWN_RETRIES {
                                eprintln!(
                                    "[sidecar] max respawn retries ({}) reached, giving up",
                                    MAX_RESPAWN_RETRIES
                                );
                                break;
                            }
                            // Wait before respawn
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                            eprintln!("[sidecar] respawning (attempt {}/{})", retries, MAX_RESPAWN_RETRIES);
                            match handle_clone
                                .shell()
                                .sidecar("digimon-server")
                                .expect("failed to create sidecar command")
                                .args(["--port", "8321", "--models-dir", &models_dir_clone])
                                .spawn()
                            {
                                Ok((new_rx, new_child)) => {
                                    *child_clone.lock().unwrap() = Some(new_child);
                                    *port_clone.lock().unwrap() = None; // Reset until new port announced
                                    rx = new_rx;
                                    eprintln!("[sidecar] respawned successfully");
                                }
                                Err(e) => {
                                    eprintln!("[sidecar] respawn failed: {}", e);
                                    break;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            });

            // Store state for cleanup and port queries
            app.manage(SidecarState {
                child: child_state,
                port: port_state,
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Kill the sidecar when the window closes
                if let Some(state) = window.try_state::<SidecarState>() {
                    if let Some(child) = state.child.lock().unwrap().take() {
                        let _ = child.kill();
                    }
                }
            }
        })
}
