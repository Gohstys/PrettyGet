// Programación de actualizaciones automáticas mediante el Programador de tareas
// de Windows (schtasks). Cada tarea ejecuta `winget upgrade --all` en silencio.

use serde::{Deserialize, Serialize};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const PREFIX: &str = "PrettyGet_";

/// El comando que ejecutará la tarea programada.
const TASK_RUN: &str = "winget upgrade --all --silent --accept-source-agreements --accept-package-agreements --include-unknown --disable-interactivity";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScheduledTask {
    pub name: String,
    pub frequency: String,
    pub time: String,
    pub next_run: String,
    pub status: String,
}

fn schtasks() -> Command {
    let mut cmd = Command::new("schtasks");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Crea (o reemplaza) una tarea programada de actualización.
/// frequency: "daily" | "weekly" | "monthly"   time: "HH:MM" (24h)
#[tauri::command]
pub fn create_schedule(name: String, frequency: String, time: String) -> Result<String, String> {
    let clean: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if clean.is_empty() {
        return Err("El nombre no puede estar vacío.".into());
    }
    let task_name = format!("{PREFIX}{clean}");

    let sc = match frequency.as_str() {
        "daily" => "DAILY",
        "weekly" => "WEEKLY",
        "monthly" => "MONTHLY",
        other => return Err(format!("Frecuencia no válida: {other}")),
    };

    let output = schtasks()
        .args([
            "/Create", "/TN", &task_name, "/TR", TASK_RUN, "/SC", sc, "/ST", &time, "/F",
        ])
        .output()
        .map_err(|e| format!("No se pudo ejecutar schtasks: {e}"))?;

    if output.status.success() {
        Ok(format!("Tarea «{clean}» programada ({frequency} a las {time})."))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Lista las tareas programadas creadas por PrettyGet.
#[tauri::command]
pub fn list_schedules() -> Result<Vec<ScheduledTask>, String> {
    let output = schtasks()
        .args(["/Query", "/FO", "CSV", "/V", "/NH"])
        .output()
        .map_err(|e| format!("No se pudo ejecutar schtasks: {e}"))?;

    let text = String::from_utf8_lossy(&output.stdout);
    let mut tasks = Vec::new();

    for line in text.lines() {
        let fields = parse_csv_line(line);
        if fields.is_empty() {
            continue;
        }
        // El campo 1 (TaskName) viene como "\PrettyGet_xxx".
        let raw_name = fields.get(1).map(|s| s.as_str()).unwrap_or("");
        let task_name = raw_name.trim_start_matches('\\');
        if !task_name.starts_with(PREFIX) {
            continue;
        }
        let display = task_name.trim_start_matches(PREFIX).to_string();
        let next_run = fields.get(2).cloned().unwrap_or_default();
        let status = fields.get(3).cloned().unwrap_or_default();
        // "Schedule Type" e "Start Time" dependen de la versión; mostramos lo disponible.
        let frequency = fields.get(8).cloned().unwrap_or_default();
        let time = fields.get(9).cloned().unwrap_or_default();

        tasks.push(ScheduledTask {
            name: display,
            frequency,
            time,
            next_run,
            status,
        });
    }

    Ok(tasks)
}

/// Elimina una tarea programada por su nombre visible.
#[tauri::command]
pub fn delete_schedule(name: String) -> Result<String, String> {
    let task_name = format!("{PREFIX}{name}");
    let output = schtasks()
        .args(["/Delete", "/TN", &task_name, "/F"])
        .output()
        .map_err(|e| format!("No se pudo ejecutar schtasks: {e}"))?;

    if output.status.success() {
        Ok(format!("Tarea «{name}» eliminada."))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Ejecuta una tarea programada inmediatamente (útil para probarla).
#[tauri::command]
pub fn run_schedule_now(name: String) -> Result<String, String> {
    let task_name = format!("{PREFIX}{name}");
    let output = schtasks()
        .args(["/Run", "/TN", &task_name])
        .output()
        .map_err(|e| format!("No se pudo ejecutar schtasks: {e}"))?;

    if output.status.success() {
        Ok(format!("Ejecutando «{name}» ahora."))
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Parser mínimo de una línea CSV con campos entre comillas.
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    fields.push(cur.trim().to_string());
    fields
}
