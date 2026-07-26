// PrettyGet Update Daemon — servicio de Windows.
//
// Corre como LocalSystem (sin UAC, sin sesión iniciada) y ejecuta
// `winget upgrade --all` (o solo los Ids configurados) según una ventana
// horaria. La app PrettyGet Pro lo instala/configura vía `sc.exe` y escribe
// %ProgramData%\PrettyGet\daemon.json.
//
// Compilar:  cd tools/prettyget-daemon && cargo build --release
// El binario resultante es el que se pasa a `daemon_apply` desde la app.

#![cfg(windows)]

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use chrono::{Datelike, Local, Timelike, Weekday};
use serde::Deserialize;

use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::{define_windows_service, service_dispatcher};

const SERVICE_NAME: &str = "PrettyGetDaemon";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Deserialize)]
struct DaemonConfig {
    #[serde(default = "default_freq")]
    frequency: String,
    #[serde(default = "default_time")]
    time: String,
    #[serde(default)]
    only: Vec<String>,
    #[serde(default)]
    enabled: bool,
}
fn default_freq() -> String { "daily".into() }
fn default_time() -> String { "03:00".into() }

fn data_dir() -> PathBuf {
    let base = std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    base.join("PrettyGet")
}
fn config_path() -> PathBuf { data_dir().join("daemon.json") }
fn log_path() -> PathBuf { data_dir().join("daemon.log") }

fn log(msg: &str) {
    use std::io::Write;
    let _ = std::fs::create_dir_all(data_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_path()) {
        let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(f, "[{ts}] {msg}");
    }
}

fn read_config() -> Option<DaemonConfig> {
    let raw = std::fs::read_to_string(config_path()).ok()?;
    serde_json::from_str(&raw).ok()
}

/// ¿Toca ejecutar ahora? Evita repetir dentro del mismo minuto con `last`.
fn due_now(cfg: &DaemonConfig, last: &mut String) -> bool {
    let now = Local::now();
    if now.format("%H:%M").to_string() != cfg.time {
        return false;
    }
    let ok_day = match cfg.frequency.as_str() {
        "weekly" => now.weekday() == Weekday::Mon,
        "monthly" => now.day() == 1,
        _ => true, // daily
    };
    if !ok_day {
        return false;
    }
    let stamp = now.format("%Y-%m-%d %H:%M").to_string();
    if *last == stamp {
        return false;
    }
    *last = stamp;
    true
}

fn run_winget(cfg: &DaemonConfig) {
    let base_flags = [
        "--silent",
        "--accept-source-agreements",
        "--accept-package-agreements",
        "--include-unknown",
        "--disable-interactivity",
    ];
    if cfg.only.is_empty() {
        log("Ejecutando: winget upgrade --all");
        let out = Command::new("winget")
            .arg("upgrade")
            .arg("--all")
            .args(base_flags)
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        match out {
            Ok(o) => log(&format!("upgrade --all → código {}", o.status.code().unwrap_or(-1))),
            Err(e) => log(&format!("error winget: {e}")),
        }
    } else {
        for id in &cfg.only {
            // Filtra Ids con espacios por seguridad.
            if id.chars().any(|c| c.is_whitespace()) {
                continue;
            }
            log(&format!("Ejecutando: winget upgrade --id {id}"));
            let out = Command::new("winget")
                .args(["upgrade", "--id", id, "--exact"])
                .args(base_flags)
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            match out {
                Ok(o) => log(&format!("upgrade {id} → código {}", o.status.code().unwrap_or(-1))),
                Err(e) => log(&format!("error winget {id}: {e}")),
            }
        }
    }
}

define_windows_service!(ffi_service_main, service_main);

fn main() -> windows_service::Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

fn service_main(_args: Vec<OsString>) {
    if let Err(e) = run_service() {
        log(&format!("fallo del servicio: {e}"));
    }
}

fn run_service() -> windows_service::Result<()> {
    let (tx, rx) = mpsc::channel();

    let event_handler = move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = tx.send(());
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    let mk_status = |state: ServiceState| ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    status_handle.set_service_status(mk_status(ServiceState::Running))?;
    log("Servicio iniciado.");

    let mut last_run = String::new();
    loop {
        if let Some(cfg) = read_config() {
            if cfg.enabled && due_now(&cfg, &mut last_run) {
                run_winget(&cfg);
            }
        }
        // Despierta cada 60 s o sale al recibir Stop.
        match rx.recv_timeout(Duration::from_secs(60)) {
            Ok(_) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        }
    }

    log("Servicio detenido.");
    status_handle.set_service_status(mk_status(ServiceState::Stopped))?;
    Ok(())
}
