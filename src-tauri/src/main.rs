// Prevents additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;
use tauri_plugin_shell::ShellExt;

/// State to hold the sidecar process handle for cleanup.
struct SidecarState {
    child: tauri_plugin_shell::process::CommandChild,
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Resolve models directory from bundled resources
            let resource_dir = app
                .path()
                .resource_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let models_dir = resource_dir.join("resources").join("models");
            let models_dir_str = models_dir.to_string_lossy().to_string();

            // Spawn the Python sidecar (game engine only, no DB/auth)
            let (mut rx, child) = app
                .shell()
                .sidecar("digimon-server")
                .expect("failed to create sidecar command")
                .args(["--port", "8321", "--models-dir", &models_dir_str])
                .spawn()
                .expect("failed to spawn sidecar");

            // Log sidecar output for debugging
            tauri::async_runtime::spawn(async move {
                use tauri_plugin_shell::process::CommandEvent;
                while let Some(event) = rx.recv().await {
                    match event {
                        CommandEvent::Stdout(line) => {
                            println!("[sidecar stdout] {}", String::from_utf8_lossy(&line));
                        }
                        CommandEvent::Stderr(line) => {
                            eprintln!("[sidecar stderr] {}", String::from_utf8_lossy(&line));
                        }
                        CommandEvent::Terminated(payload) => {
                            eprintln!("[sidecar] terminated: {:?}", payload);
                            break;
                        }
                        _ => {}
                    }
                }
            });

            // Store the child handle so we can kill it on shutdown
            app.manage(SidecarState { child });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Kill the sidecar when the window closes
                if let Some(state) = window.try_state::<SidecarState>() {
                    let _ = state.child.kill();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
