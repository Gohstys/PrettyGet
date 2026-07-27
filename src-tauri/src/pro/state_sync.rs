// PRO · StateSync — exportar/importar el estado de winget (JSON/YAML).
//
// Se apoya en los comandos nativos `winget export` / `winget import`, que ya
// producen/consumen un JSON oficial de paquetes instalados. Añadimos una capa
// propia (PackageEntry/WingetState) para poder serializar también a YAML y para
// versionar nuestro propio esquema.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageEntry {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WingetState {
    pub schema: u32,
    pub exported_at: i64,
    pub packages: Vec<PackageEntry>,
}

/// Contrato de sincronización de estado. Implementable también con un mock en tests.
pub trait StateSync {
    fn export(&self) -> Result<WingetState, String>;
    fn import(&self, state: &WingetState, silent: bool) -> Result<i32, String>;

    fn export_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.export()?).map_err(|e| e.to_string())
    }
    fn export_yaml(&self) -> Result<String, String> {
        serde_yaml::to_string(&self.export()?).map_err(|e| e.to_string())
    }
    fn import_str(&self, data: &str, silent: bool) -> Result<i32, String> {
        let state = parse_any(data)?;
        self.import(&state, silent)
    }
}

/// Detecta JSON o YAML automáticamente.
pub fn parse_any(data: &str) -> Result<WingetState, String> {
    if let Ok(s) = serde_json::from_str::<WingetState>(data) {
        return Ok(s);
    }
    serde_yaml::from_str::<WingetState>(data).map_err(|e| format!("Formato no reconocido: {e}"))
}

/// Implementación real sobre el winget del sistema.
pub struct WingetStateSync;

fn winget() -> Command {
    let mut c = Command::new("winget");
    #[cfg(windows)]
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

impl StateSync for WingetStateSync {
    fn export(&self) -> Result<WingetState, String> {
        // `winget export` escribe a un archivo; usamos uno temporal.
        let tmp = std::env::temp_dir().join(format!("pg-export-{}.json", now_unix()));
        let out = winget()
            .args([
                "export",
                "-o",
                &tmp.to_string_lossy(),
                "--accept-source-agreements",
                "--disable-interactivity",
            ])
            .output()
            .map_err(|e| format!("No se pudo ejecutar winget: {e}"))?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
        }
        let raw = std::fs::read_to_string(&tmp).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&tmp);
        Ok(parse_winget_export(&raw))
    }

    fn import(&self, state: &WingetState, silent: bool) -> Result<i32, String> {
        // Reconstruimos el formato que espera `winget import` y lo escribimos a temp.
        let doc = to_winget_export(state);
        let tmp = std::env::temp_dir().join(format!("pg-import-{}.json", now_unix()));
        {
            let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
            f.write_all(doc.as_bytes()).map_err(|e| e.to_string())?;
        }
        let mut args = vec![
            "import".to_string(),
            "-i".to_string(),
            tmp.to_string_lossy().to_string(),
            "--accept-source-agreements".to_string(),
            "--accept-package-agreements".to_string(),
            "--ignore-unavailable".to_string(),
        ];
        if silent {
            args.push("--silent".into());
        }
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let status = winget()
            .args(&refs)
            .status()
            .map_err(|e| format!("No se pudo ejecutar winget: {e}"))?;
        let _ = std::fs::remove_file(&tmp);
        Ok(status.code().unwrap_or(-1))
    }
}

/// Convierte el JSON de `winget export` a nuestro WingetState.
fn parse_winget_export(raw: &str) -> WingetState {
    let mut packages = Vec::new();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        if let Some(sources) = v.get("Sources").and_then(|s| s.as_array()) {
            for src in sources {
                let source_name = src
                    .get("SourceDetails")
                    .and_then(|d| d.get("Name"))
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string());
                if let Some(pkgs) = src.get("Packages").and_then(|p| p.as_array()) {
                    for p in pkgs {
                        if let Some(id) = p.get("PackageIdentifier").and_then(|i| i.as_str()) {
                            packages.push(PackageEntry {
                                id: id.to_string(),
                                version: p
                                    .get("Version")
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string()),
                                source: source_name.clone(),
                            });
                        }
                    }
                }
            }
        }
    }
    WingetState {
        schema: 1,
        exported_at: now_unix(),
        packages,
    }
}

/// Construye un JSON compatible con `winget import` a partir de WingetState.
fn to_winget_export(state: &WingetState) -> String {
    let packages: Vec<serde_json::Value> = state
        .packages
        .iter()
        .map(|p| serde_json::json!({ "PackageIdentifier": p.id }))
        .collect();
    serde_json::json!({
        "$schema": "https://aka.ms/winget-packages.schema.2.0.json",
        "WinGetVersion": "1.0",
        "Sources": [{
            "Packages": packages,
            "SourceDetails": { "Argument": "https://cdn.winget.microsoft.com/cache",
                "Identifier": "Microsoft.Winget.Source_8wekyb3d8bbwe",
                "Name": "winget", "Type": "Microsoft.PreIndexed.Package" }
        }]
    })
    .to_string()
}
