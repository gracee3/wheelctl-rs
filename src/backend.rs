use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    Pipewire,
}

pub fn change_volume(backend: BackendKind, step: &str, increase: bool) -> Result<()> {
    match backend {
        BackendKind::Pipewire => change_pipewire_volume(step, increase),
    }
}

pub fn ensure_available(backend: BackendKind) -> Result<()> {
    match backend {
        BackendKind::Pipewire => {
            Command::new("wpctl")
                .arg("--version")
                .output()
                .context("wpctl is required for the PipeWire backend but was not found on PATH")?;
            Ok(())
        }
    }
}

pub fn current_volume(backend: BackendKind) -> Result<String> {
    match backend {
        BackendKind::Pipewire => current_pipewire_volume(),
    }
}

fn change_pipewire_volume(step: &str, increase: bool) -> Result<()> {
    let direction = if increase { "+" } else { "-" };
    let amount = format!("{step}{direction}");
    let status = Command::new("wpctl")
        .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &amount])
        .status()
        .context("failed to execute wpctl; PipeWire wireplumber tools may be missing")?;

    if !status.success() {
        bail!("wpctl set-volume failed with status {status}");
    }

    Ok(())
}

fn current_pipewire_volume() -> Result<String> {
    let output = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .context("failed to execute wpctl; PipeWire wireplumber tools may be missing")?;

    if !output.status.success() {
        bail!("wpctl get-volume failed with status {}", output.status);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::BackendKind;
    use serde::Serialize;

    #[test]
    fn backend_names_are_lowercase_toml() {
        #[derive(Serialize)]
        struct Wrapper {
            backend: BackendKind,
        }

        let encoded = toml::to_string(&Wrapper {
            backend: BackendKind::Pipewire,
        })
        .unwrap();
        assert_eq!(encoded.trim(), "backend = \"pipewire\"");
    }
}
