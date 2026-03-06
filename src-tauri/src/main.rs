// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri_plugin_shell::ShellExt;

/// State to hold the sidecar process handle for cleanup and the discovered port.
struct SidecarState {
    child: Arc<Mutex<Option<tauri_plugin_shell::process::CommandChild>>>,
    port: Arc<Mutex<Option<u16>>>,
}

/// Max number of automatic sidecar respawn attempts.
const MAX_RESPAWN_RETRIES: u32 = 5;

#[tauri::command]
fn get_sidecar_port(state: tauri::State<'_, SidecarState>) -> Option<u16> {
    *state.port.lock().unwrap()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![get_sidecar_port])
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

            // Spawn the Python sidecar (game engine only, no DB/auth)
            let (rx, child) = app
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
            let shell_handle = app.shell().clone();
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
                            match shell_handle
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
                    if let Some(ref child) = *state.child.lock().unwrap() {
                        let _ = child.kill();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
