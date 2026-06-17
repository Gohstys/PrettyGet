// PRO · IaC_Generator — traduce selecciones de la GUI a scripts reproducibles
// (PowerShell y playbooks de Ansible). Son funciones puras: fáciles de testear.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Install,
    Upgrade,
    Uninstall,
}

impl Action {
    fn winget_verb(&self) -> &'static str {
        match self {
            Action::Install => "install",
            Action::Upgrade => "upgrade",
            Action::Uninstall => "uninstall",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub action: Action,
    /// Ids de winget (p. ej. "Google.Chrome").
    pub packages: Vec<String>,
    /// Instalación silenciosa.
    #[serde(default)]
    pub silent: bool,
}

/// Contrato de generación de Infraestructura-como-Código.
pub trait IacGenerator {
    fn powershell(&self, sel: &Selection) -> String;
    fn ansible(&self, sel: &Selection) -> String;
}

pub struct DefaultIac;

fn sanitize_id(id: &str) -> String {
    // Los Id de winget no llevan espacios ni metacaracteres; filtramos por seguridad.
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
        .collect()
}

impl IacGenerator for DefaultIac {
    fn powershell(&self, sel: &Selection) -> String {
        let verb = sel.action.winget_verb();
        let flags = if sel.silent {
            " --silent --accept-package-agreements --accept-source-agreements"
        } else {
            " --accept-source-agreements"
        };
        let mut out = String::new();
        out.push_str("# Generado por PrettyGet Pro — IaC\n");
        out.push_str("#Requires -RunAsAdministrator\n");
        out.push_str("$ErrorActionPreference = 'Continue'\n\n");
        for id in &sel.packages {
            let id = sanitize_id(id);
            out.push_str(&format!(
                "Write-Host 'PrettyGet: {verb} {id}'\nwinget {verb} --id {id} --exact{flags}\n\n"
            ));
        }
        out
    }

    fn ansible(&self, sel: &Selection) -> String {
        let verb = sel.action.winget_verb();
        let flags = if sel.silent {
            "--silent --accept-package-agreements --accept-source-agreements"
        } else {
            "--accept-source-agreements"
        };
        let mut out = String::new();
        out.push_str("# Generado por PrettyGet Pro — IaC\n");
        out.push_str("- name: PrettyGet winget deployment\n");
        out.push_str("  hosts: windows\n");
        out.push_str("  gather_facts: false\n");
        out.push_str("  tasks:\n");
        for id in &sel.packages {
            let id = sanitize_id(id);
            out.push_str(&format!("    - name: winget {verb} {id}\n"));
            out.push_str("      ansible.windows.win_command: >\n");
            out.push_str(&format!(
                "        winget {verb} --id {id} --exact {flags}\n"
            ));
            out.push_str("      register: pg_result\n");
            out.push_str("      changed_when: \"'No applicable upgrade' not in pg_result.stdout\"\n");
            out.push_str("      failed_when: false\n");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powershell_has_one_line_per_package() {
        let sel = Selection {
            action: Action::Install,
            packages: vec!["Google.Chrome".into(), "7zip.7zip".into()],
            silent: true,
        };
        let ps = DefaultIac.powershell(&sel);
        assert!(ps.contains("winget install --id Google.Chrome"));
        assert!(ps.contains("winget install --id 7zip.7zip"));
        assert!(ps.contains("--silent"));
    }

    #[test]
    fn ansible_is_valid_ish_yaml() {
        let sel = Selection {
            action: Action::Upgrade,
            packages: vec!["Microsoft.PowerToys".into()],
            silent: false,
        };
        let y = DefaultIac.ansible(&sel);
        assert!(y.contains("hosts: windows"));
        assert!(y.contains("winget upgrade --id Microsoft.PowerToys"));
    }
}
