use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use crate::logging::{log_enter, log_err, log_ok, preview_str};

pub struct ToolDependency {
    pub name: &'static str,
    pub check_cmd: &'static str,
    pub check_args: &'static [&'static str],
    /// Polecenie wymagające roota (instalacja systemowego pip/pipx)
    pub install_root_apt: &'static str,
    pub install_root_dnf: &'static str,
    pub install_root_pacman: &'static str,
    /// Polecenie wykonywane jako obecny USER (bez sudo)
    pub install_user: &'static str,
}

pub const TOOL_GEXT: ToolDependency = ToolDependency {
    name: "gext (gnome-extensions-cli)",
    check_cmd: "gext",
    check_args: &["--version"],
    // Faza 1 (root): tylko instalacja pipx z systemu
    install_root_apt: "apt-get install pipx -y",
    install_root_dnf: "dnf install pipx -y",
    install_root_pacman: "pacman -S python-pipx --noconfirm",
    // Faza 2 (user): gext instalowany jako zwykły użytkownik
    install_user: "pipx install gnome-extensions-cli",
};

// Cache: raz zainstalowane = nie pytaj ponownie w tej sesji
static GEXT_OK: OnceLock<bool> = OnceLock::new();

/// Sprawdza i ewentualnie instaluje wymagane narzędzie.
/// Wynik jest cache'owany — hasło jest proszone maksymalnie RAZ na sesję.
pub fn ensure_tool(tool: &ToolDependency) -> Result<()> {
    let started_at = log_enter("tools::ensure_tool", &format!("tool={}", tool.name));
    // Sprawdź cache
    if let Some(true) = GEXT_OK.get() {
        log_ok("tools::ensure_tool", started_at, "cache_hit=true");
        return Ok(());
    }

    if is_installed(tool) {
        let _ = GEXT_OK.set(true);
        log_ok("tools::ensure_tool", started_at, "already_installed=true");
        return Ok(());
    }

    log::info!(
        "Narzędzie {} nie jest zainstalowane. Instaluję...",
        tool.name
    );

    let pm = detect_package_manager();

    // Faza 1: Instalacja pakietu systemowego (pipx) przez pkexec jako root
    let root_cmd = match pm.as_str() {
        "apt" => tool.install_root_apt,
        "dnf" => tool.install_root_dnf,
        "pacman" => tool.install_root_pacman,
        _ => bail!("Nieobsługiwany menedżer pakietów: {}", pm),
    };

    log::info!("Faza 1: Instalacja pipx przez pkexec...");
    let root_status = Command::new("pkexec").arg("bash").arg("-c").arg(root_cmd).status()?;

    if !root_status.success() {
        let error = anyhow::anyhow!("Instalacja systemowego pipx nie powiodła się (anulowano hasło?)");
        log_err("tools::ensure_tool", started_at, &error);
        return Err(error);
    }

    // Faza 2: Instalacja gext jako BIEŻĄCY USER (bez sudo!)
    // gext musi być zainstalowane w profilu usera, nie roota
    log::info!("Faza 2: Instalacja gext jako user przez pipx...");
    let user_status = Command::new("bash")
        .arg("-c")
        .arg(tool.install_user)
        .status()?;

    if !user_status.success() {
        let error = anyhow::anyhow!("Instalacja gext przez pipx nie powiodła się");
        log_err("tools::ensure_tool", started_at, &error);
        return Err(error);
    }

    // Dodaj ~/.local/bin do PATH bieżącej sesji procesu
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let local_bin = format!("{}/.local/bin", home);
    if let Ok(current_path) = std::env::var("PATH") {
        if !current_path.contains(&local_bin) {
            std::env::set_var("PATH", format!("{}:{}", local_bin, current_path));
            log::info!("Dodano {} do PATH bieżącej sesji", local_bin);
        }
    }

    if is_installed(tool) {
        let _ = GEXT_OK.set(true);
        log::info!("Narzędzie {} zainstalowane i dostępne.", tool.name);
        log_ok("tools::ensure_tool", started_at, "installed_now=true");
        Ok(())
    } else {
        let error = anyhow::anyhow!(
            "Narzędzie '{}' zainstalowane, ale nadal niedostępne w PATH.\n\
             Ścieżka {} nie jest w PATH. Uruchom ponownie aplikację.",
            tool.name,
            local_bin
        );
        log_err("tools::ensure_tool", started_at, &error);
        Err(error)
    }
}

