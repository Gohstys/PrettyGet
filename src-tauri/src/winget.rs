// Lógica de winget: detectar, listar/buscar, instalar/actualizar con logs en vivo.
// El parseo es INDEPENDIENTE DEL IDIOMA: localiza las columnas por su posición
// (a partir de la línea de guiones), no por el texto de la cabecera.

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Upgrade {
    pub name: String,
    pub id: String,
    pub current: String,
    pub available: String,
    pub source: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Package {
    pub name: String,
    pub id: String,
    pub version: String,
    pub source: String,
}

/// Localiza el ejecutable de winget de forma robusta.
///
/// Importante: el alias `winget` vive en `%LOCALAPPDATA%\Microsoft\WindowsApps`,
/// que está en el PATH del USUARIO pero a menudo NO en el de un proceso elevado.
/// Por eso, si la app corre como administrador, hay que apuntar al `winget.exe`
/// real dentro de `Program Files\WindowsApps`.
fn winget_exe() -> std::path::PathBuf {
    use std::path::PathBuf;

    // 1) Ejecutable real del paquete (accesible y necesario cuando se corre elevado).
    for var in ["ProgramFiles", "ProgramW6432"] {
        if let Some(pf) = std::env::var_os(var) {
            let wa = PathBuf::from(&pf).join("WindowsApps");
            if let Ok(entries) = std::fs::read_dir(&wa) {
                for e in entries.flatten() {
                    let name = e.file_name();
                    let name = name.to_string_lossy();
                    if name.starts_with("Microsoft.DesktopAppInstaller_")
                        && name.contains("8wekyb3d8bbwe")
                    {
                        let cand = e.path().join("winget.exe");
                        if cand.exists() {
                            return cand;
                        }
                    }
                }
            }
        }
    }

    // 2) Alias del usuario (funciona cuando NO se corre elevado).
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let p = PathBuf::from(&local)
            .join("Microsoft")
            .join("WindowsApps")
            .join("winget.exe");
        if p.exists() {
            return p;
        }
    }

    // 3) Último recurso: confiar en el PATH.
    PathBuf::from("winget")
}

fn winget_cmd() -> Command {
    let mut cmd = Command::new(winget_exe());
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

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

// ===================== Comandos de lectura =====================

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
        let ups = parse_upgrades(&text);
        if ups.is_empty() && !output.status.success() {
            return Err(winget_error(&output.stderr, &text));
        }
        Ok(ups)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn add_source(args: &mut Vec<String>, source: &str) {
    let s = source.trim();
    if !s.is_empty() && s != "all" {
        args.push("--source".into());
        args.push(s.to_string());
    }
}

fn add_mode(args: &mut Vec<String>, silent: bool) {
    if silent {
        args.push("--silent".into());
    } else {
        args.push("--interactive".into());
    }
}

#[tauri::command]
pub async fn search_packages(query: String, source: String) -> Result<Vec<Package>, String> {
    let q = query.trim().to_string();
    if q.is_empty() {
        return Ok(Vec::new());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let mut args = vec![
            "search".to_string(),
            "--query".to_string(),
            q,
            "--accept-source-agreements".to_string(),
            "--disable-interactivity".to_string(),
        ];
        add_source(&mut args, &source);
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = winget_cmd()
            .args(&refs)
            .output()
            .map_err(|e| format!("No se pudo ejecutar winget: {e}"))?;
        let text = decode(&output.stdout);
        let pkgs = parse_packages(&text);
        if pkgs.is_empty() && !output.status.success() {
            return Err(winget_error(&output.stderr, &text));
        }
        Ok(pkgs)
    })
    .await
    .map_err(|e| e.to_string())?
}

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
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = winget_cmd()
            .args(&refs)
            .output()
            .map_err(|e| format!("No se pudo ejecutar winget: {e}"))?;
        let text = decode(&output.stdout);
        let pkgs = parse_packages(&text);
        if pkgs.is_empty() && !output.status.success() {
            return Err(winget_error(&output.stderr, &text));
        }
        Ok(pkgs)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Construye un mensaje de error legible a partir de stderr/stdout de winget.
