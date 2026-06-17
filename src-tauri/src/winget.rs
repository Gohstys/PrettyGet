// Lógica de winget: detectar, listar actualizaciones y aplicarlas con logs en vivo.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Una aplicación que tiene una actualización disponible.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Upgrade {
    pub name: String,
    pub id: String,
    pub current: String,
    pub available: String,
    pub source: String,
}

/// Un paquete encontrado al buscar o ya instalado.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Package {
    pub name: String,
    pub id: String,
    pub version: String,
    pub source: String,
}

/// Construye un Command de winget sin abrir una ventana de consola.
fn winget_cmd() -> Command {
    let mut cmd = Command::new("winget");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// ¿Está winget instalado y disponible?
#[tauri::command]
pub fn winget_available() -> bool {
    winget_cmd()
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Lista los paquetes que tienen una actualización disponible.
#[tauri::command]
pub async fn list_upgrades() -> Result<Vec<Upgrade>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let output = winget_cmd()
            .args([
                "upgrade",
                "--include-unknown",
                "--accept-source-agreements",
                "--disable-interactivity",
            ])
            .output()
            .map_err(|e| format!("No se pudo ejecutar winget: {e}. ¿Está instalado?"))?;

        let text = decode(&output.stdout);
        Ok(parse_upgrades(&text))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Actualiza un único paquete por su Id, emitiendo cada línea como evento.
#[tauri::command]
pub async fn upgrade_package(app: AppHandle, id: String) -> Result<i32, String> {
    let args = vec![
        "upgrade".to_string(),
        "--id".to_string(),
        id,
        "--exact".to_string(),
        "--silent".to_string(),
        "--accept-source-agreements".to_string(),
        "--accept-package-agreements".to_string(),
        "--include-unknown".to_string(),
        "--disable-interactivity".to_string(),
    ];
    stream(app, args).await
}

/// Actualiza TODOS los paquetes, emitiendo cada línea como evento.
#[tauri::command]
pub async fn upgrade_all(app: AppHandle) -> Result<i32, String> {
    let args = vec![
        "upgrade".to_string(),
        "--all".to_string(),
        "--silent".to_string(),
        "--accept-source-agreements".to_string(),
        "--accept-package-agreements".to_string(),
        "--include-unknown".to_string(),
        "--disable-interactivity".to_string(),
    ];
    stream(app, args).await
}

/// Busca paquetes disponibles para instalar.
#[tauri::command]
pub async fn search_packages(query: String) -> Result<Vec<Package>, String> {
    let q = query.trim().to_string();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let output = winget_cmd()
            .args([
                "search",
                "--query",
                &q,
                "--accept-source-agreements",
                "--disable-interactivity",
            ])
            .output()
            .map_err(|e| format!("No se pudo ejecutar winget: {e}"))?;
        let text = decode(&output.stdout);
        Ok(parse_packages(&text))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Lista los paquetes instalados (opcionalmente filtrados por texto).
#[tauri::command]
pub async fn list_installed(query: String) -> Result<Vec<Package>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let q = query.trim();
        let mut args: Vec<String> = vec![
            "list".into(),
            "--accept-source-agreements".into(),
            "--disable-interactivity".into(),
        ];
        if !q.is_empty() {
            args.push("--query".into());
            args.push(q.to_string());
        }
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = winget_cmd()
            .args(&arg_refs)
            .output()
            .map_err(|e| format!("No se pudo ejecutar winget: {e}"))?;
        let text = decode(&output.stdout);
        Ok(parse_packages(&text))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Instala un paquete por su Id, con logs en vivo.
#[tauri::command]
pub async fn install_package(app: AppHandle, id: String) -> Result<i32, String> {
    let args = vec![
        "install".to_string(),
        "--id".to_string(),
        id,
        "--exact".to_string(),
        "--silent".to_string(),
        "--accept-source-agreements".to_string(),
        "--accept-package-agreements".to_string(),
        "--disable-interactivity".to_string(),
    ];
    stream(app, args).await
}

/// Desinstala un paquete por su Id, con logs en vivo.
#[tauri::command]
pub async fn uninstall_package(app: AppHandle, id: String) -> Result<i32, String> {
    let args = vec![
        "uninstall".to_string(),
        "--id".to_string(),
        id,
        "--exact".to_string(),
        "--silent".to_string(),
        "--accept-source-agreements".to_string(),
        "--disable-interactivity".to_string(),
    ];
    stream(app, args).await
}

/// Ejecuta winget retransmitiendo stdout/stderr línea a línea al frontend.
async fn stream(app: AppHandle, args: Vec<String>) -> Result<i32, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut child = winget_cmd()
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("No se pudo ejecutar winget: {e}"))?;

        if let Some(out) = child.stdout.take() {
            emit_lines(&app, out);
        }
        if let Some(err) = child.stderr.take() {
            emit_lines(&app, err);
        }

        let status = child.wait().map_err(|e| e.to_string())?;
        let code = status.code().unwrap_or(-1);
        let _ = app.emit("upgrade-done", code);
        Ok(code)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Lee un stream byte a byte y emite cada línea (separadas por \n o \r) como evento "upgrade-log".
fn emit_lines<R: Read>(app: &AppHandle, reader: R) {
    let mut buf = BufReader::new(reader);
    let mut bytes: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 1];
    loop {
        match buf.read(&mut chunk) {
            Ok(0) => break,
            Ok(_) => {
                let b = chunk[0];
                if b == b'\n' || b == b'\r' {
                    flush(app, &mut bytes);
                } else {
                    bytes.push(b);
                }
            }
            Err(_) => break,
        }
    }
    flush(app, &mut bytes);
}

fn flush(app: &AppHandle, bytes: &mut Vec<u8>) {
    if bytes.is_empty() {
        return;
    }
    let line = String::from_utf8_lossy(bytes).trim().to_string();
    bytes.clear();
    // Ignora líneas de barra de progreso (solo símbolos) y líneas vacías.
    if line.is_empty() || line.chars().all(|c| matches!(c, '█' | '▒' | '░' | '-' | '\\' | '/' | '|' | '.' | ' ')) {
        return;
    }
    let _ = app.emit("upgrade-log", line);
}

/// Decodifica la salida de winget. Moderno winget usa UTF-8.
fn decode(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace('\r', "\n")
}

/// Devuelve el índice de carácter (no byte) donde empieza `needle` dentro de `haystack`.
fn char_pos(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .find(needle)
        .map(|byte_idx| haystack[..byte_idx].chars().count())
}

/// Extrae los caracteres en el rango [start, end) y los recorta.
fn slice_chars(line: &[char], start: usize, end: usize) -> String {
    if start >= line.len() {
        return String::new();
    }
    let end = end.min(line.len());
    line[start..end].iter().collect::<String>().trim().to_string()
}

/// Parsea CUALQUIER tabla de ancho fijo de winget (upgrade / search / list)
/// localizando las columnas por su cabecera y devolviendo filas como mapas
/// cabecera→valor. Tolera columnas presentes o ausentes (p. ej. "Match").
fn parse_table(text: &str) -> Vec<std::collections::HashMap<String, String>> {
    const KNOWN: [&str; 6] = ["Name", "Id", "Version", "Available", "Match", "Source"];
    let lines: Vec<&str> = text.lines().collect();

    // Cabecera = primera línea que contiene a la vez "Name" e "Id".
    let header_idx = match lines.iter().position(|l| l.contains("Name") && l.contains("Id")) {
        Some(i) => i,
        None => return Vec::new(),
    };
    let header = lines[header_idx];

    // Posiciones (en caracteres) de las columnas presentes, ordenadas.
    let mut cols: Vec<(usize, &str)> = KNOWN
        .iter()
        .filter_map(|k| char_pos(header, k).map(|p| (p, *k)))
        .collect();
    cols.sort_by_key(|(p, _)| *p);
    if cols.is_empty() {
        return Vec::new();
    }

    let mut rows = Vec::new();
    for &line in lines.iter().skip(header_idx + 1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.chars().all(|c| c == '-' || c == ' ') {
            continue; // separador de guiones
        }
        let tl = trimmed.to_lowercase();
        if tl.contains("upgrades available")
            || tl.contains("upgrade available")
            || tl.contains("no package found")
            || tl.contains("no installed package")
            || tl.contains("following packages")
        {
            break; // pie de tabla
        }

        let chars: Vec<char> = line.chars().collect();
        let mut map = std::collections::HashMap::new();
        for i in 0..cols.len() {
            let start = cols[i].0;
            let end = if i + 1 < cols.len() {
                cols[i + 1].0
            } else {
                chars.len()
            };
            map.insert(cols[i].1.to_string(), slice_chars(&chars, start, end));
        }
        // Requiere un Id no vacío para considerarla una fila válida.
        if map.get("Id").map(|s| s.is_empty()).unwrap_or(true) {
            continue;
        }
        rows.push(map);
    }
    rows
}

/// Filas de `winget upgrade` → paquetes con actualización disponible.
pub fn parse_upgrades(text: &str) -> Vec<Upgrade> {
    parse_table(text)
        .into_iter()
        .filter_map(|m| {
            let id = m.get("Id").cloned().unwrap_or_default();
            let available = m.get("Available").cloned().unwrap_or_default();
            // Los Id de winget nunca llevan espacios: descarta filas mal alineadas o pies de tabla.
            if id.is_empty() || available.is_empty() || id.contains(char::is_whitespace) {
                return None;
            }
            Some(Upgrade {
                name: m.get("Name").cloned().unwrap_or_default(),
                id,
                current: m.get("Version").cloned().unwrap_or_default(),
                available,
                source: m.get("Source").cloned().unwrap_or_default(),
            })
        })
        .collect()
}

/// Filas de `winget search` o `winget list` → paquetes.
pub fn parse_packages(text: &str) -> Vec<Package> {
    parse_table(text)
        .into_iter()
        .filter_map(|m| {
            let id = m.get("Id").cloned().unwrap_or_default();
            if id.is_empty() || id.contains(char::is_whitespace) {
                return None;
            }
            Some(Package {
                name: m.get("Name").cloned().unwrap_or_default(),
                id,
                version: m.get("Version").cloned().unwrap_or_default(),
                source: m.get("Source").cloned().unwrap_or_default(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_table() {
        let sample = "\
Name                            Id                            Version       Available     Source
---------------------------------------------------------------------------------------------------
Windows Terminal                Microsoft.WindowsTerminal     1.18.3181.0   1.19.10302.0  winget
Visual Studio Code              Microsoft.VisualStudioCode    1.88.0        1.89.1        winget
7-Zip                           7zip.7zip                     23.01         24.05         winget
3 upgrades available.";
        let ups = parse_upgrades(sample);
        assert_eq!(ups.len(), 3);
        assert_eq!(ups[0].name, "Windows Terminal");
        assert_eq!(ups[0].id, "Microsoft.WindowsTerminal");
        assert_eq!(ups[0].current, "1.18.3181.0");
        assert_eq!(ups[0].available, "1.19.10302.0");
        assert_eq!(ups[0].source, "winget");
        assert_eq!(ups[2].id, "7zip.7zip");
    }

    #[test]
    fn empty_when_no_header() {
        assert!(parse_upgrades("No installed package found matching input criteria.").is_empty());
    }

    #[test]
    fn parses_search_with_match_column() {
        let sample = "\
Name                 Id                        Version   Match            Source
-----------------------------------------------------------------------------------
Mozilla Firefox      Mozilla.Firefox           126.0     Moniker: firefox winget
PowerToys            Microsoft.PowerToys       0.81.1                     winget";
        let pkgs = parse_packages(sample);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].name, "Mozilla Firefox");
        assert_eq!(pkgs[0].id, "Mozilla.Firefox");
        assert_eq!(pkgs[0].version, "126.0");
        assert_eq!(pkgs[0].source, "winget");
        assert_eq!(pkgs[1].id, "Microsoft.PowerToys");
        assert_eq!(pkgs[1].source, "winget");
    }
}
