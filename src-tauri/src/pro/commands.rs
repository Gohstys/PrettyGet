// PRO · Comandos de Tauri para las funciones avanzadas: State Sync, Remote
// Deploy, IaC Generator y Silent Daemon. Todas libres, sin ningún tipo de
// licencia ni comprobación previa.

use super::daemon::{DaemonConfig, DaemonControl, ScDaemonControl};
use super::iac::{DefaultIac, IacGenerator, Selection};
use super::remote_deploy::{PsRemotingExecutor, RemoteExecutor, RemoteHost, RemoteResult};
use super::state_sync::{StateSync, WingetStateSync};

// ---------- StateSync ----------

#[tauri::command]
pub fn export_state(format: String) -> Result<String, String> {
    let sync = WingetStateSync;
    match format.as_str() {
        "yaml" => sync.export_yaml(),
        _ => sync.export_json(),
    }
}

#[tauri::command]
pub fn import_state(data: String, silent: bool) -> Result<i32, String> {
    WingetStateSync.import_str(&data, silent)
}

// ---------- RemoteDeploy ----------

#[tauri::command]
pub fn remote_run(hosts: Vec<RemoteHost>, winget_args: Vec<String>) -> Result<Vec<RemoteResult>, String> {
    let refs: Vec<&str> = winget_args.iter().map(|s| s.as_str()).collect();
    Ok(PsRemotingExecutor.run_many(&hosts, &refs))
}

// ---------- IaC ----------

#[tauri::command]
pub fn generate_iac(selection: Selection, target: String) -> Result<String, String> {
    let gen = DefaultIac;
    Ok(match target.as_str() {
        "ansible" => gen.ansible(&selection),
        _ => gen.powershell(&selection),
    })
}

// ---------- SilentDaemon ----------

#[tauri::command]
pub fn daemon_get_config() -> Result<DaemonConfig, String> {
    ScDaemonControl.read_config()
}

#[tauri::command]
pub fn daemon_apply(config: DaemonConfig, daemon_exe: String) -> Result<(), String> {
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
pub fn daemon_uninstall() -> Result<(), String> {
    ScDaemonControl.uninstall()
}