fn winget_error(stderr: &[u8], stdout_text: &str) -> String {
    let err = decode(stderr);
    let err = err.trim();
    if !err.is_empty() {
        return err.to_string();
    }
    let out = stdout_text.trim();
    if !out.is_empty() {
        return out.lines().take(3).collect::<Vec<_>>().join(" ");
    }
    "winget no devolvió resultados. Comprueba que está instalado y actualizado.".to_string()
}

// ===================== Comandos con streaming =====================

#[tauri::command]
pub async fn upgrade_package(app: AppHandle, id: String) -> Result<i32, String> {
    let args = vec![
        "upgrade".into(),
        "--id".into(),
        id,
        "--exact".into(),
        "--silent".into(),
        "--accept-source-agreements".into(),
        "--accept-package-agreements".into(),
        "--include-unknown".into(),
    ];
    stream(app, args).await
}

#[tauri::command]
pub async fn upgrade_all(app: AppHandle) -> Result<i32, String> {
    let args = vec![
        "upgrade".into(),
        "--all".into(),
        "--silent".into(),
        "--accept-source-agreements".into(),
        "--accept-package-agreements".into(),
        "--include-unknown".into(),
    ];
    stream(app, args).await
}

#[tauri::command]
pub async fn install_package(
    app: AppHandle,
    id: String,
    source: String,
    silent: bool,
) -> Result<i32, String> {
    let mut args = vec![
        "install".to_string(),
        "--id".to_string(),
        id,
        "--exact".to_string(),
        "--accept-source-agreements".to_string(),
        "--accept-package-agreements".to_string(),
    ];
    add_mode(&mut args, silent);
    add_source(&mut args, &source);
    stream(app, args).await
}

#[tauri::command]
pub async fn uninstall_package(app: AppHandle, id: String, silent: bool) -> Result<i32, String> {
    let mut args = vec![
        "uninstall".to_string(),
        "--id".to_string(),
        id,
        "--exact".to_string(),
        "--accept-source-agreements".to_string(),
    ];
    add_mode(&mut args, silent);
    stream(app, args).await
}

/// Lanza winget y retransmite su salida en vivo. Las actualizaciones de progreso
/// (terminadas en \r) se emiten como "transitorias" para reemplazar la línea
/// anterior; las líneas terminadas en \n se confirman.
async fn stream(app: AppHandle, args: Vec<String>) -> Result<i32, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut child = winget_cmd()
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("No se pudo ejecutar winget: {e}"))?;

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let app_err = app.clone();
        let err_handle = std::thread::spawn(move || {
            if let Some(e) = stderr {
                emit_lines(&app_err, e);
            }
        });
        if let Some(o) = stdout {
            emit_lines(&app, o);
        }
        let _ = err_handle.join();

        let status = child.wait().map_err(|e| e.to_string())?;
        let code = status.code().unwrap_or(-1);
        let _ = app.emit("winget-done", code);
        Ok(code)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn emit_lines<R: Read>(app: &AppHandle, mut reader: R) {
    let mut acc = String::new();
    let mut chunk = [0u8; 4096];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                acc.push_str(&String::from_utf8_lossy(&chunk[..n]));
                process_acc(app, &mut acc, false);
            }
            Err(_) => break,
        }
    }
    process_acc(app, &mut acc, true);
}

fn process_acc(app: &AppHandle, acc: &mut String, eof: bool) {
    loop {
        let pos = acc.find(|c| c == '\n' || c == '\r');
        let i = match pos {
            Some(i) => i,
            None => break,
        };
        let is_nl = acc.as_bytes()[i] == b'\n';
        if is_nl {
            let line: String = acc.drain(..=i).collect();
            commit(app, &line[..line.len() - 1], false);
        } else {
            // '\r': si es el último byte y no es EOF, espera por si llega "\r\n".
            if i + 1 >= acc.len() && !eof {
                break;
            }
            if acc.as_bytes().get(i + 1) == Some(&b'\n') {
                let line: String = acc.drain(..=i + 1).collect();
                commit(app, &line[..line.len() - 2], false);
            } else {
                let line: String = acc.drain(..=i).collect();
                commit(app, &line[..line.len() - 1], true);
            }
        }
    }
    if eof && !acc.is_empty() {
        let line = std::mem::take(acc);
        commit(app, &line, false);
    }
}

