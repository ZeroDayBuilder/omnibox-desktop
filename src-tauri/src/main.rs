#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod accel;
mod commands;
mod iso;
mod models;
mod qemu;
mod runtime;
mod store;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            std::fs::create_dir_all(dir.join("drivers"))?;
            app.manage(runtime::AppState::new(dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::host_info,
            commands::list_vms,
            commands::inspect_iso,
            commands::create_vm_from_iso,
            commands::save_vm,
            commands::delete_vm,
            commands::start_vm,
            commands::stop_vm,
            commands::pause_vm,
            commands::resume_vm,
            commands::create_disk,
        ])
        .build(tauri::generate_context!())
        .expect("failed to build OmniBox Desktop")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app.try_state::<runtime::AppState>() {
                    runtime::shutdown_all(&state);
                }
            }
        });
}
