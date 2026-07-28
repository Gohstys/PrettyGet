// PRO · Comandos de Tauri para las funciones avanzadas: State Sync, Remote
// Deploy, IaC Generator y Silent Daemon. Todas libres, sin ningún tipo de
// licencia ni comprobación previa.

use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

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

/// Localiza el `prettyget-daemon.exe` que se distribuye DENTRO del instalador
/// (declarado en `bundle.resources` de tauri.conf.json), para que el usuario no
/// tenga que descargarlo aparte ni pegar rutas a mano.
///
/// En una instalación real vive en el directorio de recursos, junto al .exe de la
/// app. En `tauri dev` ese directorio no existe todavía, así que se cae al sitio
/// donde `cargo build --release` deja el binario del crate del daemon.
fn bundled_daemon_exe(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    if let Ok(p) = app
        .path()
        .resolve("prettyget-daemon.exe", BaseDirectory::Resource)
    {
        if p.exists() {
            return Ok(p);
        }
    }
    // Fallback de desarrollo: repo/tools/prettyget-daemon/target/release/...
    let dev = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("tools")
        .join("prettyget-daemon")
        .join("target")
        .join("release")
        .join("prettyget-daemon.exe");
    if dev.exists() {
        return Ok(dev);
    }
    Err("No se encontró prettyget-daemon.exe. En desarrollo, compílalo antes con: \
         cargo build --release --manifest-path tools/prettyget-daemon/Cargo.toml"
        .into())
}

/// El frontend lo usa solo para mostrar qué binario se instalaría.
#[tauri::command]
pub fn daemon_exe_path(app: AppHandle) -> Result<String, String> {
    Ok(bundled_daemon_exe(&app)?.to_string_lossy().to_string())
}

#[tauri::command]
pub fn daemon_get_config() -> Result<DaemonConfig, String> {
    ScDaemonControl.read_config()
}

#[tauri::command]
pub fn daemon_apply(app: AppHandle, config: DaemonConfig) -> Result<(), String> {
    let exe = bundled_daemon_exe(&app)?;
    let ctl = ScDaemonControl;
    ctl.write_config(&config)?;
    // Instala si hace falta y arranca/para según `enabled`.
    let _ = ctl.install(&exe.to_string_lossy()); // idempotente: si ya existe, ignora el error
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
