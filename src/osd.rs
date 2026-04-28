use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::process::Command;
use std::process::Stdio;
use std::thread;
use std::time::{Duration, Instant};
use tracing::warn;

const NOTIFY_TIMEOUT: Duration = Duration::from_millis(1500);
const EXPIRE_MS: &str = "700";

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
    last_message: Option<(String, String)>,
    replace_id: Option<String>,
}

impl Notifier {
    pub fn new(config: OsdConfig) -> Self {
        Self {
            config,
            disabled_after_error: false,
            last_message: None,
            replace_id: None,
        }
    }

    pub fn show(&mut self, summary: &str, body: &str) {
        if !self.config.enabled || self.disabled_after_error {
            return;
        }

        let message = (summary.to_string(), body.to_string());
        if self.last_message.as_ref() == Some(&message) {
            return;
        }

        match show_inner(
            self.config.backend,
            summary,
            body,
            self.replace_id.as_deref(),
        ) {
            Ok(replace_id) => {
                self.replace_id = replace_id;
                self.last_message = Some(message);
            }
            Err(error) => {
                self.disabled_after_error = true;
                warn!(error = %error, "failed to show OSD notification; disabling OSD for this run");
            }
        }
    }
}

pub fn show_checked(config: &OsdConfig, summary: &str, body: &str) -> Result<()> {
    show_inner(config.backend, summary, body, None).map(|_| ())
}

fn show_inner(
    backend: OsdBackend,
    summary: &str,
    body: &str,
    replace_id: Option<&str>,
) -> Result<Option<String>> {
    match backend {
        OsdBackend::Libnotify => show_libnotify(summary, body, replace_id),
    }
}

fn show_libnotify(summary: &str, body: &str, replace_id: Option<&str>) -> Result<Option<String>> {
    let mut command = Command::new("notify-send");
    command
        .args(["--app-name=wheelctl", "--urgency=low", "--transient"])
        .args(["--expire-time", EXPIRE_MS])
        .args(["--category", "device"])
        .args(["--hint", "string:x-canonical-private-synchronous:wheelctl"])
        .arg("--print-id");

    if let Some(replace_id) = replace_id {
        command.args(["--replace-id", replace_id]);
    }

    let mut child = command
        .args([summary, body])
        .stdout(Stdio::piped())
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
            let mut output = String::new();
            if let Some(mut stdout) = child.stdout.take() {
                stdout
                    .read_to_string(&mut output)
                    .context("failed to read notify-send output")?;
            }
            let id = output.trim().to_string();
            return Ok((!id.is_empty()).then_some(id));
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