pub fn is_installed(tool: &ToolDependency) -> bool {
    let started_at = log_enter("tools::is_installed", &format!("tool={}", tool.name));
    // Sprawdź przez powłokę logowania (uwzględnia ~/.bashrc i ~/.profile)
    let result = Command::new("bash")
        .arg("-l")
        .arg("-c")
        .arg(format!("command -v {}", tool.check_cmd))
        .output();

    if let Ok(out) = result {
        if out.status.success() {
            log_ok(
                "tools::is_installed",
                started_at,
                &format!("result=true via=shell stdout={}", preview_str(&String::from_utf8_lossy(&out.stdout))),
            );
            return true;
        }
    }

    // B ezpośrednie sprawdzenie ścieżek user local bin
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let local_bin_path = format!("{}/.local/bin/{}", home, tool.check_cmd);
    if Path::new(&local_bin_path).exists() {
        // Jest zainstalowany — dodaj do PATH jeśli nie ma
        let local_bin = format!("{}/.local/bin", home);
        if let Ok(current_path) = std::env::var("PATH") {
            if !current_path.contains(&local_bin) {
                std::env::set_var("PATH", format!("{}:{}", local_bin, current_path));
            }
        }
        log_ok("tools::is_installed", started_at, "result=true via=local_bin");
        return true;
    }

    log_ok("tools::is_installed", started_at, "result=false");
    false
}

pub fn command_exists(command: &str) -> bool {
    Command::new("bash")
        .arg("-l")
        .arg("-c")
        .arg(format!("command -v {}", command))
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn detect_package_manager() -> String {
    let started_at = log_enter("tools::detect_package_manager", "");
    if Path::new("/usr/bin/apt").exists() || Path::new("/usr/bin/apt-get").exists() {
        log_ok("tools::detect_package_manager", started_at, "result=apt");
        return "apt".to_string();
    }
    if Path::new("/usr/bin/dnf").exists() {
        log_ok("tools::detect_package_manager", started_at, "result=dnf");
        return "dnf".to_string();
    }
    if Path::new("/usr/bin/pacman").exists() {
        log_ok("tools::detect_package_manager", started_at, "result=pacman");
        return "pacman".to_string();
    }
    log_ok("tools::detect_package_manager", started_at, "result=unknown");
    "unknown".to_string()
}

pub fn detect_terminal() -> (String, String) {
    let started_at = log_enter("tools::detect_terminal", "");
    if Path::new("/usr/bin/gnome-terminal").exists() {
        let result = ("gnome-terminal".to_string(), "--wait --".to_string());
        log_ok("tools::detect_terminal", started_at, "result=gnome-terminal");
        return result;
    }
    if Path::new("/usr/bin/konsole").exists() {
        let result = ("konsole".to_string(), "-e".to_string());
        log_ok("tools::detect_terminal", started_at, "result=konsole");
        return result;
    }
    if Path::new("/usr/bin/kgx").exists() {
        let result = ("kgx".to_string(), "-e".to_string());
        log_ok("tools::detect_terminal", started_at, "result=kgx");
        return result;
    }
    if Path::new("/usr/bin/alacritty").exists() {
        let result = ("alacritty".to_string(), "-e".to_string());
        log_ok("tools::detect_terminal", started_at, "result=alacritty");
        return result;
    }
    if Path::new("/usr/bin/kitty").exists() {
        let result = ("kitty".to_string(), "-e".to_string());
        log_ok("tools::detect_terminal", started_at, "result=kitty");
        return result;
    }
    log_ok("tools::detect_terminal", started_at, "result=none");
    ("none".to_string(), "".to_string())
}
