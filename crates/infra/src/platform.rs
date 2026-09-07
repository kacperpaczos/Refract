use anyhow::Result;
use std::collections::HashSet;
use std::fs;

use crate::logging::{log_enter, log_ok, preview_debug};

#[derive(Debug, Clone)]
pub struct PlatformInfo {
    pub os_id: String,
    pub os_like: Vec<String>,
    pub package_manager: Option<String>,
}

impl PlatformInfo {
    pub fn current() -> Result<Self> {
        let started_at = log_enter("PlatformInfo::current", "");
        let (id, like) = Self::parse_os_release()?;
        let mut platform = Self {
            os_id: id,
            os_like: like,
            package_manager: None,
        };
        platform.package_manager = platform.detect_package_manager();
        log_ok("PlatformInfo::current", started_at, &format!("platform={}", preview_debug(&platform.os_id)));
        Ok(platform)
    }

    fn parse_os_release() -> Result<(String, Vec<String>)> {
        let started_at = log_enter("PlatformInfo::parse_os_release", "path=/etc/os-release");
        let content = fs::read_to_string("/etc/os-release").unwrap_or_default();
        let mut id = String::new();
        let mut like = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("ID=") {
                id = line.trim_start_matches("ID=").trim_matches('"').to_string();
            } else if line.starts_with("ID_LIKE=") {
                let like_str = line.trim_start_matches("ID_LIKE=").trim_matches('"');
                like = like_str.split_whitespace().map(|s| s.to_string()).collect();
            }
        }
        log_ok(
            "PlatformInfo::parse_os_release",
            started_at,
            &format!("id={} like={}", id, preview_debug(&like)),
        );
        Ok((id, like))
    }

    fn detect_package_manager(&self) -> Option<String> {
        let started_at = log_enter(
            "PlatformInfo::detect_package_manager",
            &format!("os_id={} os_like={}", self.os_id, preview_debug(&self.os_like)),
        );
        // Collect all related OS identifiers
        let mut identifiers = HashSet::new();
        if !self.os_id.is_empty() {
            identifiers.insert(self.os_id.clone());
        }
        for l in &self.os_like {
            identifiers.insert(l.clone());
        }

        if identifiers.contains("ubuntu") || identifiers.contains("debian") || identifiers.contains("linuxmint") || identifiers.contains("pop") {
            let result = Some("apt".to_string());
            log_ok("PlatformInfo::detect_package_manager", started_at, "result=apt");
            return result;
        }
        
        if identifiers.contains("fedora") || identifiers.contains("rhel") || identifiers.contains("centos") || identifiers.contains("rocky") || identifiers.contains("almalinux") {
            let result = Some("dnf".to_string());
            log_ok("PlatformInfo::detect_package_manager", started_at, "result=dnf");
            return result;
        }

        if identifiers.contains("arch") || identifiers.contains("manjaro") || identifiers.contains("endeavouros") || identifiers.contains("garuda") {
            let result = Some("pacman".to_string());
            log_ok("PlatformInfo::detect_package_manager", started_at, "result=pacman");
            return result;
        }

        if identifiers.contains("opensuse") || identifiers.contains("suse") {
            let result = Some("zypper".to_string());
            log_ok("PlatformInfo::detect_package_manager", started_at, "result=zypper");
            return result;
        }

        log_ok("PlatformInfo::detect_package_manager", started_at, "result=None");
        None
    }

    pub fn matches_os(&self, os_list: &[String]) -> bool {
        let started_at = log_enter(
            "PlatformInfo::matches_os",
            &format!("requested={} current={} like={}", preview_debug(&os_list), self.os_id, preview_debug(&self.os_like)),
        );
        for os in os_list {
            if self.os_id == *os || self.os_like.contains(os) {
                log_ok("PlatformInfo::matches_os", started_at, "result=true");
                return true;
            }
        }
        log_ok("PlatformInfo::matches_os", started_at, "result=false");
        false
    }
}