fn commit(app: &AppHandle, raw: &str, transient: bool) {
    let text = strip_ansi(raw);
    let trimmed = text.trim_end();
    if trimmed.trim().is_empty() {
        return;
    }
    let _ = app.emit(
        "winget-out",
        serde_json::json!({ "text": trimmed, "transient": transient }),
    );
}

// ===================== Utilidades de texto =====================

/// Elimina secuencias de escape ANSI (colores, movimientos de cursor, etc.).
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    while let Some(&nc) = chars.peek() {
                        chars.next();
                        if ('@'..='~').contains(&nc) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    while let Some(&nc) = chars.peek() {
                        chars.next();
                        if nc == '\u{7}' {
                            break;
                        }
                        if nc == '\u{1b}' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                Some(_) => {
                    chars.next();
                }
                None => {}
            }
        } else if c != '\u{0}' {
            out.push(c);
        }
    }
    out
}

fn decode(bytes: &[u8]) -> String {
    let s = decode_bytes(bytes);
    strip_ansi(&s).replace('\r', "\n")
}

/// Decodifica la salida de winget tolerando UTF-8, UTF-16 (con/sin BOM) y BOM UTF-8.
fn decode_bytes(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return utf16le(&bytes[2..]);
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        // UTF-16 big-endian
        let u16s: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        return String::from_utf16_lossy(&u16s);
    }
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&bytes[3..]).into_owned();
    }
    // Heurística: UTF-16LE sin BOM → muchos bytes nulos en posiciones impares.
    let sample = &bytes[..bytes.len().min(64)];
    let nul = sample.iter().filter(|&&b| b == 0).count();
    if !sample.is_empty() && nul * 3 >= sample.len() {
        return utf16le(bytes);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn utf16le(bytes: &[u8]) -> String {
    let u16s: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    String::from_utf16_lossy(&u16s)
}

fn slice_chars(line: &[char], start: usize, end: usize) -> String {
    if start >= line.len() {
        return String::new();
    }
    let end = end.min(line.len());
    line[start..end].iter().collect::<String>().trim().to_string()
}

/// Devuelve el índice de la línea de cabecera (la que está justo encima de la
/// línea de guiones separadora), o None si no hay tabla.
fn find_header(lines: &[&str]) -> Option<usize> {
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim();
        let dashes = t.chars().filter(|&c| c == '-').count();
        if dashes >= 8 && t.chars().all(|c| c == '-' || c == ' ') && i > 0 {
            return Some(i - 1);
        }
    }
    None
}

/// Posiciones (en caracteres) donde empieza cada columna, según la cabecera.
fn column_starts(header: &str) -> Vec<usize> {
    let chars: Vec<char> = header.chars().collect();
    let mut starts = Vec::new();
    let mut prev_space = true;
    for (i, &c) in chars.iter().enumerate() {
        let is_space = c == ' ' || c == '\t';
        if !is_space && prev_space {
            starts.push(i);
        }
        prev_space = is_space;
    }
    starts
}

