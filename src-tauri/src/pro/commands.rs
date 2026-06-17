// PRO · Comandos de Tauri: activación de licencia + endpoints con feature gating.
// Todos los comandos Pro empiezan con `require(&state, Feature::X)?`.

use std::path::PathBuf;
use tauri::State;

use super::daemon::{DaemonConfig, DaemonControl, ScDaemonControl};
use super::entitlements::{require, AppState, EntitlementsView};
use super::iac::{DefaultIac, IacGenerator, Selection};
use super::license::Feature;
use super::remote_deploy::{PsRemotingExecutor, RemoteExecutor, RemoteHost, RemoteResult};
use super::state_sync::{StateSync, WingetState, WingetStateSync};

// ---------- Persistencia de la licencia ----------

fn license_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    let mut p = PathBuf::from(base);
    p.push("PrettyGet");
    std::fs::create_dir_all(&p).ok()?;
    p.push("license.key");
    Some(p)
}

fn current_hwid() -> Option<String> {
    super::hwid::hardware_id()
}

/// Se llama al arrancar la app (desde el setup de Tauri).
pub fn load_on_startup(state: &AppState) {
    if let Some(path) = license_path() {
        if let Ok(token) = std::fs::read_to_string(&path) {
            let _ = state.load_token(token.trim(), current_hwid().as_deref());
        }
    }
}

// ---------- Comandos de licencia ----------

#[tauri::command]
pub fn activate_license(state: State<AppState>, token: String) -> Result<EntitlementsView, String> {
    state.load_token(token.trim(), current_hwid().as_deref())?;
    if let Some(path) = license_path() {
        std::fs::write(path, token.trim()).map_err(|e| e.to_string())?;
    }
    Ok(state.view())
}

#[tauri::command]
pub fn deactivate_license(state: State<AppState>) -> Result<EntitlementsView, String> {
    state.clear();
    if let Some(path) = license_path() {
        let _ = std::fs::remove_file(path);
    }
    Ok(state.view())
}

/// El frontend llama a esto para saber qué UI mostrar.
#[tauri::command]
pub fn get_entitlements(state: State<AppState>) -> EntitlementsView {
    state.view()
}

#[tauri::command]
pub fn hardware_id() -> Option<String> {
    current_hwid()
}

// ---------- StateSync ----------

#[tauri::command]
pub fn export_state(state: State<AppState>, format: String) -> Result<String, String> {
    require(&state, Feature::StateSync)?;
    let sync = WingetStateSync;
    match format.as_str() {
        "yaml" => sync.export_yaml(),
        _ => sync.export_json(),
    }
}

#[tauri::command]
pub fn import_state(state: State<AppState>, data: String, silent: bool) -> Result<i32, String> {
    require(&state, Feature::StateSync)?;
    WingetStateSync.import_str(&data, silent)
}

// ---------- RemoteDeploy ----------

#[tauri::command]
pub fn remote_run(
    state: State<AppState>,
    hosts: Vec<RemoteHost>,
    winget_args: Vec<String>,
) -> Result<Vec<RemoteResult>, String> {
    require(&state, Feature::RemoteDeploy)?;
    let refs: Vec<&str> = winget_args.iter().map(|s| s.as_str()).collect();
    Ok(PsRemotingExecutor.run_many(&hosts, &refs))
}

// ---------- IaC ----------

#[tauri::command]
pub fn generate_iac(
    state: State<AppState>,
    selection: Selection,
    target: String,
) -> Result<String, String> {
    require(&state, Feature::IacGenerator)?;
    let gen = DefaultIac;
    Ok(match target.as_str() {
        "ansible" => gen.ansible(&selection),
        _ => gen.powershell(&selection),
    })
}

// ---------- SilentDaemon ----------

#[tauri::command]
pub fn daemon_get_config(state: State<AppState>) -> Result<DaemonConfig, String> {
    require(&state, Feature::SilentDaemon)?;
    ScDaemonControl.read_config()
}

#[tauri::command]
pub fn daemon_apply(
    state: State<AppState>,
    config: DaemonConfig,
    daemon_exe: String,
) -> Result<(), String> {
    require(&state, Feature::SilentDaemon)?;
    let ctl = ScDaemonControl;
    ctl.write_config(&config)?;
    // Instala si hace falta y arranca/para según `enabled`.
    let _ = ctl.install(&daemon_exe); // idempotente: si ya existe, ignora el error
    if config.enabled {
        ctl.start()
    } else {
        ctl.stop()
    }
}

#[tauri::command]
pub fn daemon_uninstall(state: State<AppState>) -> Result<(), String> {
    require(&state, Feature::SilentDaemon)?;
    ScDaemonControl.uninstall()
}

/// Lista de handlers Pro para registrar en `generate_handler!` desde main.rs.
/// (Tauri no permite concatenar listas, así que se enumeran también en main.rs;
/// este comentario sirve de referencia de qué exponer.)
pub mod handler_names {}
