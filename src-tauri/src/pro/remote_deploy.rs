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

impl RemoteExecutor for PsRemotingExecutor {
    fn run(&self, host: &RemoteHost, winget_args: &[&str]) -> Result<RemoteResult, String> {
        // Validación mínima anti-inyección: winget args sin metacaracteres de shell.
        for a in winget_args {
            if a.chars().any(|c| "&|;`$<>\"'".contains(c)) {
                return Err("Argumento de winget no permitido.".into());
            }
        }
        let inner = format!("winget {}", winget_args.join(" "));
        let mut script = format!(
            "Invoke-Command -ComputerName '{}' -ScriptBlock {{ {} }}",
            host.host.replace('\'', "''"),
            inner
        );
        if let Some(user) = &host.user {
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
