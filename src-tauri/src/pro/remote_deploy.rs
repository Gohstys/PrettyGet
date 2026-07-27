// PRO · RemoteDeploy — ejecutar winget en máquinas remotas de la red local.
//
// Estrategia por defecto: PowerShell Remoting (WinRM), nativo en redes Windows,
// vía `Invoke-Command -ComputerName <host> -ScriptBlock { winget ... }`.
// Para entornos con OpenSSH puedes implementar el mismo trait con `russh`
// (SSH puro en Rust) — ver nota al final.

use serde::{Deserialize, Serialize};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteHost {
    pub host: String,
    /// Usuario opcional "DOMINIO\\user". Las credenciales se gestionan fuera (Kerberos/CredSSP).
    pub user: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoteResult {
    pub host: String,
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Contrato de ejecución remota. Una sola operación winget contra un host.
pub trait RemoteExecutor: Send + Sync {
    fn run(&self, host: &RemoteHost, winget_args: &[&str]) -> Result<RemoteResult, String>;

    /// Conveniencia: ejecuta en varios hosts (secuencial; paraleliza con rayon/tokio si quieres).
    fn run_many(&self, hosts: &[RemoteHost], winget_args: &[&str]) -> Vec<RemoteResult> {
        hosts
            .iter()
            .map(|h| {
                self.run(h, winget_args).unwrap_or_else(|e| RemoteResult {
                    host: h.host.clone(),
                    code: -1,
                    stdout: String::new(),
                    stderr: e,
                })
            })
            .collect()
    }
}

/// Implementación vía WinRM / PowerShell Remoting.
pub struct PsRemotingExecutor;

fn powershell() -> Command {
    let mut c = Command::new("powershell");
    #[cfg(windows)]
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

/// Caracteres permitidos en un argumento de winget.
///
/// LISTA BLANCA a propósito, no lista negra: los argumentos se interpolan dentro
/// de un `-ScriptBlock { ... }`, así que una lista negra tiene que acertar con
/// TODOS los metacaracteres. La anterior olvidaba `{`, `}`, `(`, `)` y los saltos
/// de línea — y un `}` seguido de un salto de línea cierra el bloque y deja
/// ejecutar PowerShell arbitrario detrás.
fn arg_char_allowed(c: char) -> bool {
    c.is_ascii_alphanumeric() || "._:-=+/\\".contains(c)
}

/// Un nombre de host solo puede ser alfanumérico con puntos/guiones (DNS/NetBIOS)
/// o una IP. Nada de espacios ni metacaracteres.
fn host_allowed(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host.chars().all(|c| c.is_ascii_alphanumeric() || ".-".contains(c))
}

/// Usuario opcional, típicamente `DOMINIO\usuario` o `usuario@dominio`.
/// Va dentro de comillas simples de PowerShell (donde todo es literal salvo `'`),
/// pero validamos igual por defensa en profundidad.
fn user_allowed(user: &str) -> bool {
    !user.is_empty()
        && user.len() <= 256
        && user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._@\\-".contains(c))
}

pub fn validate_args(winget_args: &[&str]) -> Result<(), String> {
    for a in winget_args {
        if a.is_empty() {
            return Err("Argumento de winget vacío.".into());
        }
        if let Some(bad) = a.chars().find(|c| !arg_char_allowed(*c)) {
            return Err(format!(
                "Argumento de winget no permitido: el carácter {bad:?} podría romper el script remoto."
            ));
        }
    }
    Ok(())
}

impl RemoteExecutor for PsRemotingExecutor {
    fn run(&self, host: &RemoteHost, winget_args: &[&str]) -> Result<RemoteResult, String> {
        validate_args(winget_args)?;
        if !host_allowed(&host.host) {
            return Err(format!("Nombre de host no válido: {:?}", host.host));
        }
        let inner = format!("winget {}", winget_args.join(" "));
        let mut script = format!(
            "Invoke-Command -ComputerName '{}' -ScriptBlock {{ {} }}",
            host.host.replace('\'', "''"),
            inner
        );
        if let Some(user) = &host.user {
            if !user_allowed(user) {
                return Err(format!("Nombre de usuario no válido: {user:?}"));
            }
            // Pide credenciales de forma interactiva; en producción usa -Credential con un PSCredential almacenado.
            script = format!(
                "$c = Get-Credential -UserName '{}' -Message 'PrettyGet remote'; {} -Credential $c",
                user.replace('\'', "''"),
                script
            );
        }
        let out = powershell()
            .args(["-NoProfile", "-Command", &script])
            .output()
            .map_err(|e| format!("No se pudo ejecutar PowerShell: {e}"))?;
        Ok(RemoteResult {
            host: host.host.clone(),
            code: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
        })
    }
}

// NOTA SSH: para una variante con `russh`, implementa `RemoteExecutor` para un
// `SshExecutor { key/credenciales }`, abre una sesión, ejecuta el canal con
// `winget {args}` y mapea stdout/stderr/exit-code a `RemoteResult`. El resto de
// la app (gating, comandos) no cambia gracias al trait.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_winget_args() {
        assert!(validate_args(&["upgrade", "--all", "--silent"]).is_ok());
        assert!(validate_args(&["install", "--id", "Mozilla.Firefox", "--exact"]).is_ok());
        assert!(validate_args(&["upgrade", "--id", "Microsoft.VisualStudioCode"]).is_ok());
    }

    #[test]
    fn rejects_scriptblock_breakout() {
        // El caso que la lista negra anterior dejaba pasar: cerrar el -ScriptBlock
        // con `}` y colar otra sentencia tras un salto de línea.
        assert!(validate_args(&["upgrade", "}\nRemove-Item C:\\ -Recurse\n{"]).is_err());
        assert!(validate_args(&["}"]).is_err());
        assert!(validate_args(&["{"]).is_err());
        assert!(validate_args(&["upgrade\nWrite-Host pwned"]).is_err());
        assert!(validate_args(&["$(whoami)"]).is_err());
        assert!(validate_args(&["a;b"]).is_err());
        assert!(validate_args(&["a b"]).is_err()); // un espacio ya sería otro argumento
        assert!(validate_args(&[""]).is_err());
    }

    #[test]
    fn validates_hosts_and_users() {
        assert!(host_allowed("pc-01"));
        assert!(host_allowed("server.local"));
        assert!(host_allowed("192.168.1.10"));
        assert!(!host_allowed(""));
        assert!(!host_allowed("pc-01'; Remove-Item C:\\ #"));
        assert!(!host_allowed("host with spaces"));

        assert!(user_allowed("DOMINIO\\usuario"));
        assert!(user_allowed("user@example.com"));
        assert!(!user_allowed("user'; whoami #"));
        assert!(!user_allowed(""));
    }
}
