// PrettyGet — una interfaz bonita para winget
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pro;
mod schedule;
mod system;
mod winget;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        // Estado global compartido (nivel de licencia / entitlements).
        .manage(pro::AppState::new())
        .setup(|app| {
            // Carga y valida la licencia guardada al arrancar (modo Free si falla).
            let state = app.state::<pro::AppState>();
            pro::commands::load_on_startup(&state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // --- Free (core) ---
            winget::list_upgrades,
            winget::upgrade_package,
            winget::upgrade_all,
            winget::winget_available,
            winget::search_packages,
            winget::list_installed,
            winget::install_package,
            winget::uninstall_package,
            schedule::create_schedule,
            schedule::list_schedules,
            schedule::delete_schedule,
            schedule::run_schedule_now,
            system::is_elevated,
            system::relaunch_as_admin,
            // --- Licencia / entitlements ---
            pro::commands::activate_license,
            pro::commands::deactivate_license,
            pro::commands::get_entitlements,
            pro::commands::hardware_id,
            // --- Pro (con feature gating dentro de cada comando) ---
            pro::commands::export_state,
            pro::commands::import_state,
            pro::commands::remote_run,
            pro::commands::generate_iac,
            pro::commands::daemon_get_config,
            pro::commands::daemon_apply,
            pro::commands::daemon_uninstall
        ])
        .run(tauri::generate_context!())
        .expect("Error al iniciar PrettyGet");
}
