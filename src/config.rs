use crate::backend::BackendKind;
use crate::osd::OsdConfig;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    pub devices: Vec<DeviceConfig>,
    #[serde(default)]
    pub osd: OsdConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DeviceConfig {
    pub key: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub phys: Option<String>,
    #[serde(default)]
    pub vendor_id: Option<String>,
    #[serde(default)]
    pub product_id: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub grab: bool,
    #[serde(default)]
    pub mappings: Mappings,
    #[serde(default)]
    pub disabled: DisabledEvents,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Mappings {
    #[serde(default = "ScrollVerticalMapping::enabled_pipewire_volume")]
    pub scroll_vertical: ScrollVerticalMapping,
    #[serde(default)]
    pub mode_button: ModeButtonMapping,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ScrollVerticalMapping {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_pipewire")]
    pub backend: BackendKind,
    #[serde(default = "default_target")]
    pub target: String,
    #[serde(default = "default_step")]
    pub step: String,
    #[serde(default = "default_fine_step")]
    pub fine_step: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ModeButtonMapping {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_mode_button")]
    pub button: ButtonCode,
    #[serde(default)]
    pub behavior: ModeButtonBehavior,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ButtonCode {
    BtnRight,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModeButtonBehavior {
    Toggle,
    Hold,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DisabledEvents {
    #[serde(default = "default_true")]
    pub movement: bool,
    #[serde(default = "default_true")]
    pub buttons: bool,
    #[serde(default = "default_true")]
    pub horizontal_scroll: bool,
}

impl Default for Mappings {
    fn default() -> Self {
        Self {
            scroll_vertical: ScrollVerticalMapping::enabled_pipewire_volume(),
            mode_button: ModeButtonMapping::default(),
        }
    }
}

impl ScrollVerticalMapping {
    pub fn enabled_pipewire_volume() -> Self {
        Self {
            enabled: true,
            backend: BackendKind::Pipewire,
            target: default_target(),
            step: default_step(),
            fine_step: default_fine_step(),
        }
    }
}

impl Default for ModeButtonMapping {
    fn default() -> Self {
        Self {
            enabled: true,
            button: ButtonCode::BtnRight,
            behavior: ModeButtonBehavior::Toggle,
        }
    }
}

impl Default for ModeButtonBehavior {
    fn default() -> Self {
        Self::Toggle
    }
}

impl Default for DisabledEvents {
    fn default() -> Self {
        Self {
            movement: true,
            buttons: true,
            horizontal_scroll: true,
        }
    }
}

impl Config {
    pub fn load_or_default() -> Result<Self> {
        let path = config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let data = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config at {}", path.display()))?;
        toml::from_str(&data)
            .with_context(|| format!("failed to parse config at {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory {}", parent.display())
            })?;
        }

        let data = toml::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(&path, data)
            .with_context(|| format!("failed to write config at {}", path.display()))
    }

    pub fn add_device(&mut self, device: DeviceConfig) -> Result<()> {
        if self
            .devices
            .iter()
            .any(|existing| existing.key == device.key)
        {
            bail!("device key '{}' already exists in config", device.key);
        }
        self.devices.push(device);
        Ok(())
    }

    pub fn remove_device(&mut self, key: &str) -> bool {
        let before = self.devices.len();
        self.devices.retain(|device| device.key != key);
        self.devices.len() != before
    }
}

pub fn config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir().context("could not determine user config directory")?;
    Ok(config_dir.join("wheelctl").join("config.toml"))
}

fn default_true() -> bool {
    true
}

fn default_pipewire() -> BackendKind {
    BackendKind::Pipewire
}

fn default_target() -> String {
    "volume".to_string()
}

fn default_step() -> String {
    "5%".to_string()
}

fn default_fine_step() -> String {
    "1.5%".to_string()
}

fn default_mode_button() -> ButtonCode {
    ButtonCode::BtnRight
}

#[cfg(test)]
mod tests {
    use super::{Config, DeviceConfig, DisabledEvents, Mappings, ModeButtonBehavior};

    #[test]
    fn missing_config_defaults_to_no_devices() {
        let config = Config::default();
        assert!(config.devices.is_empty());
    }

    #[test]
    fn device_defaults_match_v1_behavior() {
        let device = DeviceConfig {
            key: "test".to_string(),
            name: "Test".to_string(),
            path: "/dev/input/event0".to_string(),
            phys: None,
            vendor_id: None,
            product_id: None,
            enabled: true,
            grab: true,
            mappings: Mappings::default(),
            disabled: DisabledEvents::default(),
        };

        assert!(device.enabled);
        assert!(device.grab);
        assert_eq!(device.mappings.scroll_vertical.step, "5%");
        assert_eq!(device.mappings.scroll_vertical.fine_step, "1.5%");
        assert!(device.mappings.mode_button.enabled);
    }

    #[test]
    fn parses_documented_device_stanza() {
        let config: Config = toml::from_str(
            r#"
[[devices]]
key = "dell-usb-optical-mouse"
name = "Dell USB Optical Mouse"
path = "/dev/input/by-id/usb-Dell_USB_Optical_Mouse-event-mouse"
phys = "usb-0000:00:14.0-1/input0"
vendor_id = "413c"
product_id = "301a"
enabled = true
grab = true

[devices.mappings.scroll_vertical]
enabled = true
backend = "pipewire"
target = "volume"
step = "5%"
fine_step = "1.5%"

[devices.mappings.mode_button]
enabled = true
button = "BTN_RIGHT"
behavior = "toggle"

[devices.disabled]
movement = true
buttons = true
horizontal_scroll = true
"#,
        )
        .unwrap();

        let device = &config.devices[0];
        assert_eq!(device.key, "dell-usb-optical-mouse");
        assert_eq!(device.vendor_id.as_deref(), Some("413c"));
        assert!(device.disabled.buttons);
        assert_eq!(
            device.mappings.mode_button.behavior,
            ModeButtonBehavior::Toggle
        );
        assert!(!config.osd.enabled);
    }
}