/// Divide la tabla en filas de campos según las posiciones de las columnas.
fn parse_rows(text: &str) -> (usize, Vec<Vec<String>>) {
    let lines: Vec<&str> = text.lines().collect();
    let header_idx = match find_header(&lines) {
        Some(i) => i,
        None => return (0, Vec::new()),
    };
    let starts = column_starts(lines[header_idx]);
    if starts.len() < 2 {
        return (0, Vec::new());
    }

    let mut rows = Vec::new();
    for &line in lines.iter().skip(header_idx + 2) {
        let t = line.trim();
        if t.is_empty() || t.chars().all(|c| c == '-' || c == ' ') {
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        let mut fields = Vec::with_capacity(starts.len());
        for i in 0..starts.len() {
            let s = starts[i];
            let e = if i + 1 < starts.len() {
                starts[i + 1]
            } else {
                chars.len()
            };
            fields.push(slice_chars(&chars, s, e));
        }
        rows.push(fields);
    }
    (starts.len(), rows)
}

/// Estructura de `winget upgrade`: Name, Id, Version, Available, Source.
pub fn parse_upgrades(text: &str) -> Vec<Upgrade> {
    let (_, rows) = parse_rows(text);
    rows.into_iter()
        .filter_map(|f| {
            let name = f.first().cloned().unwrap_or_default();
            let id = f.get(1).cloned().unwrap_or_default();
            let current = f.get(2).cloned().unwrap_or_default();
            let available = f.get(3).cloned().unwrap_or_default();
            let source = f.last().cloned().unwrap_or_default();
            // Id sin espacios y versión disponible no vacía → fila válida.
            if id.is_empty() || id.contains(char::is_whitespace) || available.is_empty() {
                return None;
            }
            Some(Upgrade {
                name,
                id,
                current,
                available,
                source,
            })
        })
        .collect()
}

/// Estructura de `winget search`/`list`: Name, Id, Version, [Match], Source.
pub fn parse_packages(text: &str) -> Vec<Package> {
    let (_, rows) = parse_rows(text);
    rows.into_iter()
        .filter_map(|f| {
            let name = f.first().cloned().unwrap_or_default();
            let id = f.get(1).cloned().unwrap_or_default();
            let version = f.get(2).cloned().unwrap_or_default();
            let source = f.last().cloned().unwrap_or_default();
            if id.is_empty() || id.contains(char::is_whitespace) {
                return None;
            }
            Some(Package {
                name,
                id,
                version,
                source,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_english_upgrades() {
        let s = "\
Name                            Id                            Version       Available     Source
---------------------------------------------------------------------------------------------------
Windows Terminal                Microsoft.WindowsTerminal     1.18.3181.0   1.19.10302.0  winget
Visual Studio Code              Microsoft.VisualStudioCode    1.88.0        1.89.1        winget
3 upgrades available.";
        let ups = parse_upgrades(s);
        assert_eq!(ups.len(), 2);
        assert_eq!(ups[0].id, "Microsoft.WindowsTerminal");
        assert_eq!(ups[0].available, "1.19.10302.0");
        assert_eq!(ups[0].source, "winget");
    }

    #[test]
    fn parses_spanish_upgrades() {
        let s = "\
Nombre                          Id                            Versión       Disponible    Origen
---------------------------------------------------------------------------------------------------
Windows Terminal                Microsoft.WindowsTerminal     1.18.3181.0   1.19.10302.0  winget
7-Zip                           7zip.7zip                     23.01         24.05         winget
2 actualizaciones disponibles.";
        let ups = parse_upgrades(s);
        assert_eq!(ups.len(), 2);
        assert_eq!(ups[1].id, "7zip.7zip");
        assert_eq!(ups[1].available, "24.05");
    }

    #[test]
    fn parses_search_spanish_with_match() {
        let s = "\
Nombre               Id                        Versión   Coincidencia     Origen
-----------------------------------------------------------------------------------
Mozilla Firefox      Mozilla.Firefox           126.0     Moniker: firefox winget
PowerToys            Microsoft.PowerToys       0.81.1                     winget";
        let pkgs = parse_packages(s);
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0].id, "Mozilla.Firefox");
        assert_eq!(pkgs[0].version, "126.0");
        assert_eq!(pkgs[0].source, "winget");
        assert_eq!(pkgs[1].id, "Microsoft.PowerToys");
    }

    #[test]
    fn strips_ansi_and_footer() {
        let s = "\u{1b}[2mNote\u{1b}[0m\nName        Id            Version   Available  Source\n----------------------------------------------------------\nGit         Git.Git       2.44.0    2.45.1     winget\n";
        let ups = parse_upgrades(s);
        assert_eq!(ups.len(), 1);
        assert_eq!(ups[0].id, "Git.Git");
    }
}
