// PRO · SilentDaemon — servicio en segundo plano para actualizaciones silenciosas.
//
// Arquitectura: el SERVICIO en sí es un binario aparte (`tools/prettyget-daemon`,
// implementado con el crate `windows-service`) que corre como LocalSystem y
// ejecuta `winget upgrade --all` según una configuración. Este módulo es el
// CONTROLADOR del lado de la app: instala/desinstala el servicio (sc.exe) y
// escribe la configuración que el servicio lee de ProgramData.
//
// ¿Por qué un servicio y no solo el Programador de tareas (que ya usa la versión
// Free)? Porque LocalSystem evita el UAC por completo y funciona sin sesión
// iniciada — requisito típico de la edición Team/Enterprise.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const SERVICE_NAME: &str = "PrettyGetDaemon";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// "daily" | "weekly" | "monthly"
    pub frequency: String,
    /// "HH:MM" 24h
    pub time: String,
    /// Solo actualizar estos Ids (vacío = todos).
    #[serde(default)]
    pub only: Vec<String>,
    pub enabled: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            frequency: "daily".into(),
            time: "03:00".into(),
            only: vec![],
            enabled: true,
        }
    }
}

/// Ruta de configuración compartida con el servicio (legible por LocalSystem).
pub fn config_path() -> PathBuf {
    let base = std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    base.join("PrettyGet").join("daemon.json")
}

/// Contrato del controlador del daemon (testeable con un mock).
pub trait DaemonControl {
    fn write_config(&self, cfg: &DaemonConfig) -> Result<(), String>;
    fn read_config(&self) -> Result<DaemonConfig, String>;
    fn install(&self, daemon_exe: &str) -> Result<(), String>;
    fn uninstall(&self) -> Result<(), String>;
    fn start(&self) -> Result<(), String>;
    fn stop(&self) -> Result<(), String>;
}

pub struct ScDaemonControl;

fn sc() -> Command {
    let mut c = Command::new("sc.exe");
    #[cfg(windows)]
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

fn run_ok(mut cmd: Command) -> Result<(), String> {
    let out = cmd.output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

impl DaemonControl for ScDaemonControl {
    fn write_config(&self, cfg: &DaemonConfig) -> Result<(), String> {
        let path = config_path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }

    fn read_config(&self) -> Result<DaemonConfig, String> {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
            Err(_) => Ok(DaemonConfig::default()),
        }
    }

    fn install(&self, daemon_exe: &str) -> Result<(), String> {
        // Crea el servicio en modo auto-arranque, como LocalSystem.
        run_ok({
            let mut c = sc();
            c.args([
                "create",
                SERVICE_NAME,
                &format!("binPath= {daemon_exe}"),
                "start=",
                "auto",
                "DisplayName=",
                "PrettyGet Update Daemon",
            ]);
            c
        })
    }

    fn uninstall(&self) -> Result<(), String> {
        let _ = self.stop();
        run_ok({
            let mut c = sc();
            c.args(["delete", SERVICE_NAME]);
            c
        })
    }

    fn start(&self) -> Result<(), String> {
        run_ok({
            let mut c = sc();
            c.args(["start", SERVICE_NAME]);
            c
        })
    }

    fn stop(&self) -> Result<(), String> {
        run_ok({
            let mut c = sc();
            c.args(["stop", SERVICE_NAME]);
            c
        })
    }
}
