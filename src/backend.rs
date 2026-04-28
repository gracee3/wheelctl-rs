use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    Pipewire,
    Alsa,
}

pub fn change_volume(backend: BackendKind, step: &str, increase: bool) -> Result<()> {
    match backend {
        BackendKind::Pipewire => change_pipewire_volume(step, increase),
        BackendKind::Alsa => bail!("ALSA backend is not implemented in v1"),
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
