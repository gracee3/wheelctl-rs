use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use tracing::warn;

const NOTIFY_TIMEOUT: Duration = Duration::from_millis(1500);

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
            enabled: true,
            backend: OsdBackend::Libnotify,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Notifier {
    config: OsdConfig,
    disabled_after_error: bool,
}

impl Notifier {
    pub fn new(config: OsdConfig) -> Self {
        Self {
            config,
            disabled_after_error: false,
        }
    }

    pub fn show(&mut self, summary: &str, body: &str) {
        if !self.config.enabled || self.disabled_after_error {
            return;
        }

        if let Err(error) = show_inner(self.config.backend, summary, body) {
            self.disabled_after_error = true;
            warn!(error = %error, "failed to show OSD notification; disabling OSD for this run");
        }
    }
}

pub fn show_checked(config: &OsdConfig, summary: &str, body: &str) -> Result<()> {
    show_inner(config.backend, summary, body)
}

fn show_inner(backend: OsdBackend, summary: &str, body: &str) -> Result<()> {
    match backend {
        OsdBackend::Libnotify => show_libnotify(summary, body),
    }
}

fn show_libnotify(summary: &str, body: &str) -> Result<()> {
    let mut child = Command::new("notify-send")
        .args([
            "--app-name=wheelctl",
            "--expire-time=900",
            "--hint=string:x-canonical-private-synchronous:wheelctl",
            summary,
            body,
        ])
        .spawn()
        .context("notify-send is required for libnotify OSD but was not found on PATH")?;

    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .context("failed to poll notify-send status")?
        {
            if !status.success() {
                anyhow::bail!("notify-send failed with status {status}");
            }
            return Ok(());
        }

        if started.elapsed() >= NOTIFY_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("notify-send did not finish within {:?}", NOTIFY_TIMEOUT);
        }

        thread::sleep(Duration::from_millis(20));
    }
}

fn default_osd_backend() -> OsdBackend {
    OsdBackend::Libnotify
}
