// PrettyGet — una interfaz bonita para winget
// Evita que se abra una consola en la versión release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod schedule;
mod winget;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
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
            schedule::run_schedule_now
        ])
        .run(tauri::generate_context!())
        .expect("Error al iniciar PrettyGet");
}
