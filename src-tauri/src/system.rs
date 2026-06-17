// Elevación a administrador. Permite reiniciar la app con permisos de admin
// una sola vez, evitando el UAC repetido por cada paquete que se actualiza.

use std::process::Command;
use tauri::AppHandle;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn powershell() -> Command {
    let mut cmd = Command::new("powershell");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// ¿Se está ejecutando la app como administrador?
#[tauri::command]
pub fn is_elevated() -> bool {
    let out = powershell()
        .args([
            "-NoProfile",
            "-Command",
            "([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)",
        ])
        .output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().eq_ignore_ascii_case("true"),
        Err(_) => false,
    }
}

/// Reinicia la app pidiendo elevación (un único aviso UAC) y cierra la instancia actual.
#[tauri::command]
pub fn relaunch_as_admin(app: AppHandle) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let path = exe.to_string_lossy().replace('\'', "''");
    powershell()
        .args([
            "-NoProfile",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &format!("Start-Process -FilePath '{path}' -Verb RunAs"),
        ])
        .spawn()
        .map_err(|e| format!("No se pudo elevar: {e}"))?;
    app.exit(0);
    Ok(())
}
