use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::warn;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct OsdConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_osd_backend")]
    pub backend: OsdBackend,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OsdBackend {
    Libnotify,
}

impl Default for OsdConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            backend: OsdBackend::Libnotify,
        }
    }
}

pub fn show(config: &OsdConfig, summary: &str, body: &str) {
    if !config.enabled {
        return;
    }

    if let Err(error) = show_inner(config.backend, summary, body) {
        warn!(error = %error, "failed to show OSD notification");
    }
}

fn show_inner(backend: OsdBackend, summary: &str, body: &str) -> Result<()> {
    match backend {
        OsdBackend::Libnotify => show_libnotify(summary, body),
    }
}

fn show_libnotify(summary: &str, body: &str) -> Result<()> {
    let status = Command::new("notify-send")
        .args([
            "--app-name=wheelctl",
            "--hint=string:x-canonical-private-synchronous:wheelctl",
            summary,
            body,
        ])
        .status()
        .context("notify-send is required for libnotify OSD but was not found on PATH")?;

    if !status.success() {
        anyhow::bail!("notify-send failed with status {status}");
    }

    Ok(())
}

fn default_osd_backend() -> OsdBackend {
    OsdBackend::Libnotify
}
