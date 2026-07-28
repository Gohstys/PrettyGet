// PrettyGet — una interfaz bonita para winget
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod pro;
mod schedule;
mod system;
mod winget;

fn main() {
    tauri::Builder::default()
        // Abre enlaces externos (pestaña Donar) en el navegador del sistema.
        .plugin(tauri_plugin_opener::init())
        // PID del proceso winget en curso, para poder abortarlo.
        .manage(winget::RunningJob::default())
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
            winget::cancel_running,
            schedule::create_schedule,
            schedule::list_schedules,
            schedule::delete_schedule,
            schedule::run_schedule_now,
            system::is_elevated,
            system::relaunch_as_admin,
            // --- Advanced ---
            pro::commands::export_state,
            pro::commands::import_state,
            pro::commands::remote_run,
            pro::commands::generate_iac,
            pro::commands::daemon_get_config,
            pro::commands::daemon_exe_path,
            pro::commands::daemon_apply,
            pro::commands::daemon_uninstall
        ])
        .run(tauri::generate_context!())
        .expect("Error al iniciar PrettyGet");
}
